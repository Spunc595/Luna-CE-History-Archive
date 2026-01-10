use crate::board::Mossa;

#[derive(Clone, Copy, PartialEq)]
pub enum Flag {
    Exact,
    Alpha, // Upper Bound (Punteggio <= Alpha)
    Beta,  // Lower Bound (Punteggio >= Beta)
}

#[derive(Clone, Copy)]
pub struct TTEntry {
    pub hash: u64,
    pub score: i32,
    pub depth: u8,
    pub flag: Flag,
    pub best_move: Option<Mossa>,
    pub age: u8,
}

impl Default for TTEntry {
    fn default() -> Self {
        TTEntry {
            hash: 0,
            score: 0,
            depth: 0,
            flag: Flag::Exact,
            best_move: None,
            age: 0,
        }
    }
}

pub struct TranspositionTable {
    table: Vec<TTEntry>,
    mask: usize,      // Usata per l'indicizzazione veloce bitwise
    size: usize,      // Numero totale di entry
    pub current_age: u8,
}

impl TranspositionTable {
    pub fn new(mb: usize) -> Self {
        let entry_size = std::mem::size_of::<TTEntry>();
        let mut n_entries = (mb * 1024 * 1024) / entry_size;

        // La dimensione deve essere una potenza di 2 per l'indicizzazione veloce
        if !n_entries.is_power_of_two() {
            n_entries = n_entries.next_power_of_two() / 2;
        }

        TranspositionTable {
            table: vec![TTEntry::default(); n_entries],
            mask: n_entries - 1,
            size: n_entries,
            current_age: 0,
        }
    }

    pub fn clear(&mut self) {
        for entry in self.table.iter_mut() {
            *entry = TTEntry::default();
        }
        self.current_age = 0;
    }

    #[inline(always)]
    pub fn probe(&self, hash: u64) -> Option<TTEntry> {
        let index = (hash as usize) & self.mask;
        let entry = self.table[index];

        if entry.hash == hash {
            Some(entry)
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn store(&mut self, hash: u64, score: i32, depth: u8, flag: Flag, best_move: Option<Mossa>) {
        let index = (hash as usize) & self.mask;
        let entry = &mut self.table[index];

        // Sostituzione: "Deep or New" + "Ageing"
        // Sostituiamo se la posizione è nuova, se l'analisi è più profonda,
        // o se la voce vecchia appartiene a una ricerca precedente.
        if entry.hash != hash || depth >= entry.depth || entry.age != self.current_age {
            *entry = TTEntry {
                hash,
                score,
                depth,
                flag,
                best_move,
                age: self.current_age,
            };
        }
    }

    /// Restituisce il valore per l'UCI "hashfull" (permille di riempimento)
    pub fn hashfull(&self) -> u32 {
        // Campioniamo le prime 1000 entry per una stima statistica veloce
        let sample_size = self.size.min(1000);
        let used = self.table.iter().take(sample_size).filter(|e| e.hash != 0).count();
        (used as f32 / sample_size as f32 * 1000.0) as u32
    }

    pub fn increment_age(&mut self) {
        self.current_age = self.current_age.wrapping_add(1);
    }

    pub fn size(&self) -> usize {
        self.size
    }
}