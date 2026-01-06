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
}

// Implementazione semplice di default
impl Default for TTEntry {
    fn default() -> Self {
        TTEntry {
            hash: 0,
            score: 0,
            depth: 0,
            flag: Flag::Exact,
            best_move: None,
        }
    }
}

pub struct TranspositionTable {
    table: Vec<TTEntry>,
    size: usize,
}

impl TranspositionTable {
    pub fn new(mb: usize) -> Self {
        // Calcola quante entry ci stanno in N megabyte
        // sizeof(TTEntry) è circa 24 byte.
        let size = (mb * 1024 * 1024) / std::mem::size_of::<TTEntry>();
        // Usiamo una potenza di 2 per velocità nei calcoli bitwise se volessimo ottimizzare,
        // ma per ora va bene size diretto.
        
        TranspositionTable {
            table: vec![TTEntry::default(); size],
            size,
        }
    }

    pub fn clear(&mut self) {
        for entry in self.table.iter_mut() {
            *entry = TTEntry::default();
        }
    }

    // Probe: Cerca se abbiamo già analizzato questa posizione
    pub fn probe(&self, hash: u64) -> Option<TTEntry> {
        let index = (hash as usize) % self.size;
        let entry = self.table[index];

        if entry.hash == hash {
            Some(entry)
        } else {
            None
        }
    }

    // Store: Salva il risultato dell'analisi
    pub fn store(&mut self, hash: u64, score: i32, depth: u8, flag: Flag, best_move: Option<Mossa>) {
        let index = (hash as usize) % self.size;
        
        // Strategia di rimpiazzo semplice: 
        // Sovrascrivi sempre (o potremmo preferire depth maggiore).
        // Per ora sovrascriviamo se la nuova depth è >= o se è un hash diverso (collisione).
        let entry = &mut self.table[index];
        
        // Sovrascrivi se è una nuova posizione o se l'analisi è più profonda
        if entry.hash != hash || depth >= entry.depth {
            *entry = TTEntry {
                hash,
                score,
                depth,
                flag,
                best_move,
            };
        }
    }
}