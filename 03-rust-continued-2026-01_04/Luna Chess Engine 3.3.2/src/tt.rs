use crate::board::Mossa;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Bound {
    None = 0,
    Exact = 1,
    Alpha = 2, // Upper Bound
    Beta = 3,  // Lower Bound
}

#[derive(Clone, Copy, Debug)]
pub struct TTEntry {
    pub key: u64,
    pub score: i32,
    pub move_data: u16, // Memorizziamo solo i dati raw della mossa
    pub depth: i32,     // Cambiato a i32 per precisione nei confronti
    pub bound: u8,      // Convertiamo Bound in u8 per compattezza
    pub generation: u8,
}

pub struct TranspositionTable {
    entries: Vec<TTEntry>,
    mask: usize,
    generation: u8,
}

impl TranspositionTable {
    pub fn new(mb_size: usize) -> Self {
        // Ogni TTEntry occupa circa 24 byte.
        let size_bytes = mb_size * 1024 * 1024;
        let num_entries_target = size_bytes / std::mem::size_of::<TTEntry>();
        
        // La dimensione deve essere una potenza di 2 per usare la maschera (mask) velocemente
        let mut real_size = 1;
        while real_size <= num_entries_target { real_size *= 2; }
        
        // Se la dimensione calcolata è troppo grande per la RAM, real_size / 2
        let final_size = real_size / 2; 

        TranspositionTable {
            entries: vec![TTEntry { key: 0, score: 0, move_data: 0, depth: 0, bound: 0, generation: 0 }; final_size],
            mask: final_size - 1,
            generation: 1,
        }
    }

    pub fn clear(&mut self) {
        for entry in &mut self.entries {
            entry.key = 0;
            entry.generation = 0;
            entry.depth = 0;
        }
        self.generation = 1;
    }

    pub fn new_search(&mut self) {
        self.generation = self.generation.wrapping_add(1);
    }

    // Aggiornato per gestire il punteggio di scaccomatto relativo al ply
    pub fn probe(&self, key: u64, depth: i32, alpha: i32, beta: i32) -> Option<i32> {
        let idx = (key as usize) & self.mask;
        let entry = &self.entries[idx];

        if entry.key == key {
            if entry.depth >= depth {
                let mut score = entry.score;
                
                // Normalizzazione Mate Score:
                // Se il punteggio è un matto, dobbiamo adattarlo alla profondità attuale (ply)
                if score > 48000 { score -= 0; } // Semplificato, il ply viene gestito in search
                else if score < -48000 { score += 0; }

                let bound = entry.bound;

                if bound == Bound::Exact as u8 {
                    return Some(score);
                }
                if bound == Bound::Alpha as u8 && score <= alpha {
                    return Some(score);
                }
                if bound == Bound::Beta as u8 && score >= beta {
                    return Some(score);
                }
            }
        }
        None
    }

    pub fn get_move(&self, key: u64) -> Mossa {
        let idx = (key as usize) & self.mask;
        let entry = &self.entries[idx];
        if entry.key == key {
            Mossa::from_data(entry.move_data)
        } else {
            Mossa::null()
        }
    }

    pub fn store(&mut self, key: u64, depth: i32, score: i32, bound: Bound, best_move: Mossa) {
        let idx = (key as usize) & self.mask;
        let entry = &mut self.entries[idx];

        // Politica di sostituzione "Always Replace" se la generazione è nuova
        // o "Depth Preferred" se siamo nella stessa generazione.
        let replace = entry.key != key 
            || self.generation != entry.generation 
            || depth >= entry.depth;

        if replace {
            entry.key = key;
            entry.score = score;
            entry.depth = depth;
            entry.bound = bound as u8;
            entry.generation = self.generation;
            
            // Non sovrascrivere una mossa TT valida con una mossa nulla
            if !best_move.is_null() {
                entry.move_data = best_move.data;
            }
        }
    }
}