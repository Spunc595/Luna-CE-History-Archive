use crate::board::Mossa;

pub const MATE_VALUE: i32 = 32000;
pub const MATE_THRESHOLD: i32 = 31000;

#[derive(Clone, Copy, PartialEq, Debug)]
#[repr(u8)]
pub enum Bound { None = 0, Exact = 1, Lower = 2, Upper = 3 }

#[derive(Clone, Copy)]
pub struct TTEntry {
    pub key: u64,
    pub move_data: u16,
    pub score: i16,
    pub depth: i8,
    pub bound: Bound,
    pub age: u8,
}

pub struct TranspositionTable {
    pub table: Box<[TTEntry]>,
    pub mask: usize, // Fondamentale per la velocità
    pub age: u8,
}

impl TranspositionTable {
    pub fn new(mb: usize) -> Self {
        let entry_size = std::mem::size_of::<TTEntry>();
        let mut num_entries = (mb * 1024 * 1024) / entry_size;

        // Forza la dimensione a una potenza di 2
        if !num_entries.is_power_of_two() {
            num_entries = num_entries.next_power_of_two() >> 1;
        }

        let table = vec![TTEntry {
            key: 0, move_data: 0, score: 0, depth: 0,
            bound: Bound::None, age: 0,
        }; num_entries].into_boxed_slice();

        Self { table, mask: num_entries - 1, age: 0 }
    }

    pub fn clear(&mut self) {
        for entry in self.table.iter_mut() {
            entry.key = 0;
            entry.bound = Bound::None;
        }
        self.age = 0;
    }

    pub fn new_search(&mut self) {
        self.age = self.age.wrapping_add(1);
    }

    #[inline(always)]
    pub fn probe(&self, key: u64) -> Option<TTEntry> {
        let index = (key as usize) & self.mask;
        let entry = self.table[index];
        if entry.key == key { Some(entry) } else { None }
    }

    pub fn store(&mut self, key: u64, score: i32, mv: Mossa, depth: i32, bound: Bound, ply: usize) {
        let index = (key as usize) & self.mask;
        let old_entry = &self.table[index];

        let mut stored_score = score;
        if score > MATE_THRESHOLD { stored_score += ply as i32; }
        else if score < -MATE_THRESHOLD { stored_score -= ply as i32; }

        let final_score = stored_score.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
        let mut move_to_store = mv.data;
        if move_to_store == 0 && old_entry.key == key {
            move_to_store = old_entry.move_data;
        }

        // Rimpiazza se la posizione è nuova, se è di una ricerca vecchia o se la profondità è maggiore
        if old_entry.key == 0 || old_entry.age != self.age || depth as i8 >= old_entry.depth {
            self.table[index] = TTEntry {
                key, move_data: move_to_store, score: final_score,
                depth: depth as i8, bound, age: self.age,
            };
        }
    }

    #[inline(always)]
    pub fn get_score(&self, entry: &TTEntry, ply: usize) -> i32 {
        let score = entry.score as i32;
        if score > MATE_THRESHOLD { return score - ply as i32; }
        if score < -MATE_THRESHOLD { return score + ply as i32; }
        score
    }
}