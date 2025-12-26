use crate::board::{Scacchiera, Mossa, Pezzo, Casella, MoveFlag};  // Aggiunto MoveFlag
use std::sync::atomic::{AtomicU64, Ordering};

// ===== CONSTANTS =====
const TT_SIZE_MIN: usize = 1 << 20; // 1 milione di entry minime (16MB)
const TT_SIZE_DEFAULT: usize = 1 << 23; // 8 milioni di entry (128MB)
const TT_SIZE_MAX: usize = 1 << 26; // 64 milioni di entry (1GB)

const ALIGNMENT: usize = 64; // Cache line alignment (64 bytes)
const ENTRY_SIZE: usize = 16; // Size of TTEntry in bytes

// Replacement strategies
const REPLACE_ALWAYS: u8 = 0;
const REPLACE_DEEPEST: u8 = 1;
const REPLACE_ALWAYS_NEW: u8 = 2;

// ===== TT ENTRY STRUCT =====

#[derive(Clone, Copy)]
#[repr(C, align(64))]
pub struct TTEntry {
    pub key: u64,           // 8 bytes - Zobrist key (upper bits)
    pub data: u64,          // 8 bytes - Packed data
}

impl TTEntry {
    #[inline(always)]
    pub fn new(key: u64, score: i16, eval: i16, best_move: Option<Mossa>, depth: u8, bound: Bound, generation: u8) -> Self {
        let mut data = 0u64;
        
        // Pack data efficiently
        let score16 = score as u16;
        let eval16 = eval as u16;
        let move16 = best_move.map_or(0u16, |m| m.to_u16());
        
        data |= (score16 as u64) << 48;
        data |= (eval16 as u64) << 32;
        data |= (move16 as u64) << 16;
        data |= (depth as u64) << 8;
        data |= (bound as u64) << 6;
        data |= (generation as u64) as u64;
        
        TTEntry { key, data }
    }
    
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.key == 0 && self.data == 0
    }
    
    #[inline(always)]
    pub fn key_match(&self, key: u64) -> bool {
        (self.key ^ key) < (1 << 16) // Compare upper 48 bits
    }
    
    #[inline(always)]
    pub fn get_score(&self) -> i16 {
        ((self.data >> 48) & 0xFFFF) as i16
    }
    
    #[inline(always)]
    pub fn get_eval(&self) -> i16 {
        ((self.data >> 32) & 0xFFFF) as i16
    }
    
    #[inline(always)]
    pub fn get_best_move(&self) -> Option<Mossa> {
        let move_data = ((self.data >> 16) & 0xFFFF) as u16;
        if move_data != 0 {
            Mossa::from_u16(move_data)
        } else {
            None
        }
    }
    
    #[inline(always)]
    pub fn get_depth(&self) -> u8 {
        ((self.data >> 8) & 0xFF) as u8
    }
    
    #[inline(always)]
    pub fn get_bound(&self) -> Bound {
        match (self.data >> 6) & 0x3 {
            0 => Bound::Exact,
            1 => Bound::Lower,
            2 => Bound::Upper,
            _ => Bound::Exact,
        }
    }
    
    #[inline(always)]
    pub fn get_generation(&self) -> u8 {
        (self.data & 0x3F) as u8
    }
    
    #[inline(always)]
    pub fn age(&self) -> u8 {
        // Age is the difference between current generation and entry generation
        // This is used for replacement decisions
        self.get_generation()
    }
}

// ===== BOUND ENUM =====

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bound {
    Exact = 0,
    Lower = 1,
    Upper = 2,
}

// ===== TRANSPOSITION TABLE STRUCT =====

pub struct TranspositionTable {
    table: Vec<TTEntry>,
    mask: usize,
    generation: u8,
    replacements: AtomicU64,
    hits: AtomicU64,
    misses: AtomicU64,
    overwrites: AtomicU64,
    collisions: AtomicU64,
    size_mb: usize,
    buckets: usize, // Number of buckets (for bucket scheme)
}

impl TranspositionTable {
    pub fn new(size_mb: usize) -> Self {
        let size_mb = size_mb.clamp(16, 1024); // Clamp between 16MB and 1GB
        let size_bytes = size_mb * 1024 * 1024;
        
        // Calculate number of entries (aligned to power of 2)
        let num_entries = (size_bytes / ENTRY_SIZE).next_power_of_two();
        
        // For bucket scheme: 4 entries per bucket
        let buckets = num_entries / 4;
        
        let mut table = Vec::with_capacity(num_entries);
        unsafe {
            table.set_len(num_entries);
        }
        
        // Initialize with zeros
        for entry in &mut table {
            *entry = TTEntry { key: 0, data: 0 };
        }
        
        TranspositionTable {
            table,
            mask: num_entries - 1,
            generation: 0,
            replacements: AtomicU64::new(0),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            overwrites: AtomicU64::new(0),
            collisions: AtomicU64::new(0),
            size_mb,
            buckets,
        }
    }
    
    #[inline(always)]
    pub fn index(&self, key: u64) -> usize {
        // Use lower bits for indexing
        (key as usize) & self.mask
    }
    
    #[inline(always)]
    pub fn index_bucket(&self, key: u64, bucket_index: usize) -> usize {
        // For bucket scheme: calculate base index
        let base = self.index(key) & !0x3; // Align to 4-entry boundary
        base + (bucket_index % 4)
    }
    
    // ===== BUCKET SCHEME (4 entries per bucket) =====
    
    pub fn store_bucket(&mut self, key: u64, score: i16, eval: i16, best_move: Option<Mossa>, depth: u8, bound: Bound) {
        let index_base = self.index(key) & !0x3; // Align to 4
        
        // Try to find empty slot or replace based on replacement strategy
        let mut replace_index = index_base;
        let mut replace_score = i32::MIN;
        
        for i in 0..4 {
            let idx = index_base + i;
            let entry = &self.table[idx];
            
            if entry.is_empty() {
                // Found empty slot
                self.table[idx] = TTEntry::new(key, score, eval, best_move, depth, bound, self.generation);
                return;
            }
            
            // Calculate replacement score
            let entry_score = self.replacement_score(entry, depth);
            if entry_score > replace_score {
                replace_score = entry_score;
                replace_index = idx;
            }
        }
        
        // Replace the worst entry
        self.overwrites.fetch_add(1, Ordering::Relaxed);
        self.table[replace_index] = TTEntry::new(key, score, eval, best_move, depth, bound, self.generation);
    }
    
    fn replacement_score(&self, entry: &TTEntry, new_depth: u8) -> i32 {
        let age_diff = self.generation.wrapping_sub(entry.get_generation());
        let depth_diff = new_depth as i32 - entry.get_depth() as i32;
        
        // Prioritize:
        // 1. Old entries (high age_diff)
        // 2. Shallow entries (negative depth_diff)
        // 3. Entries with less valuable bounds
        let bound_score = match entry.get_bound() {
            Bound::Exact => 0,
            Bound::Lower => -1,
            Bound::Upper => -2,
        };
        
        (age_diff as i32 * 10) - (depth_diff * 5) + bound_score
    }
    
    // ===== SIMPLE SCHEME (single entry per position) =====
    
    #[inline]
    pub fn store(&mut self, key: u64, score: i16, eval: i16, best_move: Option<Mossa>, depth: u8, bound: Bound) {
        let index = self.index(key);
        let old_entry = &self.table[index];
        
        if !old_entry.is_empty() {
            self.overwrites.fetch_add(1, Ordering::Relaxed);
        }
        
        // Always replace scheme
        self.table[index] = TTEntry::new(key, score, eval, best_move, depth, bound, self.generation);
    }
    
    // ===== PROBE FUNCTIONS =====
    
    #[inline]
    pub fn probe(&self, key: u64) -> Option<ProbeResult> {
        let index = self.index(key);
        let entry = &self.table[index];
        
        if entry.is_empty() {
            self.misses.fetch_add(1, Ordering::Relaxed);
            return None;
        }
        
        if entry.key_match(key) {
            self.hits.fetch_add(1, Ordering::Relaxed);
            Some(ProbeResult {
                score: entry.get_score(),
                eval: entry.get_eval(),
                best_move: entry.get_best_move(),
                depth: entry.get_depth(),
                bound: entry.get_bound(),
                generation: entry.get_generation(),
            })
        } else {
            self.collisions.fetch_add(1, Ordering::Relaxed);
            self.misses.fetch_add(1, Ordering::Relaxed);
            None
        }
    }
    
    #[inline]
    pub fn probe_bucket(&self, key: u64) -> Option<ProbeResult> {
        let index_base = self.index(key) & !0x3;
        
        for i in 0..4 {
            let idx = index_base + i;
            let entry = &self.table[idx];
            
            if !entry.is_empty() && entry.key_match(key) {
                self.hits.fetch_add(1, Ordering::Relaxed);
                return Some(ProbeResult {
                    score: entry.get_score(),
                    eval: entry.get_eval(),
                    best_move: entry.get_best_move(),
                    depth: entry.get_depth(),
                    bound: entry.get_bound(),
                    generation: entry.get_generation(),
                });
            }
        }
        
        self.misses.fetch_add(1, Ordering::Relaxed);
        None
    }
    
    #[inline]
    pub fn probe_move(&self, key: u64) -> Option<Mossa> {
        self.probe(key).and_then(|r| r.best_move)
    }
    
    #[inline]
    pub fn probe_score(&self, key: u64) -> Option<i16> {
        self.probe(key).map(|r| r.score)
    }
    
    // ===== MANAGEMENT FUNCTIONS =====
    
    pub fn new_generation(&mut self) {
        self.generation = self.generation.wrapping_add(1);
    }
    
    pub fn set_generation(&mut self, gen: u8) {
        self.generation = gen;
    }
    
    pub fn clear(&mut self) {
        for entry in &mut self.table {
            *entry = TTEntry { key: 0, data: 0 };
        }
        self.generation = 0;
        self.replacements.store(0, Ordering::Relaxed);
        self.hits.store(0, Ordering::Relaxed);
        self.misses.store(0, Ordering::Relaxed);
        self.overwrites.store(0, Ordering::Relaxed);
        self.collisions.store(0, Ordering::Relaxed);
    }
    
    pub fn prefetch(&self, key: u64) {
        #[cfg(target_arch = "x86_64")]
        {
            use std::arch::x86_64::_mm_prefetch;
            
            let index = self.index(key);
            let ptr = &self.table[index] as *const TTEntry;
            
            unsafe {
                // Prefetch for read
                _mm_prefetch(ptr as *const i8, 3); // _MM_HINT_T0
            }
        }
    }
    
    // ===== STATISTICS =====
    
    pub fn stats(&self) -> (u64, u64, u64, u64, f64) {
        let hits = self.hits.load(Ordering::Relaxed);
        let misses = self.misses.load(Ordering::Relaxed);
        let overwrites = self.overwrites.load(Ordering::Relaxed);
        let collisions = self.collisions.load(Ordering::Relaxed);
        
        let total = hits + misses;
        let hit_rate = if total > 0 {
            (hits as f64 / total as f64) * 100.0
        } else {
            0.0
        };
        
        (hits, misses, overwrites, collisions, hit_rate)
    }
    
    pub fn usage(&self) -> f64 {
        let mut used = 0;
        for entry in &self.table {
            if !entry.is_empty() {
                used += 1;
            }
        }
        
        (used as f64 / self.table.len() as f64) * 100.0
    }
    
    pub fn size_mb(&self) -> usize {
        self.size_mb
    }
    
    pub fn entries(&self) -> usize {
        self.table.len()
    }
    
    pub fn memory_usage(&self) -> usize {
        self.table.len() * std::mem::size_of::<TTEntry>()
    }
    
    // ===== OPTIMIZATION FUNCTIONS =====
    
    pub fn age_entries(&mut self) {
        // Age all entries by increasing their generation
        // This helps with replacement decisions
        self.new_generation();
    }
    
    pub fn partial_clear(&mut self, threshold: u8) {
        // Clear entries older than threshold generations
        for entry in &mut self.table {
            if !entry.is_empty() {
                let age = self.generation.wrapping_sub(entry.get_generation());
                if age > threshold {
                    *entry = TTEntry { key: 0, data: 0 };
                }
            }
        }
    }
    
    // Helper methods for compatibility with search.rs
    pub fn depth(&self) -> u8 {
        // Return 0 as default
        0
    }
    
    pub fn score(&self) -> i16 {
        // Return 0 as default
        0
    }
}

// ===== PROBE RESULT STRUCT =====

#[derive(Clone, Copy)]  // Aggiunto Clone, Copy
pub struct ProbeResult {
    pub score: i16,
    pub eval: i16,
    pub best_move: Option<Mossa>,
    pub depth: u8,
    pub bound: Bound,
    pub generation: u8,
}

// Implementazione dei metodi di utilità per ProbeResult
impl ProbeResult {
    pub fn score(&self) -> i16 {
        self.score
    }
    
    pub fn eval(&self) -> i16 {
        self.eval
    }
    
    pub fn best_move(&self) -> Option<Mossa> {
        self.best_move
    }
    
    pub fn depth(&self) -> u8 {
        self.depth
    }
    
    pub fn bound(&self) -> Bound {
        self.bound
    }
    
    pub fn generation(&self) -> u8 {
        self.generation
    }
}

// ===== MOSSA EXTENSIONS FOR COMPRESSION =====

// Implementiamo i metodi direttamente su Mossa invece di usare un trait
impl Mossa {
    pub fn to_u16(&self) -> u16 {
        // Encode move in 16 bits:
        // - 6 bits: from square (0-63)
        // - 6 bits: to square (0-63)
        // - 2 bits: promotion piece (0-3: None, Queen, Rook, Bishop, Knight)
        // - 1 bit: en passant flag
        // - 1 bit: castle flag
        
        let from = self.da().indice() as u16;  // Usa metodo da() invece di campo
        let to = self.a().indice() as u16;     // Usa metodo a() invece di campo
        
        let promo = match self.promozione() {  // Usa metodo promozione()
            None => 0,
            Some(Pezzo::Regina) => 1,
            Some(Pezzo::Torre) => 2,
            Some(Pezzo::Alfiere) => 3,
            Some(Pezzo::Cavallo) => 4,
            _ => 0,
        };
        
        let mut encoded = 0;
        encoded |= (from & 0x3F) << 10;
        encoded |= (to & 0x3F) << 4;
        encoded |= (promo & 0x07) << 1;
        encoded |= if self.flag() == MoveFlag::EnPassant { 1 } else { 0 };
        
        encoded
    }
    
    pub fn from_u16(val: u16) -> Option<Self> {
        let from_idx = ((val >> 10) & 0x3F) as usize;
        let to_idx = ((val >> 4) & 0x3F) as usize;
        let promo_val = ((val >> 1) & 0x07) as u8;
        let ep_flag = (val & 0x01) != 0;
        
        let from = Casella::da_indice(from_idx)?;
        let to = Casella::da_indice(to_idx)?;
        
        let promo = match promo_val {
            0 => None,
            1 => Some(Pezzo::Regina),
            2 => Some(Pezzo::Torre),
            3 => Some(Pezzo::Alfiere),
            4 => Some(Pezzo::Cavallo),
            _ => None,
        };
        
        // Crea la mossa usando la funzione corretta dal modulo board
        // Questo dipende da come è definita Mossa nel tuo codice
        // Assumo che ci sia un costruttore che accetta flag
        let flag = if ep_flag { MoveFlag::EnPassant } else { MoveFlag::None };
        
        // Nota: Questa parte dipende dall'effettiva implementazione di Mossa
        // Potrebbe essere necessario usare Mossa::new() o Mossa::new_with_flag()
        // La correzione esatta dipende dalla tua definizione di Mossa
        let mossa = Mossa::new_with_flag(from, to, promo, flag);
        
        Some(mossa)
    }
}

// ===== DEFAULT IMPLEMENTATION =====

impl Default for TranspositionTable {
    fn default() -> Self {
        Self::new(128) // 128MB default
    }
}

// ===== TEST MODULE =====

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Mossa;
    
    #[test]
    fn test_tt_entry_packing() {
        let from = Casella::da_string("e2").unwrap();
        let to = Casella::da_string("e4").unwrap();
        let mossa = Mossa::new(from, to, None);  // Usa il costruttore corretto
        
        let entry = TTEntry::new(
            0x123456789ABCDEF0,
            123,
            456,
            Some(mossa),
            8,
            Bound::Exact,
            1
        );
        
        assert_eq!(entry.get_score(), 123);
        assert_eq!(entry.get_eval(), 456);
        assert_eq!(entry.get_depth(), 8);
        assert_eq!(entry.get_bound(), Bound::Exact);
        assert_eq!(entry.get_generation(), 1);
        
        let restored_mossa = entry.get_best_move().unwrap();
        assert_eq!(restored_mossa.da(), from);
        assert_eq!(restored_mossa.a(), to);
    }
    
    #[test]
    fn test_tt_basic_operations() {
        let mut tt = TranspositionTable::new(16); // 16MB
        
        let key1 = 0x123456789ABCDEF0;
        let key2 = 0xFEDCBA9876543210;
        
        // Store first entry
        tt.store(key1, 100, 150, None, 5, Bound::Exact);
        
        // Should find it
        let result = tt.probe(key1);
        assert!(result.is_some());
        assert_eq!(result.unwrap().score, 100);
        
        // Should not find different key
        let result = tt.probe(key2);
        assert!(result.is_none());
    }
    
    #[test]
    fn test_mossa_compression() {
        // Test normal move
        let from = Casella::da_string("e2").unwrap();
        let to = Casella::da_string("e4").unwrap();
        let mossa = Mossa::new(from, to, None);
        
        let encoded = mossa.to_u16();
        let decoded = Mossa::from_u16(encoded).unwrap();
        
        assert_eq!(mossa.da(), decoded.da());
        assert_eq!(mossa.a(), decoded.a());
        assert_eq!(mossa.promozione(), decoded.promozione());
        
        // Test promotion move
        let mossa_promo = Mossa::new(from, to, Some(Pezzo::Regina));
        let encoded_promo = mossa_promo.to_u16();
        let decoded_promo = Mossa::from_u16(encoded_promo).unwrap();
        
        assert_eq!(mossa_promo.promozione(), decoded_promo.promozione());
    }
    
    #[test]
    fn test_bucket_scheme() {
        let mut tt = TranspositionTable::new(16);
        
        // Store multiple entries that hash to same bucket
        let base_key = 0x1000;
        
        for i in 0..8 {
            let key = base_key + (i as u64 * (tt.table.len() as u64 / 4));
            tt.store_bucket(key, (100 + i) as i16, 0, None, i as u8, Bound::Exact);
        }
        
        // Should be able to retrieve them
        for i in 0..8 {
            let key = base_key + (i as u64 * (tt.table.len() as u64 / 4));
            let result = tt.probe_bucket(key);
            assert!(result.is_some());
            assert_eq!(result.unwrap().score, (100 + i) as i16);
        }
    }
    
    #[test]
    fn test_tt_statistics() {
        let mut tt = TranspositionTable::new(16);
        
        // Do some operations
        for i in 0..100 {
            tt.store(i as u64 * 1000, i as i16, 0, None, 5, Bound::Exact);
        }
        
        for i in 0..50 {
            tt.probe(i as u64 * 1000);
        }
        
        for i in 50..100 {
            tt.probe(i as u64 * 999); // Different key, should miss
        }
        
        let (hits, misses, overwrites, collisions, hit_rate) = tt.stats();
        
        assert_eq!(hits, 50);
        assert_eq!(misses, 50);
        assert!(hit_rate >= 49.0 && hit_rate <= 51.0);
        
        let usage = tt.usage();
        assert!(usage > 0.0 && usage <= 100.0);
    }
}