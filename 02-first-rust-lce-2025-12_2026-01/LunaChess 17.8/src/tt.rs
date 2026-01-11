use crate::board::Mossa;

#[derive(Clone, Copy, PartialEq, Debug)]
#[repr(u8)]
pub enum Bound { 
    None = 0,
    Exact = 1, // Il valore è preciso (PV-Node)
    Lower = 2, // Fail-high (Beta cut-off), il valore reale è >= score
    Upper = 3  // Fail-low (Alpha), il valore reale è <= score
}

#[derive(Clone, Copy)]
pub struct TTEntry {
    pub key: u64,
    pub move_data: u16,
    pub score: i16,
    pub depth: i8,
    pub bound: u8,
    pub age: u8, // Per gestire il rimpiazzo basato sulla "freschezza"
}

pub struct TranspositionTable {
    pub table: Vec<Option<TTEntry>>,
    pub size: usize,
    pub age: u8,
}

impl TranspositionTable {
    pub fn new(mb: usize) -> Self {
        let entry_size = std::mem::size_of::<Option<TTEntry>>();
        let size = (mb * 1024 * 1024) / entry_size;
        Self { 
            table: vec![None; size], 
            size,
            age: 0,
        }
    }

    pub fn clear(&mut self) {
        for slot in self.table.iter_mut() { *slot = None; }
        self.age = 0;
    }

    /// Incrementa l'età all'inizio di ogni nuova ricerca UCI "go"
    pub fn new_search(&mut self) {
        self.age = self.age.wrapping_add(1);
    }

    pub fn probe(&self, key: u64) -> Option<TTEntry> {
        let index = (key as usize) % self.size;
        match self.table[index] {
            Some(entry) if entry.key == key => Some(entry),
            _ => None,
        }
    }

    /// Memorizza una nuova voce con logica di rimpiazzo aggressiva
    pub fn store(&mut self, key: u64, score: i32, mv: Mossa, depth: i32, bound: Bound, ply: usize) {
        let index = (key as usize) % self.size;
        
        // --- NORMALIZZAZIONE PUNTEGGIO MATTO ---
        // Salviamo il matto come "distanza dal nodo attuale" per renderlo indipendente dal ply
        let mut stored_score = score;
        if score > 40000 { stored_score += ply as i32; }
        else if score < -40000 { stored_score -= ply as i32; }

        let replace = match self.table[index] {
            None => true,
            Some(old_entry) => {
                // Sostituiamo se:
                // 1. La nuova ricerca è più profonda (o uguale)
                // 2. La vecchia voce appartiene a una ricerca precedente (age diversa)
                depth as i8 >= old_entry.depth || old_entry.age != self.age
            }
        };

        if replace {
            self.table[index] = Some(TTEntry {
                key,
                move_data: mv.data,
                score: stored_score as i16,
                depth: depth as i8,
                bound: bound as u8,
                age: self.age,
            });
        }
    }

    /// Recupera il punteggio corretto per la ricerca attuale (denormalizzazione)
    pub fn get_score(&self, entry: &TTEntry, ply: usize) -> i32 {
        let score = entry.score as i32;
        if score > 40000 { return score - ply as i32; }
        if score < -40000 { return score + ply as i32; }
        score
    }
}