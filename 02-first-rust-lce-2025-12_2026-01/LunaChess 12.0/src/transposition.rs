use crate::board::Mossa;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Bound { 
    Exact, 
    Lower, 
    Upper 
}

#[derive(Clone, Copy, Debug)]
pub struct TTEntry {
    pub key: u64,
    pub score: i16,
    pub mossa: u16,
    pub depth: u8,
    pub bound: Bound,
    pub age: u8, // Aggiunto per gestire la sostituzione intelligente
}

pub struct TranspositionTable {
    pub table: Vec<Option<TTEntry>>,
    pub size: usize,
    pub current_age: u8, // Incrementato a ogni nuova ricerca (UCI 'ucinewgame')
}

impl TranspositionTable {
    pub fn new(size_mb: usize) -> Self {
        let entry_size = std::mem::size_of::<Option<TTEntry>>();
        let num_entries = (size_mb * 1024 * 1024) / entry_size;
        
        // Inizializzazione rapida
        let table = vec![None; num_entries];
        
        TranspositionTable { 
            table, 
            size: num_entries,
            current_age: 0 
        }
    }

    pub fn store(&mut self, key: u64, score: i16, mossa: u16, depth: u8, bound: Bound, ply: usize) {
        if self.size == 0 { return; }
        let index = (key as usize) % self.size;
        
        let mut tt_score = score;
        if score > 28000 { tt_score += ply as i16; }
        else if score < -28000 { tt_score -= ply as i16; }

        let mut replace = true;
        if let Some(entry) = self.table[index] {
            // Strategia di sostituzione: 
            // Sostituisci se la nuova ricerca è più profonda O se l'entry vecchia appartiene a una ricerca precedente (age)
            if entry.key == key && entry.depth > depth && entry.age == self.current_age {
                replace = false;
            }
        }

        if replace {
            self.table[index] = Some(TTEntry { 
                key, 
                score: tt_score, 
                mossa, 
                depth, 
                bound,
                age: self.current_age 
            });
        }
    }

    pub fn probe(&self, key: u64) -> Option<TTEntry> {
        if self.size == 0 { return None; }
        let index = (key as usize) % self.size;
        
        if let Some(entry) = self.table[index] {
            if entry.key == key { 
                return Some(entry); 
            }
        }
        None
    }

    pub fn score_from_tt(&self, score: i16, ply: usize) -> i16 {
        if score > 28000 { return score - ply as i16; }
        if score < -28000 { return score + ply as i16; }
        score
    }
    
    // Da chiamare a ogni nuova mossa per "invecchiare" i dati vecchi
    pub fn next_generation(&mut self) {
        self.current_age = self.current_age.wrapping_add(1);
    }

    pub fn clear(&mut self) {
        for entry in self.table.iter_mut() { 
            *entry = None; 
        }
        self.current_age = 0;
    }
}