use crate::board::{Bitboard, Colore};

// Tabelle pre-calcolate AUTOMATICAMENTE.
// Non serve più chiamare una funzione init() manuale!
lazy_static::lazy_static! {
    static ref PAWN_ATTACKS: [[Bitboard; 64]; 2] = init_pawn_attacks();
    static ref KNIGHT_ATTACKS: [Bitboard; 64] = init_knight_attacks();
    static ref KING_ATTACKS: [Bitboard; 64] = init_king_attacks();
}

// --- Funzioni Pubbliche di Accesso ---

#[inline(always)]
pub fn pawn_attacks(sq: usize, side: Colore) -> Bitboard {
    PAWN_ATTACKS[side.indice()][sq]
}

#[inline(always)]
pub fn knight_attacks(sq: usize) -> Bitboard {
    KNIGHT_ATTACKS[sq]
}

#[inline(always)]
pub fn king_attacks(sq: usize) -> Bitboard {
    KING_ATTACKS[sq]
}

// --- Slider Attacks (Calcolati al volo / Ray-Casting) ---
// Questo metodo è sicuro e facile da debuggare.

#[inline(always)]
pub fn bishop_attacks(sq: usize, occ: Bitboard) -> Bitboard {
    generate_slider_attacks(sq, occ, true)
}

#[inline(always)]
pub fn rook_attacks(sq: usize, occ: Bitboard) -> Bitboard {
    generate_slider_attacks(sq, occ, false)
}

#[inline(always)]
pub fn queen_attacks(sq: usize, occ: Bitboard) -> Bitboard {
    bishop_attacks(sq, occ) | rook_attacks(sq, occ)
}

fn generate_slider_attacks(sq: usize, occ: Bitboard, diagonal: bool) -> Bitboard {
    let mut attacks = 0;
    let r = (sq / 8) as i8;
    let f = (sq % 8) as i8;
    
    // Direzioni: (Delta Rank, Delta File)
    let dirs = if diagonal {
        [(1, 1), (1, -1), (-1, 1), (-1, -1)] // Alfiere: NE, NW, SE, SW
    } else {
        [(1, 0), (-1, 0), (0, 1), (0, -1)]   // Torre: N, S, E, W
    };

    for (dr, df) in dirs {
        for i in 1..8 { // Massimo 7 passi in ogni direzione
            let nr = r + dr * i;
            let nf = f + df * i;
            
            // Se usciamo dalla scacchiera, stop
            if nr < 0 || nr > 7 || nf < 0 || nf > 7 { break; }
            
            let bit = 1u64 << (nr * 8 + nf);
            attacks |= bit;
            
            // Se colpiamo un pezzo, ci fermiamo (inclusa la casa del pezzo colpito)
            if (occ & bit) != 0 { break; }
        }
    }
    attacks
}

// --- Logica di Inizializzazione (Chiamata da lazy_static) ---

fn init_pawn_attacks() -> [[Bitboard; 64]; 2] {
    let mut attacks = [[0; 64]; 2];
    for sq in 0..64 {
        // Bianco (Indice 0)
        attacks[0][sq] = mask_pawn_attacks_logic(sq, Colore::Bianco);
        // Nero (Indice 1)
        attacks[1][sq] = mask_pawn_attacks_logic(sq, Colore::Nero);
    }
    attacks
}

fn init_knight_attacks() -> [Bitboard; 64] {
    let mut attacks = [0; 64];
    for sq in 0..64 {
        attacks[sq] = mask_knight_attacks_logic(sq);
    }
    attacks
}

fn init_king_attacks() -> [Bitboard; 64] {
    let mut attacks = [0; 64];
    for sq in 0..64 {
        attacks[sq] = mask_king_attacks_logic(sq);
    }
    attacks
}

// --- Logica Pura di Generazione Maschere ---

fn mask_pawn_attacks_logic(sq: usize, side: Colore) -> Bitboard {
    let mut attacks: Bitboard = 0;
    let r = (sq / 8) as i8;
    let f = (sq % 8) as i8;

    // Bianco sale (+1), Nero scende (-1)
    let dr = if side == Colore::Bianco { 1 } else { -1 };
    
    // Catture in diagonale: File -1 e File +1
    for df in [-1, 1] {
        let nr = r + dr;
        let nf = f + df;
        
        if nr >= 0 && nr < 8 && nf >= 0 && nf < 8 {
            attacks |= 1u64 << (nr * 8 + nf);
        }
    }
    attacks
}

fn mask_knight_attacks_logic(sq: usize) -> Bitboard {
    let mut attacks: Bitboard = 0;
    let r = (sq / 8) as i8;
    let f = (sq % 8) as i8;
    
    let jumps = [
        (2, 1), (2, -1), (-2, 1), (-2, -1),
        (1, 2), (1, -2), (-1, 2), (-1, -2)
    ];

    for (dr, df) in jumps {
        let nr = r + dr;
        let nf = f + df;
        if nr >= 0 && nr < 8 && nf >= 0 && nf < 8 {
            attacks |= 1u64 << (nr * 8 + nf);
        }
    }
    attacks
}

fn mask_king_attacks_logic(sq: usize) -> Bitboard {
    let mut attacks: Bitboard = 0;
    let r = (sq / 8) as i8;
    let f = (sq % 8) as i8;

    for dr in -1..=1 {
        for df in -1..=1 {
            if dr == 0 && df == 0 { continue; }
            
            let nr = r + dr;
            let nf = f + df;
            
            if nr >= 0 && nr < 8 && nf >= 0 && nf < 8 {
                attacks |= 1u64 << (nr * 8 + nf);
            }
        }
    }
    attacks
}