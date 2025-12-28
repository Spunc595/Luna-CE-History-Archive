// src/transposition.rs

use crate::board::{Mossa, MoveFlag, Pezzo, Casella};
use std::cell::UnsafeCell;
use std::mem;
use std::sync::atomic::{AtomicU64, Ordering};

// ===== CONSTANTS =====
const CACHE_LINE_SIZE: usize = 64;
const ENTRY_SIZE: usize = std::mem::size_of::<TTEntry>(); // 16 bytes
const CLUSTER_SIZE: usize = 4; // 4 entries fit perfectly in 64 bytes

// Valori per la gestione dei punteggi di matto
const MATE_BOUND: i16 = 20000; // Deve essere minore di VITTORIA in search.rs ma alto abbastanza
const TT_MATE: i16 = 30000;    // Valore interno per il matto nella TT

// ===== ENUMS =====

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Bound {
    None = 0,
    Exact = 1,
    Lower = 2, // Beta cutoff
    Upper = 3, // Alpha cutoff
}

// ===== TT ENTRY STRUCT (16 Bytes) =====
// Non usiamo align(64) qui, ma sul Cluster.
// Struttura packed per massima efficienza.

#[derive(Clone, Copy)]
#[repr(C)] // Layout C garantito
pub struct TTEntry {
    pub key: u64,           // 8 bytes: Hash completo (o parte alta)
    pub data: u64,          // 8 bytes: Dati impacchettati
}

impl TTEntry {
    // Bit layout of 'data' (u64):
    // 0-15:  Move (16 bits)
    // 16-31: Score (16 bits)
    // 32-47: Eval (16 bits)
    // 48-55: Depth (8 bits)
    // 56-57: Bound (2 bits)
    // 58-63: Generation (6 bits)

    #[inline(always)]
    pub fn new(key: u64, score: i16, eval: i16, m: Option<Mossa>, depth: u8, bound: Bound, gen: u8) -> Self {
        let move_data = m.map_or(0u16, |mv| compress_move(mv));
        
        let mut data = 0u64;
        data |= move_data as u64;
        data |= (score as u16 as u64) << 16;
        data |= (eval as u16 as u64) << 32;
        data |= (depth as u64) << 48;
        data |= (bound as u8 as u64) << 56;
        data |= ((gen & 0x3F) as u64) << 58; // Usiamo solo 6 bit per la generazione (0-63)

        // Usiamo lo XOR key ^ data per un controllo di integrità rudimentale nei sistemi lock-free
        // Ma per ora salviamo la key pulita per semplicità di matching.
        // In motori avanzati si usa: key ^ data nei 16 bit alti della key.
        
        TTEntry { key, data }
    }

    #[inline(always)]
    pub fn is_valid(&self, key: u64) -> bool {
        // Verifica se la chiave corrisponde (XOR per gestire eventuali collisioni parziali)
        // Qui controlliamo l'uguaglianza esatta.
        self.key == key
    }

    #[inline(always)]
    pub fn score(&self) -> i16 {
        ((self.data >> 16) & 0xFFFF) as i16
    }

    #[inline(always)]
    pub fn eval(&self) -> i16 {
        ((self.data >> 32) & 0xFFFF) as i16
    }

    #[inline(always)]
    pub fn mossa(&self) -> Option<Mossa> {
        let md = (self.data & 0xFFFF) as u16;
        if md == 0 { None } else { decompress_move(md) }
    }

    #[inline(always)]
    pub fn depth(&self) -> u8 {
        ((self.data >> 48) & 0xFF) as u8
    }

    #[inline(always)]
    pub fn bound(&self) -> Bound {
        unsafe { mem::transmute(((self.data >> 56) & 0x3) as u8) }
    }

    #[inline(always)]
    pub fn generation(&self) -> u8 {
        ((self.data >> 58) & 0x3F) as u8
    }
}

// ===== CLUSTER STRUCT (64 Bytes) =====
// Allineato alla cache line per ottimizzare le letture della RAM
#[derive(Clone, Copy)]
#[repr(C, align(64))]
struct Cluster {
    entries: [TTEntry; CLUSTER_SIZE],
}

impl Default for Cluster {
    fn default() -> Self {
        Cluster {
            entries: [TTEntry { key: 0, data: 0 }; CLUSTER_SIZE],
        }
    }
}

// ===== TRANSPOSITION TABLE =====

// Sync è necessario per condividere la TT tra thread (Lazy SMP)
// Usiamo UnsafeCell per permettere mutabilità condivisa (Lock-Free)
pub struct TranspositionTable {
    table: Vec<UnsafeCell<Cluster>>,
    size: usize,
    generation: u8, // Si incrementa ad ogni ricerca
}

unsafe impl Sync for TranspositionTable {}
unsafe impl Send for TranspositionTable {}

impl TranspositionTable {
    pub fn new(mb_size: usize) -> Self {
        let entry_count = (mb_size * 1024 * 1024) / std::mem::size_of::<Cluster>();
        // Arrotonda alla potenza di 2 inferiore per mascheramento veloce
        let size = entry_count.next_power_of_two(); 
        
        let mut table = Vec::with_capacity(size);
        for _ in 0..size {
            table.push(UnsafeCell::new(Cluster::default()));
        }

        println!("Transposition Table: {} MB, {} Clusters, {} Entries", 
                 (size * std::mem::size_of::<Cluster>()) / (1024*1024), size, size * 4);

        TranspositionTable {
            table,
            size,
            generation: 0,
        }
    }

    pub fn set_generation(&mut self, gen: u8) {
        self.generation = gen;
    }

    pub fn clear(&mut self) {
        for cluster in self.table.iter_mut() {
            unsafe { *cluster.get() = Cluster::default(); }
        }
    }

    // Corregge i punteggi di matto in relazione al ply corrente
    // Quando leggiamo dalla TT: Matto in 5 (score 29995) al ply 10
    // Al ply 20 deve diventare Matto in 5 (score 29995).
    // Ma internamente salviamo "Matto in X mosse ASSOLUTE".
    pub fn score_to_tt(&self, score: i16, ply: usize) -> i16 {
        if score > MATE_BOUND {
            score + ply as i16
        } else if score < -MATE_BOUND {
            score - ply as i16
        } else {
            score
        }
    }

    pub fn score_from_tt(&self, score: i16, ply: usize) -> i16 {
        if score > MATE_BOUND {
            score - ply as i16
        } else if score < -MATE_BOUND {
            score + ply as i16
        } else {
            score
        }
    }

    #[inline(always)]
    pub fn probe(&self, key: u64) -> Option<TTEntry> {
        let index = (key as usize) & (self.size - 1);
        let cluster_ptr = self.table[index].get();

        unsafe {
            let cluster = &*cluster_ptr; // Accesso lock-free
            
            // Scansiona le 4 entrate nel cluster
            for i in 0..CLUSTER_SIZE {
                let entry = &cluster.entries[i];
                if entry.key == key {
                     // Trovato! 
                     // Nota: In un ambiente multithread, qui potremmo leggere dati sporchi.
                     // Si può aggiungere un checksum xor key^data per validare, 
                     // ma per ora ci fidiamo della chiave a 64 bit.
                     return Some(*entry);
                }
            }
        }
        None
    }

    #[inline(always)]
    pub fn probe_move(&self, key: u64) -> Option<Mossa> {
        self.probe(key).and_then(|e| e.mossa())
    }

    #[inline(always)]
    pub fn store(&self, key: u64, score: i16, eval: i16, m: Option<Mossa>, depth: u8, bound: Bound) {
        let index = (key as usize) & (self.size - 1);
        let cluster_ptr = self.table[index].get();

        // Strategia di rimpiazzo (Replacement Strategy)
        // 1. Cerca slot vuoto o chiave identica
        // 2. Se pieno, sostituisci in base a: Profondità, Generazione (Aging)

        unsafe {
            let cluster = &mut *cluster_ptr; // Mutabilità unsafe
            
            let mut replace_idx = 0;
            let mut min_score = i32::MAX;

            for i in 0..CLUSTER_SIZE {
                let entry = &cluster.entries[i];

                // 1. Sovrascrittura esatta o slot vuoto
                if entry.key == key || entry.key == 0 {
                    replace_idx = i;
                    break;
                }

                // 2. Calcolo punteggio per rimpiazzo
                // Preferiamo tenere entry della generazione corrente e profondità alta
                let entry_gen = entry.generation();
                let entry_depth = entry.depth();
                
                // Penalità per entry vecchie
                let age = self.generation.wrapping_sub(entry_gen);
                
                // Score basso = candidato al rimpiazzo
                // Vecchio (age alto) -> score basso
                // Bassa profondità -> score basso
                let score = (entry_depth as i32) - (age as i32 * 20);

                if score < min_score {
                    min_score = score;
                    replace_idx = i;
                }
            }

            // Scriviamo la nuova entry
            // Preserviamo la mossa se quella nuova è None e stiamo sovrascrivendo la stessa chiave
            let mut new_move = m;
            if new_move.is_none() && cluster.entries[replace_idx].key == key {
                new_move = cluster.entries[replace_idx].mossa();
            }

            cluster.entries[replace_idx] = TTEntry::new(
                key, score, eval, new_move, depth, bound, self.generation
            );
        }
    }
    
    // Metodo helper per prefetching (ottimizzazione avanzata x86)
    #[inline(always)]
    pub fn prefetch(&self, key: u64) {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        unsafe {
            let index = (key as usize) & (self.size - 1);
            let ptr = self.table[index].get();
            // _mm_prefetch 0 = NTA (Non-Temporal data), 1-3 levels cache
            std::arch::x86_64::_mm_prefetch(ptr as *const i8, std::arch::x86_64::_MM_HINT_T0);
        }
    }
}

// ===== MOVE COMPRESSION HELPER =====
// Comprime una Mossa in 16 bit per risparmiare spazio nella TT

fn compress_move(m: Mossa) -> u16 {
    let from = m.da().indice() as u16; // 6 bit (0-63)
    let to = m.a().indice() as u16;    // 6 bit (0-63)
    
    // Promozione: usiamo 3 bit (0=None, 1=Q, 2=R, 3=B, 4=N)
    let promo = match m.promozione() {
        None => 0,
        Some(Pezzo::Regina) => 1,
        Some(Pezzo::Torre) => 2,
        Some(Pezzo::Alfiere) => 3,
        Some(Pezzo::Cavallo) => 4,
        _ => 0,
    };

    // Layout: 
    // FFFFFF TTTTTT PPP X (16 bit)
    // F=From, T=To, P=Promo, X=Reserved
    
    (from << 10) | (to << 4) | (promo << 1)
}

fn decompress_move(data: u16) -> Option<Mossa> {
    let from_idx = (data >> 10) & 0x3F;
    let to_idx = (data >> 4) & 0x3F;
    let promo_code = (data >> 1) & 0x7;

    let from = Casella::da_indice(from_idx as usize)?;
    let to = Casella::da_indice(to_idx as usize)?;
    
    let promo = match promo_code {
        1 => Some(Pezzo::Regina),
        2 => Some(Pezzo::Torre),
        3 => Some(Pezzo::Alfiere),
        4 => Some(Pezzo::Cavallo),
        _ => None,
    };

    // Ricostruzione flag (semplificata, poiché la TT serve solo per ordinamento mosse e refutation)
    // Flag preciso non è critico per l'ordinamento, ma se serve bisogna ricalcolarlo dalla board
    // o usare bit extra. Per ora Promotion è deducibile.
    let mut flag = MoveFlag::None;
    if promo.is_some() { flag = MoveFlag::Promotion; }
    
    Some(Mossa::new_with_flag(from, to, promo, flag))
}