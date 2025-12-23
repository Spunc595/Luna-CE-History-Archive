use crate::movegen::Mossa;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TipoNodo {
    Exact,
    LowerBound,
    UpperBound,
}

#[derive(Clone, Copy)]
pub struct TranspositionEntry {
    pub chiave: u64,
    pub valore: i32,
    pub profondita: u8,
    pub tipo: TipoNodo,
    pub mossa: Option<Mossa>,
    pub eta: u32,
}

pub struct TranspositionTable {
    entries: Vec<Option<TranspositionEntry>>,
    size: usize,
    eta: u32,
}

impl TranspositionTable {
    pub fn nuova(size_mb: usize) -> Self {
        let size = (size_mb * 1024 * 1024) / std::mem::size_of::<Option<TranspositionEntry>>();
        TranspositionTable {
            entries: vec![None; size],
            size,
            eta: 0,
        }
    }
    
    pub fn inserisci(&mut self, chiave: u64, valore: i32, profondita: u8, tipo: TipoNodo, mossa: Option<Mossa>) {
        let indice = (chiave as usize) % self.size;
        self.entries[indice] = Some(TranspositionEntry {
            chiave,
            valore,
            profondita,
            tipo,
            mossa,
            eta: self.eta,
        });
    }
    
    pub fn cerca(&self, chiave: u64) -> Option<TranspositionEntry> {
        let indice = (chiave as usize) % self.size;
        if let Some(entry) = self.entries[indice] {
            if entry.chiave == chiave {
                return Some(entry);
            }
        }
        None
    }
    
    pub fn incrementa_eta(&mut self) {
        self.eta = self.eta.wrapping_add(1);
    }
}