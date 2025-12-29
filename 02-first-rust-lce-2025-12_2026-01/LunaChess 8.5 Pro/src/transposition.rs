use crate::board::Mossa;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Bound {
    Exact = 0,
    Lower = 1, // Beta Cut-off
    Upper = 2, // Alpha Node
}

/* Data Layout Ottimizzato (64 bit):
- Score (16 bit): 0-15
- Move (16 bit): 16-31
- Depth (8 bit): 32-39
- Bound (2 bit): 40-41
- Age/Static (22 bit): 42-63 (riservato per evoluzioni future)
*/

pub struct DecodedEntry {
    pub score: i16,
    pub mossa_u16: u16,
    pub depth: u8,
    pub bound: Bound,
}

pub struct TranspositionTable {
    // Usiamo due array separati per mantenere l'allineamento a 64 bit,
    // ma la logica di accesso garantisce che siano caricate vicine in cache.
    table: Vec<AtomicU64>,
    key_table: Vec<AtomicU64>,
    mask: usize,
}

impl TranspositionTable {
    pub fn new(size_mb: usize) -> Self {
        // Ogni entry (chiave + dati) occupa 16 byte.
        let num_entries = (size_mb * 1024 * 1024 / 16).next_power_of_two();
        let mut table = Vec::with_capacity(num_entries);
        let mut key_table = Vec::with_capacity(num_entries);
        
        for _ in 0..num_entries {
            table.push(AtomicU64::new(0));
            key_table.push(AtomicU64::new(0));
        }
        
        Self { table, key_table, mask: num_entries - 1 }
    }

    pub fn store(&self, hash: u64, score: i16, mossa_u16: u16, depth: u8, bound: Bound) {
        let idx = (hash as usize) & self.mask;
        
        // Strategia di rimpiazzo: "Always Replace" se la nuova profondità è maggiore o uguale.
        // Carichiamo i dati esistenti per controllare la profondità (opzionale ma consigliato).
        let old_data = self.table[idx].load(Ordering::Relaxed);
        let old_depth = ((old_data >> 32) & 0xFF) as u8;

        // Se la mossa vecchia è più profonda, non sovrascrivere (a meno che non sia un punteggio esatto)
        if depth < old_depth && bound != Bound::Exact {
            return;
        }

        let b_val = bound as u64;
        let data = (score as u16 as u64) |
                   ((mossa_u16 as u64) << 16) |
                   ((depth as u64) << 32) |
                   (b_val << 40);

        // Store atomico con barriere di memoria:
        // 1. Scriviamo i dati (Relaxed)
        // 2. Scriviamo la chiave con Release (assicura che 'data' sia visibile)
        self.table[idx].store(data, Ordering::Relaxed);
        self.key_table[idx].store(hash, Ordering::Release);
    }

    pub fn probe(&self, hash: u64) -> Option<DecodedEntry> {
        let idx = (hash as usize) & self.mask;
        
        // Acquire: se leggiamo l'hash corretto, tutto ciò che è stato scritto prima 
        // con Release (ovvero i 'data') è ora visibile a questo thread.
        if self.key_table[idx].load(Ordering::Acquire) == hash {
            let data = self.table[idx].load(Ordering::Relaxed);
            
            return Some(DecodedEntry {
                score: (data & 0xFFFF) as u16 as i16,
                mossa_u16: ((data >> 16) & 0xFFFF) as u16,
                depth: ((data >> 32) & 0xFF) as u8,
                bound: match (data >> 40) & 0x3 {
                    0 => Bound::Exact,
                    1 => Bound::Lower,
                    _ => Bound::Upper,
                },
            });
        }
        None
    }

    // Normalizzazione punteggi di matto: trasforma il matto relativo al ply
    // in un matto relativo alla posizione (indipendente dalla profondità).
    pub fn score_to_tt(&self, score: i16, ply: usize) -> i16 {
        if score > 28000 { score + ply as i16 }
        else if score < -28000 { score - ply as i16 }
        else { score }
    }

    pub fn score_from_tt(&self, score: i16, ply: usize) -> i16 {
        if score > 28000 { score - ply as i16 }
        else if score < -28000 { score + ply as i16 }
        else { score }
    }

    pub fn clear(&self) {
        for i in 0..self.table.len() {
            self.table[i].store(0, Ordering::Relaxed);
            self.key_table[i].store(0, Ordering::Relaxed);
        }
    }
}

impl DecodedEntry {
    /// Ritorna la mossa se presente nella entry
    pub fn mossa(&self) -> Option<Mossa> {
        Mossa::from_u16(self.mossa_u16)
    }
}