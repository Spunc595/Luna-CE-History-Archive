use crate::board::{Bitboard, Colore};

// Tabelle pre-calcolate all'avvio del motore
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

// --- Slider Attacks (Alfieri, Torri, Regine) ---

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
    // La Regina è semplicemente l'unione dei raggi di Alfiere e Torre
    bishop_attacks(sq, occ) | rook_attacks(sq, occ)
}

/// Genera attacchi per pezzi a lungo raggio usando il Ray-Casting.
/// sq: posizione, occ: bitboard di occupazione totale, diagonal: true per Alfiere, false per Torre.
fn generate_slider_attacks(sq: usize, occ: Bitboard, diagonal: bool) -> Bitboard {
    let mut attacks = 0u64;
    let r = (sq / 8) as i8;
    let f = (sq % 8) as i8;
    
    let dirs: &[(i8, i8)] = if diagonal {
        &[(1, 1), (1, -1), (-1, 1), (-1, -1)]
    } else {
        &[(1, 0), (-1, 0), (0, 1), (0, -1)]
    };

    for &(dr, df) in dirs {
        let mut nr = r + dr;
        let mut nf = f + df;
        
        while nr >= 0 && nr < 8 && nf >= 0 && nf < 8 {
            let bit = 1u64 << (nr * 8 + nf);
            attacks |= bit;
            
            // Se incontriamo un pezzo (qualsiasi), il raggio si interrompe
            if (occ & bit) != 0 {
                break;
            }
            
            nr += dr;
            nf += df;
        }
    }
    attacks
}

// --- Inizializzazione Tabelle ---

fn init_pawn_attacks() -> [[Bitboard; 64]; 2] {
    let mut attacks = [[0; 64]; 2];
    for sq in 0..64 {
        attacks[0][sq] = mask_pawn_attacks_logic(sq, Colore::Bianco);
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

// --- Logica Maschere ---

fn mask_pawn_attacks_logic(sq: usize, side: Colore) -> Bitboard {
    let mut attacks = 0u64;
    let r = (sq / 8) as i8;
    let f = (sq % 8) as i8;
    let dr = if side == Colore::Bianco { 1 } else { -1 };
    
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
    let mut attacks = 0u64;
    let r = (sq / 8) as i8;
    let f = (sq % 8) as i8;
    let jumps = [(2,1), (2,-1), (-2,1), (-2,-1), (1,2), (1,-2), (-1,2), (-1,-2)];
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
    let mut attacks = 0u64;
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