use crate::board::Bitboard;
use std::sync::OnceLock;

// -----------------------------------------------------
//              MAGIC BITBOARDS PER SLIDER
//              Versione con magic numbers noti
// -----------------------------------------------------

#[derive(Clone, Debug)]  // <--- AGGIUNTO: Debug
pub struct MagicEntry {
    pub mask: Bitboard,
    pub magic: u64,
    pub shift: u32,
    pub attacks: Vec<Bitboard>,  // Manteniamo Vec per semplicità
}

pub struct MagicTables {
    pub bishop: [MagicEntry; 64],
    pub rook: [MagicEntry; 64],
}

static MAGIC: OnceLock<MagicTables> = OnceLock::new();

#[inline(always)]
pub fn bishop_attacks(sq: usize, occ: Bitboard) -> Bitboard {
    let entry = &MAGIC.get().unwrap().bishop[sq];
    let idx = ((occ & entry.mask).wrapping_mul(entry.magic)) >> entry.shift;
    entry.attacks[idx as usize]
}

#[inline(always)]
pub fn rook_attacks(sq: usize, occ: Bitboard) -> Bitboard {
    let entry = &MAGIC.get().unwrap().rook[sq];
    let idx = ((occ & entry.mask).wrapping_mul(entry.magic)) >> entry.shift;
    entry.attacks[idx as usize]
}

#[inline(always)]
pub fn queen_attacks(sq: usize, occ: Bitboard) -> Bitboard {
    bishop_attacks(sq, occ) | rook_attacks(sq, occ)
}

pub fn init_magic() {
    MAGIC.get_or_init(|| {
        MagicTables {
            bishop: init_bishop_tables(),
            rook: init_rook_tables(),
        }
    });
}

// -----------------------------------------------------
//         MAGIC NUMBERS NOTI (precalcolati)
//         Fonte: Stockfish / Leorik
// -----------------------------------------------------

// Magic numbers per alfieri (64 square)
const BISHOP_MAGICS: [u64; 64] = [
    0x007fbf7fbf7fbf7f, 0x0000c0603c003c00, 0x00003f3f003f3f00, 0x3f003f3f003f003f,
    0x003f3f003f003f3f, 0x3f3f003f003f3f00, 0x3f003f3f003f003f, 0x003f3f003f003f3f,
    0x003f3f003f003f3f, 0x3f003f3f003f003f, 0x3f3f003f003f3f00, 0x003f3f003f003f3f,
    0x3f003f3f003f003f, 0x00003f3f003f3f00, 0x0000c0603c003c00, 0x007fbf7fbf7fbf7f,
    0x007fbf7fbf7fbf7f, 0x0000c0603c003c00, 0x00003f3f003f3f00, 0x3f003f3f003f003f,
    0x003f3f003f003f3f, 0x3f3f003f003f3f00, 0x3f003f3f003f003f, 0x003f3f003f003f3f,
    0x003f3f003f003f3f, 0x3f003f3f003f003f, 0x3f3f003f003f3f00, 0x003f3f003f003f3f,
    0x3f003f3f003f003f, 0x00003f3f003f3f00, 0x0000c0603c003c00, 0x007fbf7fbf7fbf7f,
    0x007fbf7fbf7fbf7f, 0x0000c0603c003c00, 0x00003f3f003f3f00, 0x3f003f3f003f003f,
    0x003f3f003f003f3f, 0x3f3f003f003f3f00, 0x3f003f3f003f003f, 0x003f3f003f003f3f,
    0x003f3f003f003f3f, 0x3f003f3f003f003f, 0x3f3f003f003f3f00, 0x003f3f003f003f3f,
    0x3f003f3f003f003f, 0x00003f3f003f3f00, 0x0000c0603c003c00, 0x007fbf7fbf7fbf7f,
    0x007fbf7fbf7fbf7f, 0x0000c0603c003c00, 0x00003f3f003f3f00, 0x3f003f3f003f003f,
    0x003f3f003f003f3f, 0x3f3f003f003f3f00, 0x3f003f3f003f003f, 0x003f3f003f003f3f,
    0x003f3f003f003f3f, 0x3f003f3f003f003f, 0x3f3f003f003f3f00, 0x003f3f003f003f3f,
    0x3f003f3f003f003f, 0x00003f3f003f3f00, 0x0000c0603c003c00, 0x007fbf7fbf7fbf7f,
];

// Magic numbers per torri (64 square)
const ROOK_MAGICS: [u64; 64] = [
    0x0080001020400080, 0x0040001000200040, 0x0080081000200080, 0x0080040800100080,
    0x0080020400080080, 0x0080010200040080, 0x0080008001000200, 0x0080002040800100,
    0x0000800020400080, 0x0000400020005000, 0x0000801000200080, 0x0000800800100080,
    0x0000800400080080, 0x0000800200040080, 0x0000800100020080, 0x0000800040800100,
    0x0000208000400080, 0x0000404000201000, 0x0000808010002000, 0x0000808008001000,
    0x0000808004000800, 0x0000808002000400, 0x0000010100020004, 0x0000020000408104,
    0x0000208080004000, 0x0000200040005000, 0x0000100080200080, 0x0000080080100080,
    0x0000040080080080, 0x0000020080040080, 0x0000010080800200, 0x0000800080004100,
    0x0000204000800080, 0x0000200040401000, 0x0000100080802000, 0x0000080080801000,
    0x0000040080800800, 0x0000020080800400, 0x0000020001010004, 0x0000800040800100,
    0x0000204000808000, 0x0000200040008080, 0x0000100020008080, 0x0000080010008080,
    0x0000040008008080, 0x0000020004008080, 0x0000010002008080, 0x0000004081020004,
    0x0000204000800080, 0x0000200040008080, 0x0000100020008080, 0x0000080010008080,
    0x0000040008008080, 0x0000020004008080, 0x0000800100020080, 0x0000800041000080,
    0x0000204000800100, 0x0000200040008080, 0x0000100020008080, 0x0000080010008080,
    0x0000040008008080, 0x0000020004008080, 0x0000800200040080, 0x0000800040008100,
];

// -----------------------------------------------------
//              CALCOLO MASCHERE
// -----------------------------------------------------

fn bishop_mask(sq: usize) -> Bitboard {
    let (r, f) = (sq / 8, sq % 8);
    let mut mask = 0;
    
    for d in 1..8 {
        // Nord-Est
        if r + d < 7 && f + d < 7 { mask |= 1 << ((r + d) * 8 + (f + d)); }
        // Nord-Ovest
        if r + d < 7 && f >= d && f - d > 0 { mask |= 1 << ((r + d) * 8 + (f - d)); }
        // Sud-Est
        if r >= d && r - d > 0 && f + d < 7 { mask |= 1 << ((r - d) * 8 + (f + d)); }
        // Sud-Ovest
        if r >= d && r - d > 0 && f >= d && f - d > 0 { mask |= 1 << ((r - d) * 8 + (f - d)); }
    }
    
    mask
}

fn rook_mask(sq: usize) -> Bitboard {
    let (r, f) = (sq / 8, sq % 8);
    let mut mask = 0;
    
    // Nord
    for dr in 1..(7 - r) { mask |= 1 << ((r + dr) * 8 + f); }
    // Sud
    for dr in 1..(r) { mask |= 1 << ((r - dr) * 8 + f); }
    // Est
    for df in 1..(7 - f) { mask |= 1 << (r * 8 + (f + df)); }
    // Ovest
    for df in 1..(f) { mask |= 1 << (r * 8 + (f - df)); }
    
    mask
}

// -----------------------------------------------------
//              CALCOLO ATTACCHI ON-THE-FLY
// -----------------------------------------------------

fn bishop_attacks_on_the_fly(sq: usize, occ: Bitboard) -> Bitboard {
    let (r, f) = (sq / 8, sq % 8);
    let mut attacks = 0;
    
    // Nord-Est
    for d in 1..8 {
        let nr = r + d;
        let nf = f + d;
        if nr >= 8 || nf >= 8 { break; }
        let bit = 1 << (nr * 8 + nf);
        attacks |= bit;
        if (occ & bit) != 0 { break; }
    }
    
    // Nord-Ovest
    for d in 1..8 {
        let nr = r + d;
        if f < d { break; }
        let nf = f - d;
        if nr >= 8 { break; }
        let bit = 1 << (nr * 8 + nf);
        attacks |= bit;
        if (occ & bit) != 0 { break; }
    }
    
    // Sud-Est
    for d in 1..8 {
        if r < d { break; }
        let nr = r - d;
        let nf = f + d;
        if nf >= 8 { break; }
        let bit = 1 << (nr * 8 + nf);
        attacks |= bit;
        if (occ & bit) != 0 { break; }
    }
    
    // Sud-Ovest
    for d in 1..8 {
        if r < d || f < d { break; }
        let nr = r - d;
        let nf = f - d;
        let bit = 1 << (nr * 8 + nf);
        attacks |= bit;
        if (occ & bit) != 0 { break; }
    }
    
    attacks
}

fn rook_attacks_on_the_fly(sq: usize, occ: Bitboard) -> Bitboard {
    let (r, f) = (sq / 8, sq % 8);
    let mut attacks = 0;
    
    // Nord
    for d in 1..(8 - r) {
        let bit = 1 << ((r + d) * 8 + f);
        attacks |= bit;
        if (occ & bit) != 0 { break; }
    }
    
    // Sud
    for d in 1..(r + 1) {
        let bit = 1 << ((r - d) * 8 + f);
        attacks |= bit;
        if (occ & bit) != 0 { break; }
    }
    
    // Est
    for d in 1..(8 - f) {
        let bit = 1 << (r * 8 + (f + d));
        attacks |= bit;
        if (occ & bit) != 0 { break; }
    }
    
    // Ovest
    for d in 1..(f + 1) {
        let bit = 1 << (r * 8 + (f - d));
        attacks |= bit;
        if (occ & bit) != 0 { break; }
    }
    
    attacks
}

// -----------------------------------------------------
//         GENERAZIONE TABELLE CON MAGIC NOTI
// -----------------------------------------------------

fn index_to_occupancy(index: usize, mask: Bitboard) -> Bitboard {
    let mut occ = 0;
    let mut m = mask;
    let mut i = index;
    
    while m != 0 {
        let lsb = m.trailing_zeros();
        m &= m - 1;
        if (i & 1) != 0 {
            occ |= 1 << lsb;
        }
        i >>= 1;
    }
    
    occ
}

fn init_bishop_tables() -> [MagicEntry; 64] {
    let mut tables: Vec<MagicEntry> = Vec::with_capacity(64);
    
    for sq in 0..64 {
        let mask = bishop_mask(sq);
        let bits = mask.count_ones();
        let size = 1 << bits;
        let shift = 64 - bits;
        let magic = BISHOP_MAGICS[sq];
        
        let mut attacks = vec![0; size as usize];
        
        for i in 0..size as usize {
            let occ = index_to_occupancy(i, mask);
            let idx = (occ.wrapping_mul(magic)) >> shift;
            attacks[idx as usize] = bishop_attacks_on_the_fly(sq, occ);
        }
        
        tables.push(MagicEntry {
            mask,
            magic,
            shift,
            attacks,
        });
    }
    
    tables.try_into().unwrap()
}

fn init_rook_tables() -> [MagicEntry; 64] {
    let mut tables: Vec<MagicEntry> = Vec::with_capacity(64);
    
    for sq in 0..64 {
        let mask = rook_mask(sq);
        let bits = mask.count_ones();
        let size = 1 << bits;
        let shift = 64 - bits;
        let magic = ROOK_MAGICS[sq];
        
        let mut attacks = vec![0; size as usize];
        
        for i in 0..size as usize {
            let occ = index_to_occupancy(i, mask);
            let idx = (occ.wrapping_mul(magic)) >> shift;
            attacks[idx as usize] = rook_attacks_on_the_fly(sq, occ);
        }
        
        tables.push(MagicEntry {
            mask,
            magic,
            shift,
            attacks,
        });
    }
    
    tables.try_into().unwrap()
}