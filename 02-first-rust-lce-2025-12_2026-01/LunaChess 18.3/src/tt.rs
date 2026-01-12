use crate::board::Mossa;

pub const MATE_VALUE: i32 = 49000;

#[derive(Clone, Copy, PartialEq, Debug)]
#[repr(u8)]
pub enum Bound {
    None = 0,
    Exact = 1,  // Punteggio preciso (PV-Node)
    Lower = 2,  // Cut-node (Beta cut)
    Upper = 3,  // All-node (Alpha fail-low)
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
    pub table: Box<[TTEntry]>,
    pub size: usize,
    pub age: u8,
}

impl TranspositionTable {
    pub fn new(mb: usize) -> Self {
        let entry_size = std::mem::size_of::<TTEntry>();
        // Calcoliamo il numero di entry basandoci sui MB allocati
        let num_entries = (mb * 1024 * 1024) / entry_size;
        
        // Inizializziamo la tabella con entry vuote (key = 0)
        let table = vec![TTEntry {
            key: 0,
            move_data: 0,
            score: 0,
            depth: 0,
            bound: 0,
            age: 0,
        }; num_entries].into_boxed_slice();
        
        Self {
            table,
            size: num_entries,
            age: 0,
        }
    }

    pub fn clear(&mut self) {
        for entry in self.table.iter_mut() {
            entry.key = 0;
            entry.move_data = 0;
            entry.depth = 0;
        }
        self.age = 0;
    }

    pub fn new_search(&mut self) {
        // Incrementiamo l'età per distinguere le entry di questa ricerca
        self.age = self.age.wrapping_add(1);
    }

    #[inline(always)]
    pub fn probe(&self, key: u64) -> Option<TTEntry> {
        let index = (key as usize) % self.size;
        let entry = self.table[index];
        
        if entry.key == key {
            Some(entry)
        } else {
            None
        }
    }

    pub fn store(&mut self, key: u64, score: i32, mv: Mossa, depth: i32, bound: Bound, ply: usize) {
        let index = (key as usize) % self.size;
        let old_entry = &self.table[index];

        // Gestione del punteggio di matto per renderlo relativo alla radice
        let mut stored_score = score;
        if score > 40000 { stored_score += ply as i32; }
        else if score < -40000 { stored_score -= ply as i32; }

        // Manteniamo la mossa precedente se quella nuova è nulla
        let mut move_to_store = mv.data;
        if move_to_store == 0 && old_entry.key == key {
            move_to_store = old_entry.move_data;
        }

        // Strategia di rimpiazzo: sovrascrivi se la nuova è più profonda 
        // o se l'entry vecchia appartiene a una ricerca precedente
        let replace = old_entry.key == 0 
            || old_entry.age != self.age 
            || depth as i8 >= old_entry.depth;

        if replace {
            self.table[index] = TTEntry {
                key,
                move_data: move_to_store,
                score: stored_score as i16,
                depth: depth as i8,
                bound: bound as u8,
                age: self.age,
            };
        }
    }

    #[inline(always)]
    pub fn get_score(&self, entry: &TTEntry, ply: usize) -> i32 {
        let score = entry.score as i32;
        // Riconvertiamo il punteggio di matto da assoluto a relativo al ply corrente
        if score > 40000 { return score - ply as i32; }
        if score < -40000 { return score + ply as i32; }
        score
    }
}