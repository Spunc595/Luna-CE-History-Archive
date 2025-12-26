use crate::board::Bitboard;
use lazy_static::lazy_static;
use std::sync::RwLock;

// ============================================
// FUNZIONI DI CALCOLO PER LE TABELLE
// ============================================

fn compute_pawn_attacks() -> [[Bitboard; 64]; 2] {
    let mut table = [[0; 64]; 2];
    
    for square in 0..64 {
        let rank = square / 8;
        let file = square % 8;
        
        // Bianco (muove verso rank più alto)
        let mut white_attacks = 0;
        if file > 0 && rank < 7 {
            white_attacks |= 1 << (square + 7);
        }
        if file < 7 && rank < 7 {
            white_attacks |= 1 << (square + 9);
        }
        table[0][square] = white_attacks;
        
        // Nero (muove verso rank più basso)
        let mut black_attacks = 0;
        if file > 0 && rank > 0 {
            black_attacks |= 1 << (square - 9);
        }
        if file < 7 && rank > 0 {
            black_attacks |= 1 << (square - 7);
        }
        table[1][square] = black_attacks;
    }
    
    table
}

fn compute_knight_attacks() -> [Bitboard; 64] {
    let mut table = [0; 64];
    let offsets = [(-2, -1), (-2, 1), (-1, -2), (-1, 2),
                   (1, -2), (1, 2), (2, -1), (2, 1)];
    
    for square in 0..64 {
        let rank = (square / 8) as i32;
        let file = (square % 8) as i32;
        let mut attacks = 0;
        
        for &(dr, df) in &offsets {
            let new_rank = rank + dr;
            let new_file = file + df;
            
            if new_rank >= 0 && new_rank < 8 && new_file >= 0 && new_file < 8 {
                attacks |= 1 << (new_rank * 8 + new_file);
            }
        }
        table[square] = attacks;
    }
    
    table
}

fn compute_king_attacks() -> [Bitboard; 64] {
    let mut table = [0; 64];
    
    for square in 0..64 {
        let rank = (square / 8) as i32;
        let file = (square % 8) as i32;
        let mut attacks = 0;
        
        for dr in -1..=1 {
            for df in -1..=1 {
                if dr == 0 && df == 0 { continue; }
                
                let new_rank = rank + dr;
                let new_file = file + df;
                
                if new_rank >= 0 && new_rank < 8 && new_file >= 0 && new_file < 8 {
                    attacks |= 1 << (new_rank * 8 + new_file);
                }
            }
        }
        table[square] = attacks;
    }
    
    table
}

fn compute_bishop_masks() -> [Bitboard; 64] {
    let mut masks = [0; 64];
    
    for square in 0..64 {
        let rank = square / 8;
        let file = square % 8;
        let mut mask = 0;
        
        // Diagonale NE (rank+, file+)
        let mut r = rank + 1;
        let mut f = file + 1;
        while r < 7 && f < 7 {
            mask |= 1 << (r * 8 + f);
            r += 1;
            f += 1;
        }
        
        // Diagonale SE (rank-, file+)
        let mut r_i32 = rank as i32 - 1;
        let mut f_i32 = file as i32 + 1;
        while r_i32 > 0 && f_i32 < 7 {
            mask |= 1 << (r_i32 as usize * 8 + f_i32 as usize);
            r_i32 -= 1;
            f_i32 += 1;
        }
        
        // Diagonale SO (rank-, file-)
        r_i32 = rank as i32 - 1;
        f_i32 = file as i32 - 1;
        while r_i32 > 0 && f_i32 > 0 {
            mask |= 1 << (r_i32 as usize * 8 + f_i32 as usize);
            r_i32 -= 1;
            f_i32 -= 1;
        }
        
        // Diagonale NO (rank+, file-)
        r = rank + 1;
        f_i32 = file as i32 - 1;
        while r < 7 && f_i32 > 0 {
            mask |= 1 << (r * 8 + f_i32 as usize);
            r += 1;
            f_i32 -= 1;
        }
        
        masks[square] = mask;
    }
    
    masks
}

fn compute_bishop_shifts() -> [usize; 64] {
    let bishop_masks = compute_bishop_masks();
    let mut shifts = [0; 64];
    
    for square in 0..64 {
        let bits = bishop_masks[square].count_ones() as usize;
        shifts[square] = 64 - bits;
    }
    
    shifts
}

fn compute_rook_masks() -> [Bitboard; 64] {
    let mut masks = [0; 64];
    
    for square in 0..64 {
        let rank = square / 8;
        let file = square % 8;
        let mut mask = 0;
        
        // Nord (rank+)
        for r in (rank + 1)..7 {
            mask |= 1 << (r * 8 + file);
        }
        
        // Sud (rank-)
        for r in (1..rank).rev() {
            mask |= 1 << (r * 8 + file);
        }
        
        // Est (file+)
        for f in (file + 1)..7 {
            mask |= 1 << (rank * 8 + f);
        }
        
        // Ovest (file-)
        for f in (1..file).rev() {
            mask |= 1 << (rank * 8 + f);
        }
        
        masks[square] = mask;
    }
    
    masks
}

fn compute_rook_shifts() -> [usize; 64] {
    let rook_masks = compute_rook_masks();
    let mut shifts = [0; 64];
    
    for square in 0..64 {
        let bits = rook_masks[square].count_ones() as usize;
        shifts[square] = 64 - bits;
    }
    
    shifts
}

// Numeri magici per alfiere (precalcolati)
const BISHOP_MAGICS: [u64; 64] = [
    0x89a1121896040240, 0x2004844802002010, 0x2068080051921000, 0x62880a0220200808,
    0x4042004000000000, 0x1008220202000110, 0xc00444222012000a, 0x28808801200001,
    0x4004920884081000, 0x201c401040c0084, 0x840800910a0010, 0x82080240060,
    0x2000840504006000, 0x30010c4108405004, 0x1008005410080802, 0x8144042209100900,
    0x208081020014400, 0x4800201208ca00, 0xf18140408012008, 0x1004002802102001,
    0x841000820080811, 0x40200200a42008, 0x800054042000, 0x88010400410c9000,
    0x520040470104290, 0x1004040051500081, 0x2002081833080021, 0x400c00c010142,
    0x941408200c002000, 0x658810000806011, 0x188071040440a00, 0x4800404002011c00,
    0x104442040404200, 0x511080202091021, 0x4022401120400, 0x80c0040400080120,
    0x8040010040820802, 0x480810700020090, 0x102008e00040242, 0x8090052000500100,
    0x8002024220104080, 0x431008804142000, 0x19001802081400, 0x200014208040080,
    0x3308082008200100, 0x41010500040c020, 0x4012020c04210308, 0x208220a202004080,
    0x111040120082000, 0x6803040141280a00, 0x2101004202410000, 0x8200000041108022,
    0x21082088000, 0x2410204010040, 0x40100400809000, 0x822088220820214,
    0x40808090012004, 0x910224040218c9, 0x402814422015008, 0x90014004842410,
    0x1000042304105, 0x10008830412a00, 0x2520081090008908, 0x40102000a0a60140,
];

// Numeri magici per torre
const ROOK_MAGICS: [u64; 64] = [
    0x0080001020400080, 0x0040001000200040, 0x0080081000200080, 0x0080040800100080,
    0x0080020400080080, 0x0080010200040080, 0x0080008001000200, 0x0080002040800100,
    0x0000800020400080, 0x0000400020005000, 0x0000801000200080, 0x0000800800100080,
    0x0000800400080080, 0x0000800200040080, 0x0000800100020080, 0x0000800040800100,
    0x0000208000400080, 0x0000404000201000, 0x0000808010002000, 0x0000808008001000,
    0x0000808004000800, 0x0000808002000400, 0x0000010100020004, 0x0000020000408104,
    0x0000208080004000, 0x0000200040005000, 0x0000100080200080, 0x0000080080100080,
    0x0000040080080080, 0x0000020080040080, 0x0000010080800200, 0x0000800080004100,
    0x0000204000800080, 0x0000200040401000, 0x0000100020802000, 0x0000080020801000,
    0x0000040020800800, 0x0000020020800400, 0x0000010020800200, 0x0000004081020004,
    0x0000204000808000, 0x0000200040008080, 0x0000100020008080, 0x0000080010008080,
    0x0000040008008080, 0x0000020004008080, 0x0000010002008080, 0x0000004081020004,
    0x0000204000800080, 0x0000200040008080, 0x0000100020008080, 0x0000080010008080,
    0x0000040008008080, 0x0000020004008080, 0x0000010002008080, 0x0000004081020004,
    0x0000208000400080, 0x0000204000201000, 0x0000102000100800, 0x0000081000080400,
    0x0000040800080200, 0x0000020400080100, 0x0000010200080080, 0x0000004081020004,
];

// ============================================
// TABELLE INIZIALIZZATE CON lazy_static
// ============================================

lazy_static! {
    pub static ref PAWN_ATTACKS: [[Bitboard; 64]; 2] = compute_pawn_attacks();
    pub static ref KNIGHT_ATTACKS: [Bitboard; 64] = compute_knight_attacks();
    pub static ref KING_ATTACKS: [Bitboard; 64] = compute_king_attacks();
    pub static ref BISHOP_MASKS: [Bitboard; 64] = compute_bishop_masks();
    pub static ref BISHOP_SHIFTS: [usize; 64] = compute_bishop_shifts();
    pub static ref ROOK_MASKS: [Bitboard; 64] = compute_rook_masks();
    pub static ref ROOK_SHIFTS: [usize; 64] = compute_rook_shifts();
    
    // Usiamo RwLock per mutabilità thread-safe invece di RefCell
    static ref BISHOP_ATTACKS_INTERNAL: RwLock<Vec<Vec<Bitboard>>> = {
        let bishop_masks = compute_bishop_masks();
        let mut attacks = Vec::with_capacity(64);
        
        for square in 0..64 {
            let mask = bishop_masks[square];
            let bits = mask.count_ones() as usize;
            let size = 1 << bits;
            attacks.push(vec![0; size]);
        }
        
        RwLock::new(attacks)
    };
    
    static ref ROOK_ATTACKS_INTERNAL: RwLock<Vec<Vec<Bitboard>>> = {
        let rook_masks = compute_rook_masks();
        let mut attacks = Vec::with_capacity(64);
        
        for square in 0..64 {
            let mask = rook_masks[square];
            let bits = mask.count_ones() as usize;
            let size = 1 << bits;
            attacks.push(vec![0; size]);
        }
        
        RwLock::new(attacks)
    };
}

// Funzioni di accesso sicure alle tabelle
fn bishop_attacks_table() -> &'static RwLock<Vec<Vec<Bitboard>>> {
    &BISHOP_ATTACKS_INTERNAL
}

fn rook_attacks_table() -> &'static RwLock<Vec<Vec<Bitboard>>> {
    &ROOK_ATTACKS_INTERNAL
}

// ============================================
// FUNZIONI DI INIZIALIZZAZIONE
// ============================================

pub fn init_slider_attacks() {
    init_bishop_attacks();
    init_rook_attacks();
}

fn init_bishop_attacks() {
    let bishop_masks = compute_bishop_masks();
    let table = bishop_attacks_table();
    
    for square in 0..64 {
        let mask = bishop_masks[square];
        let bits = mask.count_ones() as usize;
        let size = 1 << bits;
        
        for index in 0..size {
            let occupancy = index_to_occupancy(index, bits, mask);
            let attacks_bb = bishop_attacks_on_the_fly(square, occupancy);
            
            // Calcola l'indice usando magic
            let magic_index = ((occupancy.wrapping_mul(BISHOP_MAGICS[square])) >> BISHOP_SHIFTS[square]) as usize;
            
            // Modifica la tabella tramite RwLock
            let mut table_write = table.write().unwrap();
            table_write[square][magic_index] = attacks_bb;
        }
    }
}

fn init_rook_attacks() {
    let rook_masks = compute_rook_masks();
    let table = rook_attacks_table();
    
    for square in 0..64 {
        let mask = rook_masks[square];
        let bits = mask.count_ones() as usize;
        let size = 1 << bits;
        
        for index in 0..size {
            let occupancy = index_to_occupancy(index, bits, mask);
            let attacks_bb = rook_attacks_on_the_fly(square, occupancy);
            
            let magic_index = ((occupancy.wrapping_mul(ROOK_MAGICS[square])) >> ROOK_SHIFTS[square]) as usize;
            
            // Modifica la tabella tramite RwLock
            let mut table_write = table.write().unwrap();
            table_write[square][magic_index] = attacks_bb;
        }
    }
}

// ============================================
// FUNZIONI AUSILIARIE
// ============================================

// Converte un indice in una configurazione di occupancy
fn index_to_occupancy(index: usize, bits: usize, mut mask: Bitboard) -> Bitboard {
    let mut occupancy = 0;
    
    for i in 0..bits {
        let square = mask.trailing_zeros() as usize;
        mask &= mask - 1;  // Rimuovi il bit più basso
        
        if (index & (1 << i)) != 0 {
            occupancy |= 1 << square;
        }
    }
    
    occupancy
}

// Calcola attacchi dell'alfiere in tempo reale (per inizializzazione)
fn bishop_attacks_on_the_fly(square: usize, block: Bitboard) -> Bitboard {
    let mut attacks = 0;
    let rank = (square / 8) as i32;
    let file = (square % 8) as i32;
    
    // NE
    let mut r = rank + 1;
    let mut f = file + 1;
    while r < 8 && f < 8 {
        let idx = (r * 8 + f) as usize;
        attacks |= 1 << idx;
        if (block >> idx) & 1 != 0 { break; }
        r += 1;
        f += 1;
    }
    
    // SE
    r = rank - 1;
    f = file + 1;
    while r >= 0 && f < 8 {
        let idx = (r * 8 + f) as usize;
        attacks |= 1 << idx;
        if (block >> idx) & 1 != 0 { break; }
        r -= 1;
        f += 1;
    }
    
    // SO
    r = rank - 1;
    f = file - 1;
    while r >= 0 && f >= 0 {
        let idx = (r * 8 + f) as usize;
        attacks |= 1 << idx;
        if (block >> idx) & 1 != 0 { break; }
        r -= 1;
        f -= 1;
    }
    
    // NO
    r = rank + 1;
    f = file - 1;
    while r < 8 && f >= 0 {
        let idx = (r * 8 + f) as usize;
        attacks |= 1 << idx;
        if (block >> idx) & 1 != 0 { break; }
        r += 1;
        f -= 1;
    }
    
    attacks
}

// Calcola attacchi della torre in tempo reale
fn rook_attacks_on_the_fly(square: usize, block: Bitboard) -> Bitboard {
    let mut attacks = 0;
    let rank = (square / 8) as i32;
    let file = (square % 8) as i32;
    
    // Nord
    let mut r = rank + 1;
    while r < 8 {
        let idx = (r * 8 + file) as usize;
        attacks |= 1 << idx;
        if (block >> idx) & 1 != 0 { break; }
        r += 1;
    }
    
    // Sud
    r = rank - 1;
    while r >= 0 {
        let idx = (r * 8 + file) as usize;
        attacks |= 1 << idx;
        if (block >> idx) & 1 != 0 { break; }
        r -= 1;
    }
    
    // Est
    let mut f = file + 1;
    while f < 8 {
        let idx = (rank * 8 + f) as usize;
        attacks |= 1 << idx;
        if (block >> idx) & 1 != 0 { break; }
        f += 1;
    }
    
    // Ovest
    f = file - 1;
    while f >= 0 {
        let idx = (rank * 8 + f) as usize;
        attacks |= 1 << idx;
        if (block >> idx) & 1 != 0 { break; }
        f -= 1;
    }
    
    attacks
}

// ============================================
// FUNZIONI PUBBLICHE DI ACCESSO
// ============================================

// Ottieni attacchi dell'alfiere usando magic bitboards
pub fn get_bishop_attacks(square: usize, occupancy: Bitboard) -> Bitboard {
    let mask = BISHOP_MASKS[square];
    let blocker = occupancy & mask;
    let magic = BISHOP_MAGICS[square];
    let shift = BISHOP_SHIFTS[square];
    
    let index = ((blocker.wrapping_mul(magic)) >> shift) as usize;
    
    // Accesso sicuro alla tabella tramite RwLock
    let table = bishop_attacks_table().read().unwrap();
    table[square][index]
}

// Ottieni attacchi della torre usando magic bitboards
pub fn get_rook_attacks(square: usize, occupancy: Bitboard) -> Bitboard {
    let mask = ROOK_MASKS[square];
    let blocker = occupancy & mask;
    let magic = ROOK_MAGICS[square];
    let shift = ROOK_SHIFTS[square];
    
    let index = ((blocker.wrapping_mul(magic)) >> shift) as usize;
    
    // Accesso sicuro alla tabella tramite RwLock
    let table = rook_attacks_table().read().unwrap();
    table[square][index]
}

// Ottieni attacchi della regina (alfiere + torre)
pub fn get_queen_attacks(square: usize, occupancy: Bitboard) -> Bitboard {
    get_bishop_attacks(square, occupancy) | get_rook_attacks(square, occupancy)
}

// ============================================
// RE-EXPORT PER COMPATIBILITÀ
// ============================================

pub fn pawn_attacks(square: usize, color: crate::board::Colore) -> Bitboard {
    PAWN_ATTACKS[color.indice()][square]
}

pub fn knight_attacks(square: usize) -> Bitboard {
    KNIGHT_ATTACKS[square]
}

pub fn king_attacks(square: usize) -> Bitboard {
    KING_ATTACKS[square]
}

pub fn bishop_attacks(square: usize, occupancy: Bitboard) -> Bitboard {
    get_bishop_attacks(square, occupancy)
}

pub fn rook_attacks(square: usize, occupancy: Bitboard) -> Bitboard {
    get_rook_attacks(square, occupancy)
}

pub fn queen_attacks(square: usize, occupancy: Bitboard) -> Bitboard {
    get_queen_attacks(square, occupancy)
}