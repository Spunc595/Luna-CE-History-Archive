use crate::board::Bitboard;

// Maschere per gli attacchi di torre e alfiere (senza bordi)
pub static RANK_MASK: [u64; 8] = [
    0xFF,                    // Rank 1
    0xFF00,                  // Rank 2
    0xFF0000,               // Rank 3
    0xFF000000,             // Rank 4
    0xFF00000000,           // Rank 5
    0xFF0000000000,         // Rank 6
    0xFF000000000000,       // Rank 7
    0xFF00000000000000,     // Rank 8
];

pub static FILE_MASK: [u64; 8] = [
    0x0101010101010101,     // File a
    0x0202020202020202,     // File b
    0x0404040404040404,     // File c
    0x0808080808080808,     // File d
    0x1010101010101010,     // File e
    0x2020202020202020,     // File f
    0x4040404040404040,     // File g
    0x8080808080808080,     // File h
];

// Tabelle per attacchi di pedone
pub static PAWN_ATTACKS: [[u64; 64]; 2] = {
    let mut attacks = [[0; 64]; 2];
    
    let mut square = 0;
    while square < 64 {
        let mut white_attacks = 0u64;
        let mut black_attacks = 0u64;
        
        let file = square % 8;
        let rank = square / 8;
        
        // Attacchi pedone bianco
        if rank < 7 {
            if file > 0 {
                white_attacks |= 1u64 << (square + 7);
            }
            if file < 7 {
                white_attacks |= 1u64 << (square + 9);
            }
        }
        
        // Attacchi pedone nero
        if rank > 0 {
            if file > 0 {
                black_attacks |= 1u64 << (square - 9);
            }
            if file < 7 {
                black_attacks |= 1u64 << (square - 7);
            }
        }
        
        attacks[0][square] = white_attacks; // Bianco
        attacks[1][square] = black_attacks; // Nero
        
        square += 1;
    }
    
    attacks
};

// Tabelle per attacchi di cavallo
pub static KNIGHT_ATTACKS: [u64; 64] = {
    let mut attacks = [0; 64];
    
    let knight_moves = [
        (2, 1), (2, -1), (-2, 1), (-2, -1),
        (1, 2), (1, -2), (-1, 2), (-1, -2)
    ];
    
    for square in 0..64 {
        let file = (square % 8) as i32;
        let rank = (square / 8) as i32;
        let mut attack_mask = 0u64;
        
        for &(df, dr) in &knight_moves {
            let new_file = file + df;
            let new_rank = rank + dr;
            
            if new_file >= 0 && new_file < 8 && new_rank >= 0 && new_rank < 8 {
                let target_square = (new_rank * 8 + new_file) as u64;
                attack_mask |= 1u64 << target_square;
            }
        }
        
        attacks[square] = attack_mask;
    }
    
    attacks
};

// Tabelle per attacchi di re
pub static KING_ATTACKS: [u64; 64] = {
    let mut attacks = [0; 64];
    
    let king_moves = [
        (1, 0), (1, 1), (0, 1), (-1, 1),
        (-1, 0), (-1, -1), (0, -1), (1, -1)
    ];
    
    for square in 0..64 {
        let file = (square % 8) as i32;
        let rank = (square / 8) as i32;
        let mut attack_mask = 0u64;
        
        for &(df, dr) in &king_moves {
            let new_file = file + df;
            let new_rank = rank + dr;
            
            if new_file >= 0 && new_file < 8 && new_rank >= 0 && new_rank < 8 {
                let target_square = (new_rank * 8 + new_file) as u64;
                attack_mask |= 1u64 << target_square;
            }
        }
        
        attacks[square] = attack_mask;
    }
    
    attacks
};

// Magic numbers per rook attacks (pre-calcolati per ogni casella)
pub static ROOK_MAGICS: [u64; 64] = [
    0x8a80104000800020, 0x140002000100040, 0x2801880a0017001, 0x100081001000420,
    0x200020010080420, 0x3001c0002010008, 0x8480008002000100, 0x2080088004402900,
    0x800080204000, 0x80144000200004, 0x100808020004000, 0xa08018014000880,
    0x8000808004000200, 0x20100080001100, 0x8000200060805100, 0x40008010008001,
    0x1a00060008210044, 0x4008010008020, 0x80004000200080, 0x208002008008080,
    0x80400080200800, 0x4200041002080, 0x880002008008041, 0x4040008040800020,
    0x80002000400080, 0x110100a020010080, 0x800040008080, 0x10100890a0014008,
    0x100040080020080, 0x220201008010040, 0x200010008080, 0x241000401808040,
    0x280080080100040, 0x900040004028010, 0x4800100020080, 0x410001000802001,
    0x280008020004010, 0x1200080042001, 0x200004008002080, 0x804000200801002,
    0x208000200100801, 0x400080002080, 0x2400040000800801, 0x200004000802080,
    0x180003002004008, 0x8400008004002002, 0x40001000208004, 0x100020260020904,
    0x2008004000800801, 0x20400080200080, 0x8000400800400, 0x4002001000802004,
    0x1200080200040100, 0x20000400080201, 0x20000400080201, 0x20000400080201,
    0x8000200010008080, 0x8800040008008080, 0x4400020002008, 0x40010002010004,
    0x100800100020080, 0x408040000801, 0x400080002080, 0x100020020004010,
];

pub static BISHOP_MAGICS: [u64; 64] = [
    0x40040844404084, 0x2004208a004208, 0x10190041080202, 0x108060845042010,
    0x581104180800210, 0x2112080446200010, 0x1080820820060210, 0x3c0808410220200,
    0x4050404440404, 0x21001420088, 0x24d0080801082102, 0x1020a0a020400,
    0x40308200402, 0x4011002100800, 0x401484104104005, 0x801010402020200,
    0x400210c3880100, 0x404022024108200, 0x810018200204102, 0x4002801a02003,
    0x85040820080400, 0x810102c808880400, 0xe900410884800, 0x8002020480840102,
    0x220200865090201, 0x2010100a02021202, 0x152048408022401, 0x20080002081110,
    0x4001001021004000, 0x800040400a011002, 0xe4004081011002, 0x1c004001012080,
    0x8004200962a00220, 0x8422100208500202, 0x2000402200300c08, 0x8646020080080080,
    0x80020a0200100808, 0x2010004880111000, 0x623000a080011400, 0x42008c0340209202,
    0x209188240001000, 0x400408a884001800, 0x110400a6080400, 0x1840060a44020800,
    0x90080104000041, 0x201011000808101, 0x1a2208080504f080, 0x8012020600211212,
    0x500861011240000, 0x180806108200800, 0x4000020e01040044, 0x300000261044000a,
    0x802241102020002, 0x20906061210001, 0x5a84841004010310, 0x4010801011c04,
    0xa010109502200, 0x4a02012000, 0x500201010098b028, 0x8040002811040900,
    0x28000010020204, 0x6000020202d0240, 0x8918844842082200, 0x4010011029020020,
];

// Shift values per magic bitboards
pub static ROOK_SHIFTS: [u8; 64] = [
    52, 53, 53, 53, 53, 53, 53, 52,
    53, 54, 54, 54, 54, 54, 54, 53,
    53, 54, 54, 54, 54, 54, 54, 53,
    53, 54, 54, 54, 54, 54, 54, 53,
    53, 54, 54, 54, 54, 54, 54, 53,
    53, 54, 54, 54, 54, 54, 54, 53,
    53, 54, 54, 54, 54, 54, 54, 53,
    52, 53, 53, 53, 53, 53, 53, 52,
];

pub static BISHOP_SHIFTS: [u8; 64] = [
    58, 59, 59, 59, 59, 59, 59, 58,
    59, 59, 59, 59, 59, 59, 59, 59,
    59, 59, 57, 57, 57, 57, 59, 59,
    59, 59, 57, 55, 55, 57, 59, 59,
    59, 59, 57, 55, 55, 57, 59, 59,
    59, 59, 57, 57, 57, 57, 59, 59,
    59, 59, 59, 59, 59, 59, 59, 59,
    58, 59, 59, 59, 59, 59, 59, 58,
];

// Maschere di occupazione (senza bordi)
pub static ROOK_MASKS: [u64; 64] = compute_rook_masks();
pub static BISHOP_MASKS: [u64; 64] = compute_bishop_masks();

fn compute_rook_masks() -> [u64; 64] {
    let mut masks = [0; 64];
    
    for square in 0..64 {
        let file = square % 8;
        let rank = square / 8;
        let mut mask = 0u64;
        
        // Est verso il bordo (esclusa la casella stessa)
        for r in (rank + 1)..7 {
            mask |= 1u64 << (r * 8 + file);
        }
        
        // Ovest verso il bordo
        for r in (1..rank).rev() {
            mask |= 1u64 << (r * 8 + file);
        }
        
        // Nord verso il bordo
        for f in (file + 1)..7 {
            mask |= 1u64 << (rank * 8 + f);
        }
        
        // Sud verso il bordo
        for f in (1..file).rev() {
            mask |= 1u64 << (rank * 8 + f);
        }
        
        masks[square] = mask;
    }
    
    masks
}

fn compute_bishop_masks() -> [u64; 64] {
    let mut masks = [0; 64];
    
    for square in 0..64 {
        let file = square % 8;
        let rank = square / 8;
        let mut mask = 0u64;
        
        // Diagonale nord-est
        let mut r = rank + 1;
        let mut f = file + 1;
        while r < 7 && f < 7 {
            mask |= 1u64 << (r * 8 + f);
            r += 1;
            f += 1;
        }
        
        // Diagonale nord-ovest
        let mut r = rank + 1;
        let mut f = file as i32 - 1;
        while r < 7 && f > 0 {
            mask |= 1u64 << (r * 8 + f as usize);
            r += 1;
            f -= 1;
        }
        
        // Diagonale sud-est
        let mut r = rank as i32 - 1;
        let mut f = file + 1;
        while r > 0 && f < 7 {
            mask |= 1u64 << (r as usize * 8 + f);
            r -= 1;
            f += 1;
        }
        
        // Diagonale sud-ovest
        let mut r = rank as i32 - 1;
        let mut f = file as i32 - 1;
        while r > 0 && f > 0 {
            mask |= 1u64 << (r as usize * 8 + f as usize);
            r -= 1;
            f -= 1;
        }
        
        masks[square] = mask;
    }
    
    masks
}

// Tabelle di attacco per torre e alfiere
static mut ROOK_ATTACKS: Vec<Vec<u64>> = Vec::new();
static mut BISHOP_ATTACKS: Vec<Vec<u64>> = Vec::new();

// Inizializzazione dei magic bitboards
pub fn init_magics() {
    unsafe {
        init_rook_attacks();
        init_bishop_attacks();
    }
}

unsafe fn init_rook_attacks() {
    let mut attacks = Vec::with_capacity(64);
    
    for square in 0..64 {
        let mask = ROOK_MASKS[square];
        let shift = ROOK_SHIFTS[square];
        let magic = ROOK_MAGICS[square];
        
        // Numero di bit impostati nella maschera
        let bits = mask.count_ones() as usize;
        let size = 1 << bits;
        
        let mut table = vec![0; size];
        
        // Genera tutte le possibili occupazioni
        let mut subset = 0u64;
        loop {
            // Calcola l'indice usando il magic number
            let index = ((subset & mask).wrapping_mul(magic) >> shift) as usize;
            
            // Calcola gli attacchi per questa occupazione
            let attacks_bb = rook_attacks_slow(square as u8, subset);
            table[index] = attacks_bb;
            
            // Prossima occupazione
            subset = (subset.wrapping_sub(mask)) & mask;
            if subset == 0 {
                break;
            }
        }
        
        attacks.push(table);
    }
    
    ROOK_ATTACKS = attacks;
}

unsafe fn init_bishop_attacks() {
    let mut attacks = Vec::with_capacity(64);
    
    for square in 0..64 {
        let mask = BISHOP_MASKS[square];
        let shift = BISHOP_SHIFTS[square];
        let magic = BISHOP_MAGICS[square];
        
        let bits = mask.count_ones() as usize;
        let size = 1 << bits;
        
        let mut table = vec![0; size];
        
        let mut subset = 0u64;
        loop {
            let index = ((subset & mask).wrapping_mul(magic) >> shift) as usize;
            let attacks_bb = bishop_attacks_slow(square as u8, subset);
            table[index] = attacks_bb;
            
            subset = (subset.wrapping_sub(mask)) & mask;
            if subset == 0 {
                break;
            }
        }
        
        attacks.push(table);
    }
    
    BISHOP_ATTACKS = attacks;
}

// Funzioni lente per calcolare attacchi (usate solo per inizializzazione)
fn rook_attacks_slow(square: u8, blockers: u64) -> u64 {
    let file = (square % 8) as i32;
    let rank = (square / 8) as i32;
    let mut attacks = 0u64;
    
    // Est
    for r in (rank + 1)..8 {
        let s = (r * 8 + file) as u64;
        attacks |= 1u64 << s;
        if (blockers >> s) & 1 != 0 {
            break;
        }
    }
    
    // Ovest
    for r in (0..rank).rev() {
        let s = (r * 8 + file) as u64;
        attacks |= 1u64 << s;
        if (blockers >> s) & 1 != 0 {
            break;
        }
    }
    
    // Nord
    for f in (file + 1)..8 {
        let s = (rank * 8 + f) as u64;
        attacks |= 1u64 << s;
        if (blockers >> s) & 1 != 0 {
            break;
        }
    }
    
    // Sud
    for f in (0..file).rev() {
        let s = (rank * 8 + f) as u64;
        attacks |= 1u64 << s;
        if (blockers >> s) & 1 != 0 {
            break;
        }
    }
    
    attacks
}

fn bishop_attacks_slow(square: u8, blockers: u64) -> u64 {
    let file = (square % 8) as i32;
    let rank = (square / 8) as i32;
    let mut attacks = 0u64;
    
    // Nord-est
    let mut r = rank + 1;
    let mut f = file + 1;
    while r < 8 && f < 8 {
        let s = (r * 8 + f) as u64;
        attacks |= 1u64 << s;
        if (blockers >> s) & 1 != 0 {
            break;
        }
        r += 1;
        f += 1;
    }
    
    // Nord-ovest
    let mut r = rank + 1;
    let mut f = file - 1;
    while r < 8 && f >= 0 {
        let s = (r * 8 + f) as u64;
        attacks |= 1u64 << s;
        if (blockers >> s) & 1 != 0 {
            break;
        }
        r += 1;
        f -= 1;
    }
    
    // Sud-est
    let mut r = rank - 1;
    let mut f = file + 1;
    while r >= 0 && f < 8 {
        let s = (r * 8 + f) as u64;
        attacks |= 1u64 << s;
        if (blockers >> s) & 1 != 0 {
            break;
        }
        r -= 1;
        f += 1;
    }
    
    // Sud-ovest
    let mut r = rank - 1;
    let mut f = file - 1;
    while r >= 0 && f >= 0 {
        let s = (r * 8 + f) as u64;
        attacks |= 1u64 << s;
        if (blockers >> s) & 1 != 0 {
            break;
        }
        r -= 1;
        f -= 1;
    }
    
    attacks
}

// Funzioni pubbliche per ottenere attacchi con magic bitboards
pub fn rook_attacks(square: usize, occupancy: u64) -> u64 {
    unsafe {
        if ROOK_ATTACKS.is_empty() {
            // Fallback a calcolo lento se non inizializzato
            return rook_attacks_slow(square as u8, occupancy);
        }
        
        let mask = ROOK_MASKS[square];
        let magic = ROOK_MAGICS[square];
        let shift = ROOK_SHIFTS[square];
        
        // Usa solo i bit rilevanti della maschera
        let blockers = occupancy & mask;
        let index = (blockers.wrapping_mul(magic) >> shift) as usize;
        
        ROOK_ATTACKS[square][index]
    }
}

pub fn bishop_attacks(square: usize, occupancy: u64) -> u64 {
    unsafe {
        if BISHOP_ATTACKS.is_empty() {
            return bishop_attacks_slow(square as u8, occupancy);
        }
        
        let mask = BISHOP_MASKS[square];
        let magic = BISHOP_MAGICS[square];
        let shift = BISHOP_SHIFTS[square];
        
        let blockers = occupancy & mask;
        let index = (blockers.wrapping_mul(magic) >> shift) as usize;
        
        BISHOP_ATTACKS[square][index]
    }
}

pub fn queen_attacks(square: usize, occupancy: u64) -> u64 {
    let rook = rook_attacks(square, occupancy);
    let bishop = bishop_attacks(square, occupancy);
    rook | bishop
}

// Tabella per le direzioni (utile per SEE e altri algoritmi)
pub static BETWEEN_SQUARES: [[u64; 64]; 64] = compute_between_squares();
pub static LINE_SQUARES: [[u64; 64]; 64] = compute_line_squares();

fn compute_between_squares() -> [[u64; 64]; 64] {
    let mut between = [[0; 64]; 64];
    
    for a in 0..64 {
        for b in 0..64 {
            let mut mask = 0u64;
            
            if a != b {
                let a_file = a % 8;
                let a_rank = a / 8;
                let b_file = b % 8;
                let b_rank = b / 8;
                
                // Stessa riga
                if a_rank == b_rank {
                    let start = a_file.min(b_file) + 1;
                    let end = a_file.max(b_file);
                    for f in start..end {
                        mask |= 1u64 << (a_rank * 8 + f);
                    }
                }
                // Stessa colonna
                else if a_file == b_file {
                    let start = a_rank.min(b_rank) + 1;
                    let end = a_rank.max(b_rank);
                    for r in start..end {
                        mask |= 1u64 << (r * 8 + a_file);
                    }
                }
                // Stessa diagonale
                else if (a_file as i32 - b_file as i32).abs() == (a_rank as i32 - b_rank as i32).abs() {
                    let file_step = if b_file > a_file { 1 } else { -1 };
                    let rank_step = if b_rank > a_rank { 1 } else { -1 };
                    
                    let mut f = a_file as i32 + file_step;
                    let mut r = a_rank as i32 + rank_step;
                    
                    while f != b_file as i32 && r != b_rank as i32 {
                        mask |= 1u64 << (r as usize * 8 + f as usize);
                        f += file_step;
                        r += rank_step;
                    }
                }
            }
            
            between[a][b] = mask;
        }
    }
    
    between
}

fn compute_line_squares() -> [[u64; 64]; 64] {
    let mut line = [[0; 64]; 64];
    
    for a in 0..64 {
        for b in 0..64 {
            let mut mask = 0u64;
            
            let a_file = a % 8;
            let a_rank = a / 8;
            let b_file = b % 8;
            let b_rank = b / 8;
            
            if a_rank == b_rank || a_file == b_file || 
               (a_file as i32 - b_file as i32).abs() == (a_rank as i32 - b_rank as i32).abs() {
                // Aggiungi tutte le caselle sulla stessa linea
                let mut f = a_file as i32;
                let mut r = a_rank as i32;
                
                let file_step = if b_file > a_file { 1 } else if b_file < a_file { -1 } else { 0 };
                let rank_step = if b_rank > a_rank { 1 } else if b_rank < a_rank { -1 } else { 0 };
                
                while f >= 0 && f < 8 && r >= 0 && r < 8 {
                    mask |= 1u64 << (r as usize * 8 + f as usize);
                    f += file_step;
                    r += rank_step;
                    
                    if f == b_file as i32 && r == b_rank as i32 {
                        mask |= 1u64 << (r as usize * 8 + f as usize);
                        break;
                    }
                }
            }
            
            line[a][b] = mask;
        }
    }
    
    line
}

fn compute_pawn_passed_masks() -> [[u64; 64]; 2] {
    let mut masks = [[0; 64]; 2];
    
    for square in 0..64 {
        let file = square % 8;
        let rank = square / 8;
        
        // Maschera per pedone bianco (avanza verso rank 8)
        let mut white_mask = 0u64;
        if rank < 7 {
            let start_rank = rank + 1;
            for r in start_rank..8 {
                for df in -1..=1 {
                    let f = file as i32 + df;
                    if f >= 0 && f < 8 {
                        white_mask |= 1u64 << (r * 8 + f as usize);
                    }
                }
            }
        }
        
        // Maschera per pedone nero (avanza verso rank 1)
        let mut black_mask = 0u64;
        if rank > 0 {
            for r in 0..rank {
                for df in -1..=1 {
                    let f = file as i32 + df;
                    if f >= 0 && f < 8 {
                        black_mask |= 1u64 << (r * 8 + f as usize);
                    }
                }
            }
        }
        
        masks[0][square] = white_mask;  // Bianco
        masks[1][square] = black_mask;  // Nero
    }
    
    masks
}

fn compute_pawn_isolated_masks() -> [u64; 64] {
    let mut masks = [0; 64];
    
    for square in 0..64 {
        let file = square % 8;
        let mut mask = 0u64;
        
        // File adiacenti
        if file > 0 {
            mask |= FILE_MASK[file - 1];
        }
        if file < 7 {
            mask |= FILE_MASK[file + 1];
        }
        
        masks[square] = mask;
    }
    
    masks
}

// Struttura globale per accedere alle tabelle
pub struct Tables {
    pub attacchi_pedone: [[u64; 64]; 2],
    pub attacchi_cavallo: [u64; 64],
    pub attacchi_re: [u64; 64],
    pub attacchi_alfiere: Vec<Vec<u64>>,
    pub attacchi_torre: Vec<Vec<u64>>,
    pub between_squares: [[u64; 64]; 64],
    pub line_squares: [[u64; 64]; 64],
    pub pawn_passed_masks: [[u64; 64]; 2],
    pub pawn_isolated_masks: [u64; 64],
}

impl Tables {
    pub fn globale() -> &'static Tables {
        static mut TABLES: Option<Tables> = None;
        static INIT: std::sync::Once = std::sync::Once::new();
        
        unsafe {
            INIT.call_once(|| {
                // Inizializza magic bitboards
                init_magics();
                
                TABLES = Some(Tables {
                    attacchi_pedone: PAWN_ATTACKS,
                    attacchi_cavallo: KNIGHT_ATTACKS,
                    attacchi_re: KING_ATTACKS,
                    attacchi_alfiere: BISHOP_ATTACKS.clone(),
                    attacchi_torre: ROOK_ATTACKS.clone(),
                    between_squares: BETWEEN_SQUARES,
                    line_squares: LINE_SQUARES,
                    pawn_passed_masks: compute_pawn_passed_masks(),
                    pawn_isolated_masks: compute_pawn_isolated_masks(),
                });
            });
            
            TABLES.as_ref().unwrap()
        }
    }
    
    // Funzioni helper per compatibilità
    pub fn attacchi_alfiere(&self, square: usize, occupancy: u64) -> u64 {
        bishop_attacks(square, occupancy)
    }
    
    pub fn attacchi_torre(&self, square: usize, occupancy: u64) -> u64 {
        rook_attacks(square, occupancy)
    }
    
    pub fn attacchi_regina(&self, square: usize, occupancy: u64) -> u64 {
        queen_attacks(square, occupancy)
    }
    
    // Alias per compatibilità con le chiamate nel codice
    pub fn bishop_attacks(&self, square: usize, occupancy: u64) -> u64 {
        bishop_attacks(square, occupancy)
    }
    
    pub fn rook_attacks(&self, square: usize, occupancy: u64) -> u64 {
        rook_attacks(square, occupancy)
    }
    
    pub fn queen_attacks(&self, square: usize, occupancy: u64) -> u64 {
        queen_attacks(square, occupancy)
    }
    
    // Nuove funzioni utili per evaluation
    pub fn squares_between(&self, a: usize, b: usize) -> u64 {
        self.between_squares[a][b]
    }
    
    pub fn squares_on_line(&self, a: usize, b: usize) -> u64 {
        self.line_squares[a][b]
    }
    
    pub fn is_aligned(&self, a: usize, b: usize, c: usize) -> bool {
        (self.line_squares[a][b] >> c) & 1 != 0
    }
}

// Funzioni per il calcolo veloce di attacchi (fallback se magics non inizializzati)
pub fn sliding_attacks(square: usize, occupied: u64, directions: &[(i32, i32)]) -> u64 {
    let file = square % 8;
    let rank = square / 8;
    let mut attacks = 0u64;
    
    for &(df, dr) in directions {
        let mut f = file as i32;
        let mut r = rank as i32;
        
        loop {
            f += df;
            r += dr;
            
            if f < 0 || f >= 8 || r < 0 || r >= 8 {
                break;
            }
            
            let s = r as usize * 8 + f as usize;
            attacks |= 1u64 << s;
            
            if (occupied >> s) & 1 != 0 {
                break;
            }
        }
    }
    
    attacks
}

// Inizializzazione al caricamento del modulo
// Rimuoviamo il ctor poiché potrebbe causare problemi
pub fn initialize() {
    init_magics();
}

// Test module
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_knight_attacks() {
        // Casella centrale e4 (square 28)
        let e4_attacks = KNIGHT_ATTACKS[28];
        assert_eq!(e4_attacks.count_ones(), 8);
        
        // Casella angolo a1 (square 0)
        let a1_attacks = KNIGHT_ATTACKS[0];
        assert_eq!(a1_attacks.count_ones(), 2);
    }
    
    #[test]
    fn test_king_attacks() {
        // Re in e4 (square 28)
        let e4_attacks = KING_ATTACKS[28];
        assert_eq!(e4_attacks.count_ones(), 8);
        
        // Re in a1 (square 0)
        let a1_attacks = KING_ATTACKS[0];
        assert_eq!(a1_attacks.count_ones(), 3);
    }
    
    #[test]
    fn test_pawn_attacks() {
        // Pedone bianco in e2 (square 12)
        let white_e2 = PAWN_ATTACKS[0][12];
        assert_eq!(white_e2.count_ones(), 2);
        
        // Pedone nero in e7 (square 52)
        let black_e7 = PAWN_ATTACKS[1][52];
        assert_eq!(black_e7.count_ones(), 2);
    }
    
    #[test]
    fn test_magic_init() {
        // Verifica che le tabelle siano inizializzate
        let tables = Tables::globale();
        
        // Test attacco torre in a1 con nessun blocco
        let a1_empty = rook_attacks(0, 0);
        assert!(a1_empty > 0);
        
        // Test attacco alfiere in e4 con nessun blocco
        let e4_empty = bishop_attacks(28, 0);
        assert!(e4_empty > 0);
    }
}