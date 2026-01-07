use crate::board::Mossa;

#[derive(Clone, Copy, PartialEq)]
pub enum Flag {
    Exact,
    Alpha, // Upper Bound
    Beta,  // Lower Bound
}

#[derive(Clone, Copy)]
pub struct TTEntry {
    pub hash: u64,
    pub score: i32,
    pub depth: u8,
    pub flag: Flag,
    pub best_move: Option<Mossa>,
    pub age: u8, // Aggiunto per gestire la sostituzione intelligente
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
    size: usize,
    pub current_age: u8, // Incrementato ad ogni nuova ricerca
}

impl TranspositionTable {
    pub fn new(mb: usize) -> Self {
        // Ogni entry occupa circa 24-32 byte a seconda dell'allineamento
        let entry_size = std::mem::size_of::<TTEntry>();
        let size = (mb * 1024 * 1024) / entry_size;
        
        TranspositionTable {
            table: vec![TTEntry::default(); size],
            size,
            current_age: 0,
        }
    }

    pub fn clear(&mut self) {
        for entry in self.table.iter_mut() {
            *entry = TTEntry::default();
        }
        self.current_age = 0;
    }

    // Restituisce la dimensione totale
    pub fn size(&self) -> usize {
        self.size
    }

    // Conta quante entry sono effettivamente occupate (per hashfull)
    pub fn count(&self) -> usize {
        // Per performance, campioniamo solo le prime 1000 entry per stimare il riempimento
        // o contiamo tutto se la tabella è piccola.
        self.table.iter().take(1000).filter(|e| e.hash != 0).count() * (self.size / 1000).max(1)
    }

    pub fn probe(&self, hash: u64) -> Option<TTEntry> {
        let index = (hash as usize) % self.size;
        let entry = self.table[index];

        if entry.hash == hash {
            Some(entry)
        } else {
            None
        }
    }

    pub fn store(&mut self, hash: u64, score: i32, depth: u8, flag: Flag, best_move: Option<Mossa>) {
        let index = (hash as usize) % self.size;
        let entry = &mut self.table[index];

        // STRATEGIA DI RIMPIAZZO:
        // Sostituiamo se:
        // 1. La posizione è nuova (hash diverso)
        // 2. La nuova analisi è più profonda (depth maggiore)
        // 3. La vecchia analisi appartiene a una ricerca precedente (age diverso)
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

    // Da chiamare all'inizio di ogni "go" in uci.rs per rinfrescare la tabella
    pub fn increment_age(&mut self) {
        self.current_age = self.current_age.wrapping_add(1);
    }
}