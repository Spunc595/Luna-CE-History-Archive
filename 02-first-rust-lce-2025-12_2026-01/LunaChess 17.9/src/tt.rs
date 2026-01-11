use crate::board::Mossa;

pub const MATE_VALUE: i32 = 49000;

#[derive(Clone, Copy, PartialEq, Debug)]
#[repr(u8)]
pub enum Bound {
    None = 0,
    Exact = 1, // PV-Node (Valore preciso)
    Lower = 2, // Beta cut-off (Score >= beta)
    Upper = 3  // Alpha node (Score <= alpha)
}

#[derive(Clone, Copy)]
pub struct TTEntry {
    pub key: u64,
    pub move_data: u16,
    pub score: i16,
    pub depth: i8,
    pub bound: u8,
    pub age: u8,
}

pub struct TranspositionTable {
    // Usiamo Box per una gestione della memoria più rigida e performante
    pub table: Box<[Option<TTEntry>]>,
    pub size: usize,
    pub age: u8,
}

impl TranspositionTable {
    pub fn new(mb: usize) -> Self {
        let entry_size = std::mem::size_of::<Option<TTEntry>>();
        // Calcoliamo il numero di entry che possono stare nei MB richiesti
        let num_entries = (mb * 1024 * 1024) / entry_size;
        
        // Inizializziamo la tabella con None
        let table = vec![None; num_entries].into_boxed_slice();
        
        Self {
            table,
            size: num_entries,
            age: 0,
        }
    }

    pub fn clear(&mut self) {
        for slot in self.table.iter_mut() {
            *slot = None;
        }
        self.age = 0;
    }

    /// Incrementa l'età all'inizio di ogni nuova ricerca UCI "go"
    pub fn new_search(&mut self) {
        self.age = self.age.wrapping_add(1);
    }

    pub fn probe(&self, key: u64) -> Option<TTEntry> {
        let index = (key as usize) % self.size;
        let entry = self.table[index];
        
        match entry {
            Some(e) if e.key == key => Some(e),
            _ => None,
        }
    }

    /// Memorizza una nuova voce con logica di rimpiazzo basata su profondità ed età
    pub fn store(&mut self, key: u64, score: i32, mv: Mossa, depth: i32, bound: Bound, ply: usize) {
        let index = (key as usize) % self.size;
        
        // --- NORMALIZZAZIONE PUNTEGGIO MATTO ---
        // Rendiamo il punteggio del matto indipendente dalla profondità attuale (ply)
        let mut stored_score = score;
        if score > 40000 { stored_score += ply as i32; }
        else if score < -40000 { stored_score -= ply as i32; }

        let replace = match self.table[index] {
            None => true,
            Some(old_entry) => {
                // Sostituiamo se:
                // 1. La nuova analisi è più profonda (più affidabile)
                // 2. La vecchia entry è "vecchia" (appartiene a una ricerca precedente)
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

    /// Denormalizza il punteggio recuperato dal TT per adattarlo al ply attuale
    pub fn get_score(&self, entry: &TTEntry, ply: usize) -> i32 {
        let score = entry.score as i32;
        if score > 40000 { return score - ply as i32; }
        if score < -40000 { return score + ply as i32; }
        score
    }

    /// Calcola la percentuale di riempimento della tabella (utile per info UCI)
    pub fn hashfull(&self) -> usize {
        let sampled = 1000;
        let occupied = self.table.iter().take(sampled).filter(|e| e.is_some()).count();
        (occupied * 1000) / sampled
    }
}