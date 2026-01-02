use crate::board::Mossa;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Bound { 
    Exact, // Punteggio preciso
    Lower, // Fall-high (Beta cutoff): lo score reale è >= di questo
    Upper  // Fall-low (Alpha): lo score reale è <= di questo
}

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
        
        // Uso vec![None; num_entries] che è più veloce di un ciclo manuale
        let table = vec![None; num_entries];
        
        TranspositionTable { table, size: num_entries }
    }

    pub fn store(&mut self, key: u64, score: i16, mossa: u16, depth: u8, bound: Bound, ply: usize) {
        if self.size == 0 { return; }
        
        // Calcolo indice con modulo (o bitwise AND se size fosse potenza di 2)
        let index = (key as usize) % self.size;
        
        // Normalizzazione del punteggio del matto:
        // Memorizziamo il matto come "distanza dalla posizione attuale"
        let mut tt_score = score;
        if score > 28000 { tt_score += ply as i16; }
        else if score < -28000 { tt_score -= ply as i16; }
        
        // Sostituzione: scriviamo se la profondità è maggiore o se è un nuovo nodo
        if let Some(entry) = self.table[index] {
            // Se l'entry esistente è più profonda e non è la stessa posizione, non sovrascrivere
            if entry.key == key && entry.depth > depth { 
                return; 
            }
        }
        
        self.table[index] = Some(TTEntry { 
            key, 
            score: tt_score, 
            mossa, 
            depth, 
            bound 
        });
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

    /// Converte il punteggio dalla TT (relativo al nodo) a punteggio reale (relativo alla radice)
    pub fn score_from_tt(&self, score: i16, ply: usize) -> i16 {
        if score > 28000 { return score - ply as i16; }
        if score < -28000 { return score + ply as i16; }
        score
    }
    
    pub fn clear(&mut self) {
        for entry in self.table.iter_mut() { 
            *entry = None; 
        }
    }
}