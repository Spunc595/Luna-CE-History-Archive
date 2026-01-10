use crate::board::{Bitboard, Colore};

// Tabelle pre-calcolate per pezzi a salto (non-sliding)
// Inizializzate staticamente per evitare overhead di runtime
pub struct AttackTables {
    pub pawn_attacks: [[Bitboard; 64]; 2],
    pub knight_attacks: [Bitboard; 64],
    pub king_attacks: [Bitboard; 64],
}

// Inizializzazione globale pigra (disponibile da Rust 1.70+)
use std::sync::OnceLock;
static TABLES: OnceLock<AttackTables> = OnceLock::new();

fn get_tables() -> &'static AttackTables {
    TABLES.get_or_init(|| AttackTables {
        pawn_attacks: init_pawn_attacks(),
        knight_attacks: init_knight_attacks(),
        king_attacks: init_king_attacks(),
    })
}

// --- Funzioni Pubbliche di Accesso ---

#[inline(always)]
pub fn pawn_attacks(sq: usize, side: Colore) -> Bitboard {
    get_tables().pawn_attacks[side.indice()][sq]
}

#[inline(always)]
pub fn knight_attacks(sq: usize) -> Bitboard {
    get_tables().knight_attacks[sq]
}

#[inline(always)]
pub fn king_attacks(sq: usize) -> Bitboard {
    get_tables().king_attacks[sq]
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
    bishop_attacks(sq, occ) | rook_attacks(sq, occ)
}

/// Genera attacchi per pezzi a lungo raggio usando il Ray-Casting.
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
            
            // Se incontriamo un pezzo, il raggio si interrompe
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
        let r = (sq / 8) as i8;
        let f = (sq % 8) as i8;

        // Bianco (attacca verso l'alto: rank + 1)
        if r < 7 {
            if f > 0 { attacks[0][sq] |= 1u64 << (sq + 7); }
            if f < 7 { attacks[0][sq] |= 1u64 << (sq + 9); }
        }
        // Nero (attacca verso il basso: rank - 1)
        if r > 0 {
            if f > 0 { attacks[1][sq] |= 1u64 << (sq - 9); }
            if f < 7 { attacks[1][sq] |= 1u64 << (sq - 7); }
        }
    }
    attacks
}

fn init_knight_attacks() -> [Bitboard; 64] {
    let mut attacks = [0; 64];
    for sq in 0..64 {
        let r = (sq / 8) as i8;
        let f = (sq % 8) as i8;
        let jumps = [(2,1), (2,-1), (-2,1), (-2,-1), (1,2), (1,-2), (-1,2), (-1,-2)];
        for (dr, df) in jumps {
            let nr = r + dr;
            let nf = f + df;
            if nr >= 0 && nr < 8 && nf >= 0 && nf < 8 {
                attacks[sq] |= 1u64 << (nr * 8 + nf);
            }
        }
    }
    attacks
}

fn init_king_attacks() -> [Bitboard; 64] {
    let mut attacks = [0; 64];
    for sq in 0..64 {
        let r = (sq / 8) as i8;
        let f = (sq % 8) as i8;
        for dr in -1..=1 {
            for df in -1..=1 {
                if dr == 0 && df == 0 { continue; }
                let nr = r + dr;
                let nf = f + df;
                if nr >= 0 && nr < 8 && nf >= 0 && nf < 8 {
                    attacks[sq] |= 1u64 << (nr * 8 + nf);
                }
            }
        }
    }
    attacks
}