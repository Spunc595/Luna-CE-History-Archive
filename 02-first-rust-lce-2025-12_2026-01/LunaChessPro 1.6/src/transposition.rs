use crate::board::Mossa;

#[derive(Copy, Clone, PartialEq)]
pub enum NodeType {
    Exact,
    LowerBound,
    UpperBound,
}

#[derive(Copy, Clone)]
pub struct TTEntry {
    pub hash: u64,
    pub depth: i32,
    pub score: i32,
    pub node_type: NodeType,
    pub mossa: Mossa,
}

pub struct TranspositionTable {
    pub table: Vec<Option<TTEntry>>,
    pub size: usize,
}

impl TranspositionTable {
    pub fn new(mb: usize) -> Self {
        let count = (mb * 1024 * 1024) / std::mem::size_of::<Option<TTEntry>>();
        Self {
            table: vec![None; count],
            size: count,
        }
    }

    pub fn clear(&mut self) {
        for entry in self.table.iter_mut() { *entry = None; }
    }

    pub fn get(&self, hash: u64) -> Option<TTEntry> {
        let index = (hash as usize) % self.size;
        match self.table[index] {
            Some(entry) if entry.hash == hash => Some(entry),
            _ => None,
        }
    }

    pub fn save(&mut self, hash: u64, depth: i32, score: i32, node_type: NodeType, mossa: Mossa) {
        let index = (hash as usize) % self.size;
        self.table[index] = Some(TTEntry {
            hash,
            depth,
            score,
            node_type,
            mossa,
        });
    }
}