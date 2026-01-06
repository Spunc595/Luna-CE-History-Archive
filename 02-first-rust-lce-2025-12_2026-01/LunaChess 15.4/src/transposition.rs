use crate::board::Mossa;

#[derive(Clone, Copy)]
pub struct TTEntry {
    pub hash: u64,
    pub depth: u8,
    pub score: i16,
    pub best_move: Option<Mossa>,
}

pub struct TranspositionTable {
    pub table: Vec<Option<TTEntry>>,
    pub size: usize,
}

impl TranspositionTable {
    pub fn new(size_mb: usize) -> Self {
        let num_entries = (size_mb * 1024 * 1024) / std::mem::size_of::<Option<TTEntry>>();
        Self {
            table: vec![None; num_entries],
            size: num_entries,
        }
    }

    pub fn get(&self, hash: u64) -> Option<TTEntry> {
        let index = (hash as usize) % self.size;
        match self.table[index] {
            Some(entry) if entry.hash == hash => Some(entry),
            _ => None,
        }
    }

    pub fn insert(&mut self, hash: u64, depth: u8, score: i16, best_move: Option<Mossa>) {
        let index = (hash as usize) % self.size;
        // Semplice politica di rimpiazzo: se la nuova profondità è maggiore o uguale
        self.table[index] = Some(TTEntry { hash, depth, score, best_move });
    }

    pub fn clear(&mut self) {
        for entry in self.table.iter_mut() {
            *entry = None;
        }
    }
}