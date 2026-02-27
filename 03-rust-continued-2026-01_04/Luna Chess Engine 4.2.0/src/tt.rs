use crate::board::Mossa;

#[derive(Clone, Copy, Debug)]
pub enum Bound {
    Alpha = 0,
    Beta = 1,
    Exact = 2,
}

#[derive(Clone, Copy)]
pub struct TTEntry {
    key: u64,
    score: i32,
    mv: u16,
    depth: u8,
    bound: u8,
    gen: u8,
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
        
        let mut n = target_bytes / entry_size;
        let mut real = 1usize;
        while real * 2 <= n {
            real *= 2;
        }

        TranspositionTable {
            entries: vec![
                TTEntry {
                    key: 0,
                    score: 0,
                    mv: 0,
                    depth: 0,
                    bound: 0,
                    gen: 0,
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
        self.generation = self.generation.wrapping_add(1);
    }

    fn store_score(score: i32, ply: i32) -> i32 {
        let mate = 49000;
        if score >= mate - 1000 {
            return score + ply;
        }
        if score <= -mate + 1000 {
            return score - ply;
        }
        score
    }

    fn load_score(score: i32, ply: i32) -> i32 {
        let mate = 49000;
        if score >= mate - 1000 {
            return score - ply;
        }
        if score <= -mate + 1000 {
            return score + ply;
        }
        score
    }

    pub fn probe(&self, key: u64, depth: i32, alpha: i32, beta: i32, ply: i32) -> Option<i32> {
        let idx = (key as usize) & self.mask;
        let e = &self.entries[idx];

        if e.key != key {
            return None;
        }

        if (e.depth as i32) >= depth {
            let score = Self::load_score(e.score, ply);

            if e.bound == Bound::Exact as u8 {
                return Some(score);
            }
            if e.bound == Bound::Alpha as u8 && score <= alpha {
                return Some(score);
            }
            if e.bound == Bound::Beta as u8 && score >= beta {
                return Some(score);
            }
        }

        None
    }

    pub fn get_move(&self, key: u64) -> Mossa {
        let idx = (key as usize) & self.mask;
        let e = &self.entries[idx];

        if e.key == key {
            Mossa::from_data(e.mv)
        } else {
            Mossa::null()
        }
    }

    pub fn store(&mut self, key: u64, depth: i32, score: i32, bound: Bound, mv: Mossa, ply: i32) {
        let idx = (key as usize) & self.mask;
        let e = &mut self.entries[idx];

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

            if !mv.is_null() {
                e.mv = mv.data;
            }

            e.score = Self::store_score(score, ply);
        }
    }
}