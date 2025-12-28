// src/transposition.rs

use crate::board::Mossa;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Bound { Exact, Lower, Upper }

pub struct TTEntry {
    pub key: u64,     // Hash completo per verifica
    pub data: u64,    // Dati compressi (Score, Mossa, Depth, Bound)
}

/* Data Layout (64 bit):
- Score (16 bit): 0-15
- Move (16 bit): 16-31
- Depth (8 bit): 32-39
- Bound (2 bit): 40-41
- Age/Static (22 bit): 42-63
*/

pub struct TranspositionTable {
    table: Vec<AtomicU64>,
    key_table: Vec<AtomicU64>,
    mask: usize,
}

impl TranspositionTable {
    pub fn new(size_mb: usize) -> Self {
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
        let b_val = match bound { Bound::Exact => 0, Bound::Lower => 1, Bound::Upper => 2 };
        
        // Comprime i dati
        let data = (score as u16 as u64) | 
                   ((mossa_u16 as u64) << 16) | 
                   ((depth as u64) << 32) | 
                   ((b_val as u64) << 40);

        // Write Atomico: scriviamo prima il dato, poi la chiave (hash)
        // Se un altro thread legge la chiave vecchia, ignorerà il dato nuovo
        self.table[idx].store(data, Ordering::Relaxed);
        self.key_table[idx].store(hash, Ordering::Release);
    }

    pub fn probe(&self, hash: u64) -> Option<DecodedEntry> {
        let idx = (hash as usize) & self.mask;
        
        // Read Atomico
        let key = self.key_table[idx].load(Ordering::Acquire);
        if key == hash {
            let data = self.table[idx].load(Ordering::Relaxed);
            return Some(DecodedEntry {
                score: (data & 0xFFFF) as u16 as i16,
                mossa_raw: ((data >> 16) & 0xFFFF) as u16,
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

pub struct DecodedEntry {
    pub score: i16,
    pub mossa_raw: u16,
    pub depth: u8,
    pub bound: Bound,
}

impl DecodedEntry {
    pub fn mossa(&self) -> Option<Mossa> {
        if self.mossa_raw == 0 { None } else { Mossa::from_u32(self.mossa_raw as u32) }
    }
    pub fn score(&self) -> i16 { self.score }
    pub fn depth(&self) -> u8 { self.depth }
    pub fn bound(&self) -> Bound { self.bound }
}