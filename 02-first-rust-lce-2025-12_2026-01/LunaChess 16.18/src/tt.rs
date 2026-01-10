use crate::board::Mossa;

#[derive(Clone, Copy, PartialEq)]
pub enum Bound { Exact = 1, Lower = 2, Upper = 3 }

#[derive(Clone, Copy)]
pub struct TTEntry {
    pub key: u64,
    pub move_data: u16,
    pub score: i16,
    pub depth: i8,
    pub bound: u8,
}

pub struct TranspositionTable {
    pub table: Vec<Option<TTEntry>>,
    pub size: usize,
}

impl TranspositionTable {
    pub fn new(mb: usize) -> Self {
        let size = (mb * 1024 * 1024) / std::mem::size_of::<Option<TTEntry>>();
        Self { table: vec![None; size], size }
    }

    pub fn clear(&mut self) {
        for slot in self.table.iter_mut() { *slot = None; }
    }

    pub fn new_search(&mut self) {}

    pub fn probe(&self, key: u64) -> Option<TTEntry> {
        let index = (key as usize) % self.size;
        match self.table[index] {
            Some(entry) if entry.key == key => Some(entry),
            _ => None,
        }
    }

    pub fn store(&mut self, key: u64, mut score: i32, mv: Option<Mossa>, depth: i32, bound: Bound, ply: usize) {
        let index = (key as usize) % self.size;
        if score > 40000 { score += ply as i32; }
        else if score < -40000 { score -= ply as i32; }

        self.table[index] = Some(TTEntry {
            key,
            move_data: mv.map(|m| m.data).unwrap_or(0),
            score: score as i16,
            depth: depth as i8,
            bound: bound as u8,
        });
    }
}