use crate::board::Mossa;

pub const MATE_VALUE: i32 = 49000;

#[derive(Clone, Copy, PartialEq, Debug)]
#[repr(u8)]
pub enum Bound {
    None = 0,
    Exact = 1,
    Lower = 2,
    Upper = 3,
}

#[derive(Clone, Copy)]
pub struct TTEntry {
    pub key: u64,
    pub move_data: u16,
    pub score: i16,
    pub depth: i8,
    pub bound: u8,
    pub age: u8,
}

pub struct TranspositionTable {
    pub table: Box<[Option<TTEntry>]>,
    pub size: usize,
    pub age: u8,
}

impl TranspositionTable {
    pub fn new(mb: usize) -> Self {
        let entry_size = std::mem::size_of::<Option<TTEntry>>();
        let num_entries = (mb * 1024 * 1024) / entry_size;
        let table = vec![None; num_entries].into_boxed_slice();
        
        Self {
            table,
            size: num_entries,
            age: 0,
        }
    }

    pub fn clear(&mut self) {
        for slot in self.table.iter_mut() {
            *slot = None;
        }
        self.age = 0;
    }

    pub fn new_search(&mut self) {
        self.age = self.age.wrapping_add(1);
    }

    #[inline(always)]
    pub fn probe(&self, key: u64) -> Option<TTEntry> {
        let index = (key as usize) % self.size;
        let entry = self.table[index];
        
        match entry {
            // Verifica rigorosa della chiave per evitare collisioni distruttive
            Some(e) if e.key == key => Some(e),
            _ => None,
        }
    }

    pub fn store(&mut self, key: u64, score: i32, mv: Mossa, depth: i32, bound: Bound, ply: usize) {
        let index = (key as usize) % self.size;
        
        // Se la mossa è nulla (0), non sovrascrivere una mossa utile già esistente
        let mut move_to_store = mv.data;
        if move_to_store == 0 {
            if let Some(old_e) = self.table[index] {
                if old_e.key == key {
                    move_to_store = old_e.move_data;
                }
            }
        }

        let mut stored_score = score;
        if score > 40000 { stored_score += ply as i32; }
        else if score < -40000 { stored_score -= ply as i32; }

        let replace = match self.table[index] {
            None => true,
            Some(old_entry) => {
                // Sostituisci se la nuova analisi è più profonda o se l'entry è di una partita precedente
                depth as i8 >= old_entry.depth || old_entry.age != self.age
            }
        };

        if replace {
            self.table[index] = Some(TTEntry {
                key,
                move_data: move_to_store,
                score: stored_score as i16,
                depth: depth as i8,
                bound: bound as u8,
                age: self.age,
            });
        }
    }

    #[inline(always)]
    pub fn get_score(&self, entry: &TTEntry, ply: usize) -> i32 {
        let score = entry.score as i32;
        if score > 40000 { return score - ply as i32; }
        if score < -40000 { return score + ply as i32; }
        score
    }
}