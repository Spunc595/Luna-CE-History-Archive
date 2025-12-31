use crate::board::Mossa;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Bound { Exact, Lower, Upper }

#[derive(Clone, Copy, Debug)]
pub struct TTEntry {
    pub key: u64,
    pub score: i16,
    pub mossa: u16,
    pub depth: u8,
    pub bound: Bound,
}

pub struct TranspositionTable {
    pub table: Vec<Option<TTEntry>>,
    pub size: usize,
}

impl TranspositionTable {
    pub fn new(size_mb: usize) -> Self {
        let entry_size = std::mem::size_of::<Option<TTEntry>>();
        let num_entries = (size_mb * 1024 * 1024) / entry_size;
        let mut table = Vec::with_capacity(num_entries);
        for _ in 0..num_entries { table.push(None); }
        TranspositionTable { table, size: num_entries }
    }

    pub fn store(&mut self, key: u64, score: i16, mossa: u16, depth: u8, bound: Bound, ply: usize) {
        if self.size == 0 { return; }
        let index = (key as usize) % self.size;
        let mut tt_score = score;
        if score > 28000 { tt_score += ply as i16; }
        else if score < -28000 { tt_score -= ply as i16; }
        if let Some(entry) = self.table[index] {
            if entry.key == key && entry.depth > depth { return; }
        }
        self.table[index] = Some(TTEntry { key, score: tt_score, mossa, depth, bound });
    }

    pub fn probe(&self, key: u64) -> Option<TTEntry> {
        if self.size == 0 { return None; }
        let index = (key as usize) % self.size;
        if let Some(entry) = self.table[index] {
            if entry.key == key { return Some(entry); }
        }
        None
    }

    pub fn score_from_tt(&self, score: i16, ply: usize) -> i16 {
        if score > 28000 { return score - ply as i16; }
        if score < -28000 { return score + ply as i16; }
        score
    }
    
    pub fn clear(&mut self) { for entry in self.table.iter_mut() { *entry = None; } }
}