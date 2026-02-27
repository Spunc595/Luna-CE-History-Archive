use crate::board::Mossa;

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(u8)]
pub enum Bound {
    Alpha = 0, // Valore è un limite superiore (Upper Bound)
    Beta = 1,  // Valore è un limite inferiore (Lower Bound)
    Exact = 2, // Valore esatto
}

#[derive(Clone, Copy)]
#[repr(C)] // Garantisce un layout di memoria prevedibile e compatto
pub struct TTEntry {
    pub key: u64,      // 8 bytes
    pub score: i32,    // 4 bytes
    pub move_data: u16,// 2 bytes (Mossa.data)
    pub move_pro: u8,  // 1 byte  (Mossa.promozione)
    pub depth: u8,     // 1 byte
    pub bound: u8,     // 1 byte
    pub gen: u8,       // 1 byte
    pub padding: u16,  // 2 bytes per allineare a 20/24 bytes
}

pub struct TranspositionTable {
    entries: Vec<TTEntry>,
    mask: usize,
    generation: u8,
}

impl TranspositionTable {
    pub fn new(size_mb: usize) -> Self {
        let entry_size = std::mem::size_of::<TTEntry>();
        let target_bytes = size_mb * 1024 * 1024;
        
        // Calcoliamo la potenza di 2 più vicina per usare il mask invece del modulo (%)
        let n = target_bytes / entry_size;
        let mut real = 1usize;
        while real * 2 <= n {
            real *= 2;
        }

        TranspositionTable {
            entries: vec![
                TTEntry {
                    key: 0,
                    score: 0,
                    move_data: 0,
                    move_pro: 6, // Null promo
                    depth: 0,
                    bound: 0,
                    gen: 0,
                    padding: 0,
                };
                real
            ],
            mask: real - 1,
            generation: 1,
        }
    }

    pub fn clear(&mut self) {
        for e in &mut self.entries {
            e.key = 0;
            e.gen = 0;
        }
        self.generation = 1;
    }

    pub fn new_search(&mut self) {
        // Incrementiamo la generazione per distinguere le voci di questa ricerca da quelle precedenti
        self.generation = self.generation.wrapping_add(1);
    }

    // Normalizza il punteggio del matto in base alla distanza dalla radice (ply)
    fn store_score(score: i32, ply: i32) -> i32 {
        let mate = 49000;
        if score >= mate - 1000 { return score + ply; }
        if score <= -mate + 1000 { return score - ply; }
        score
    }

    fn load_score(score: i32, ply: i32) -> i32 {
        let mate = 49000;
        if score >= mate - 1000 { return score - ply; }
        if score <= -mate + 1000 { return score + ply; }
        score
    }

    pub fn probe(&self, key: u64, depth: i32, alpha: i32, beta: i32, ply: i32) -> Option<i32> {
        let idx = (key as usize) & self.mask;
        let e = &self.entries[idx];

        if e.key != key {
            return None;
        }

        // Possiamo usare il punteggio della TT solo se la profondità salvata è sufficiente
        if (e.depth as i32) >= depth {
            let score = Self::load_score(e.score, ply);

            match e.bound {
                b if b == Bound::Exact as u8 => return Some(score),
                b if b == Bound::Alpha as u8 && score <= alpha => return Some(alpha),
                b if b == Bound::Beta as u8 && score >= beta => return Some(beta),
                _ => {}
            }
        }

        None
    }

    pub fn get_move(&self, key: u64) -> Mossa {
        let idx = (key as usize) & self.mask;
        let e = &self.entries[idx];

        if e.key == key {
            Mossa { data: e.move_data, promozione: e.move_pro }
        } else {
            Mossa::null()
        }
    }

    pub fn store(&mut self, key: u64, depth: i32, score: i32, bound: Bound, mv: Mossa, ply: i32) {
        let idx = (key as usize) & self.mask;
        let e = &mut self.entries[idx];

        // Se la mossa passata è nulla, cerchiamo di mantenere quella già presente per la stessa posizione
        let (final_mv, final_pro) = if e.key == key && mv.is_null() {
            (e.move_data, e.move_pro)
        } else {
            (mv.data, mv.promozione)
        };

        // Strategia di rimpiazzo:
        // 1. Casa vuota
        // 2. Stessa posizione (aggiornamento)
        // 3. Voce di una generazione precedente (stale)
        // 4. Nuova ricerca più profonda
        let replace = 
            e.key == 0 ||
            e.key == key ||
            e.gen != self.generation ||
            (depth as u8) >= e.depth;

        if replace {
            e.key = key;
            e.depth = depth as u8;
            e.bound = bound as u8;
            e.gen = self.generation;
            e.move_data = final_mv;
            e.move_pro = final_pro;
            e.score = Self::store_score(score, ply);
        }
    }
}