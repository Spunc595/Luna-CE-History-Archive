use crate::board::{Bitboard, Colore};
use lazy_static::lazy_static;

lazy_static! {
    pub static ref ATTACKS: AttackTables = AttackTables::new();
}

pub struct AttackTables {
    pub pawn_attacks: [[Bitboard; 64]; 2],
    pub knight_attacks: [Bitboard; 64],
    pub king_attacks: [Bitboard; 64],
}

impl AttackTables {
    fn new() -> Self {
        let mut pawn = [[0; 64]; 2];
        let mut knight = [0; 64];
        let mut king = [0; 64];

        for sq in 0..64 {
            let b = 1u64 << sq;
            let file = sq % 8;

            // --- Attacchi Pedoni ---
            // Bianco (Sposta verso l'alto: +7, +9)
            if file > 0 { pawn[0][sq] |= b << 7; }
            if file < 7 { pawn[0][sq] |= b << 9; }
            // Nero (Sposta verso il basso: -7, -9)
            if file > 0 { pawn[1][sq] |= b >> 9; }
            if file < 7 { pawn[1][sq] |= b >> 7; }

            // --- Attacchi Cavalli ---
            if file > 1 { knight[sq] |= (b >> 10) | (b << 6); }
            if file > 0 { knight[sq] |= (b >> 17) | (b << 15); }
            if file < 7 { knight[sq] |= (b >> 15) | (b << 17); }
            if file < 6 { knight[sq] |= (b >> 6) | (b << 10); }

            // --- Attacchi Re ---
            let mut k = (b << 8) | (b >> 8);
            if file > 0 { k |= (b >> 1) | (b >> 9) | (b << 7); }
            if file < 7 { k |= (b << 1) | (b << 9) | (b >> 7); }
            king[sq] = k;
        }

        AttackTables {
            pawn_attacks: pawn,
            knight_attacks: knight,
            king_attacks: king,
        }
    }
}

// ============================================
// FUNZIONI DI ACCESSO PUBBLICHE
// ============================================

#[inline(always)]
pub fn pawn_attacks(sq: usize, col: Colore) -> Bitboard {
    ATTACKS.pawn_attacks[col as usize][sq]
}

#[inline(always)]
pub fn knight_attacks(sq: usize) -> Bitboard {
    ATTACKS.knight_attacks[sq]
}

#[inline(always)]
pub fn king_attacks(sq: usize) -> Bitboard {
    ATTACKS.king_attacks[sq]
}

// --- Slider Libero (On-the-fly) ---

pub fn bishop_attacks(sq: usize, occ: Bitboard) -> Bitboard {
    genera_sliding_attacks(sq, occ, &[(1, 1), (1, -1), (-1, 1), (-1, -1)])
}

pub fn rook_attacks(sq: usize, occ: Bitboard) -> Bitboard {
    genera_sliding_attacks(sq, occ, &[(1, 0), (-1, 0), (0, 1), (0, -1)])
}

pub fn queen_attacks(sq: usize, occ: Bitboard) -> Bitboard {
    bishop_attacks(sq, occ) | rook_attacks(sq, occ)
}

// ============================================
// LOGICA DI GENERAZIONE RAGGI
// ============================================

fn genera_sliding_attacks(sq: usize, occ: Bitboard, dirs: &[(i32, i32)]) -> Bitboard {
    let mut attacks = 0u64;
    let r = (sq / 8) as i32;
    let f = (sq % 8) as i32;

    for &(dr, df) in dirs {
        let mut cur_r = r + dr;
        let mut cur_f = f + df;
        
        while cur_r >= 0 && cur_r < 8 && cur_f >= 0 && cur_f < 8 {
            let bit = 1u64 << (cur_r * 8 + cur_f);
            attacks |= bit;
            // Se incontriamo un pezzo (occupazione), il raggio si ferma
            if (occ & bit) != 0 {
                break;
            }
            cur_r += dr;
            cur_f += df;
        }
    }
    attacks
}