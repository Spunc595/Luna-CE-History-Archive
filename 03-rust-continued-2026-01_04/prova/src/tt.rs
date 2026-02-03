use crate::board::Mossa;

const MATE_BOUND: i32 = 48000;

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
    pub depth: u8,
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
        let size = (mb_size * 1024 * 1024) / std::mem::size_of::<TTEntry>();
        let mut real_size = 1;
        while real_size <= size { real_size *= 2; }
        
        TranspositionTable {
            entries: vec![TTEntry { key: 0, score: 0, move_data: 0, depth: 0, bound: 0, generation: 0 }; real_size],
            mask: real_size - 1,
            generation: 1,
        }
    }

    pub fn clear(&mut self) {
        for entry in &mut self.entries {
            entry.key = 0;
            entry.generation = 0;
        }
        self.generation = 1;
    }

    pub fn new_search(&mut self) {
        self.generation = self.generation.wrapping_add(1);
    }

    // Aggiunto "ply" per gestire correttamente la distanza del Matto
    pub fn probe(&self, key: u64, depth: i32, alpha: i32, beta: i32, ply: i32) -> Option<i32> {
        let idx = (key as usize) & self.mask;
        let entry = &self.entries[idx];

        if entry.key == key {
            if entry.depth as i32 >= depth {
                let mut score = entry.score; 
                
                // Correggiamo il punteggio del matto in base alla profondità (ply)
                if score > MATE_BOUND {
                    score -= ply;
                } else if score < -MATE_BOUND {
                    score += ply;
                }

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

    // Metodo helper per recuperare la mossa (usato per l'ordinamento mosse)
    pub fn get_move(&self, key: u64) -> Mossa {
        let idx = (key as usize) & self.mask;
        let entry = &self.entries[idx];
        if entry.key == key {
            Mossa::from_data(entry.move_data)
        } else {
            Mossa::null()
        }
    }

    // Aggiunto "ply" per salvare la distanza corretta del Matto
    pub fn store(&mut self, key: u64, depth: i32, score: i32, bound: Bound, best_move: Mossa, ply: i32) {
        let idx = (key as usize) & self.mask;
        let entry = &mut self.entries[idx];

        // Sostituzione: se la generazione è vecchia, o se cerchiamo più a fondo, o se è Exact
        if entry.key != key || depth as u8 >= entry.depth || entry.generation != self.generation || bound == Bound::Exact {
            
            // Regoliamo il punteggio prima di salvarlo se è un matto
            let mut adjusted_score = score;
            if adjusted_score > MATE_BOUND {
                adjusted_score += ply;
            } else if adjusted_score < -MATE_BOUND {
                adjusted_score -= ply;
            }

            entry.key = key;
            entry.score = adjusted_score;
            entry.depth = depth as u8;
            entry.bound = bound as u8;
            entry.generation = self.generation;
            
            if !best_move.is_null() {
                entry.move_data = best_move.data;
            }
        }
    }
}