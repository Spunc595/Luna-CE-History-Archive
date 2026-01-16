use crate::board::{Bitboard, Colore};
use std::sync::OnceLock;

// --- STRUTTURA TABELLE PER PEZZI NON SCORREVOLI ---
pub struct AttackTables {
    pub pawn_attacks: [[Bitboard; 64]; 2],
    pub knight_attacks: [Bitboard; 64],
    pub king_attacks: [Bitboard; 64],
}

static TABLES: OnceLock<AttackTables> = OnceLock::new();

#[inline(always)]
fn get_tables() -> &'static AttackTables {
    TABLES.get_or_init(|| AttackTables {
        pawn_attacks: init_pawn_attacks(),
        knight_attacks: init_knight_attacks(),
        king_attacks: init_king_attacks(),
    })
}

// --- API PUBBLICHE ---

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

// Alfieri, Torri e Regine usano la generazione on-the-fly ottimizzata
#[inline(always)]
pub fn bishop_attacks(sq: usize, occ: Bitboard) -> Bitboard {
    let mut attacks = 0u64;
    attacks |= ray_attacks(sq, occ, 1, 1);
    attacks |= ray_attacks(sq, occ, 1, -1);
    attacks |= ray_attacks(sq, occ, -1, 1);
    attacks |= ray_attacks(sq, occ, -1, -1);
    attacks
}

#[inline(always)]
pub fn rook_attacks(sq: usize, occ: Bitboard) -> Bitboard {
    let mut attacks = 0u64;
    attacks |= ray_attacks(sq, occ, 1, 0);
    attacks |= ray_attacks(sq, occ, -1, 0);
    attacks |= ray_attacks(sq, occ, 0, 1);
    attacks |= ray_attacks(sq, occ, 0, -1);
    attacks
}

#[inline(always)]
pub fn queen_attacks(sq: usize, occ: Bitboard) -> Bitboard {
    bishop_attacks(sq, occ) | rook_attacks(sq, occ)
}

/// Genera un raggio di attacco in una direzione specifica.
/// Più veloce della versione precedente perché evita iterazioni inutili.
#[inline(always)]
fn ray_attacks(sq: usize, occ: Bitboard, dr: i8, df: i8) -> Bitboard {
    let mut attacks = 0u64;
    let r = (sq / 8) as i8;
    let f = (sq % 8) as i8;
    
    let mut nr = r + dr;
    let mut nf = f + df;
    
    while nr >= 0 && nr < 8 && nf >= 0 && nf < 8 {
        let bit = 1u64 << (nr * 8 + nf);
        attacks |= bit;
        if (occ & bit) != 0 { break; }
        nr += dr;
        nf += df;
    }
    attacks
}

// --- INIZIALIZZATORI (Eseguiti una sola volta all'avvio) ---

fn init_pawn_attacks() -> [[Bitboard; 64]; 2] {
    let mut attacks = [[0; 64]; 2];
    for sq in 0..64 {
        let bit = 1u64 << sq;
        let r = sq / 8;
        let f = sq % 8;

        // BIANCO (Indice 0)
        if r < 7 {
            if f > 0 { attacks[0][sq] |= bit << 7; } // Nord-Ovest
            if f < 7 { attacks[0][sq] |= bit << 9; } // Nord-Est
        }

        // NERO (Indice 1)
        if r > 0 {
            if f < 7 { attacks[1][sq] |= bit >> 7; } // Sud-Est
            if f > 0 { attacks[1][sq] |= bit >> 9; } // Sud-Ovest
        }
    }
    attacks
}

fn init_knight_attacks() -> [Bitboard; 64] {
    let mut attacks = [0; 64];
    for sq in 0..64 {
        let r = (sq / 8) as i8;
        let f = (sq % 8) as i8;
        let jumps = [(2,1),(2,-1),(-2,1),(-2,-1),(1,2),(1,-2),(-1,2),(-1,-2)];
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