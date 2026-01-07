use crate::board::Mossa;

#[derive(Clone, Copy, PartialEq)]
pub enum Bound {
    None = 0,
    Exact = 1, // Valore esatto (siamo tra alpha e beta)
    Lower = 2, // Valore Alpha (il punteggio è ALMENO questo, o meglio) -> Fail High
    Upper = 3, // Valore Beta (il punteggio è AL MASSIMO questo, o peggio) -> Fail Low
}

#[derive(Clone, Copy)]
pub struct TTEntry {
    pub key: u64,       // Per verificare che sia la stessa posizione
    pub score: i16,     // Il punteggio salvato
    pub move_data: u16, // La mossa migliore (compressa)
    pub depth: u8,      // A che profondità abbiamo cercato
    pub bound: u8,      // Tipo di punteggio (Exact, Lower, Upper)
    pub age: u8,        // Per capire se la entry è vecchia
}

pub struct TranspositionTable {
    pub table: Vec<TTEntry>,
    pub size: usize,
    pub age: u8, // contatore per l'invecchiamento delle entry
}

impl TranspositionTable {
    pub fn new(mb_size: usize) -> Self {
        let size = (mb_size * 1024 * 1024) / std::mem::size_of::<TTEntry>();
        // Arrotonda alla potenza di 2 inferiore per indexing veloce
        let size = size.next_power_of_two() / 2; 
        
        Self {
            table: vec![TTEntry { key: 0, score: 0, move_data: 0, depth: 0, bound: 0, age: 0 }; size],
            size,
            age: 0,
        }
    }

    pub fn clear(&mut self) {
        for entry in self.table.iter_mut() {
            entry.key = 0;
            entry.depth = 0;
            entry.bound = 0;
        }
    }

    pub fn new_search(&mut self) {
        self.age = self.age.wrapping_add(1);
    }

    // Recupera una entry
    pub fn probe(&self, key: u64) -> Option<TTEntry> {
        let index = (key as usize) & (self.size - 1);
        let entry = self.table[index];

        if entry.key == key {
            return Some(entry);
        }
        None
    }

    // Salva una entry
    pub fn store(&mut self, key: u64, score: i32, best_move: Option<Mossa>, depth: i32, bound: Bound, ply: usize) {
        let index = (key as usize) & (self.size - 1);
        let entry = &mut self.table[index];

        // Sostituzione: Sovrascrivi se...
        // 1. La entry è vuota (key == 0)
        // 2. La nuova ricerca è più profonda (depth >= entry.depth)
        // 3. La entry vecchia è di una generazione precedente (entry.age != self.age)
        if entry.key == 0 || depth as u8 >= entry.depth || entry.age != self.age {
            
            // Aggiustamento Mate Score per la TT
            // Dobbiamo salvare il mate score "assoluto", non relativo al ply corrente
            let mut tt_score = score as i16;
            if score > 40000 { tt_score = (score + ply as i32) as i16; }
            else if score < -40000 { tt_score = (score - ply as i32) as i16; }

            entry.key = key;
            entry.score = tt_score;
            entry.depth = depth as u8;
            entry.bound = bound as u8;
            entry.age = self.age;
            if let Some(m) = best_move {
                entry.move_data = m.data;
            } else {
                entry.move_data = 0;
            }
        }
    }
}