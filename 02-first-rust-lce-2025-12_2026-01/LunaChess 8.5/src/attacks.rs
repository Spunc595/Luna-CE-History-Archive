use crate::board::{Bitboard, Colore};
use lazy_static::lazy_static;

lazy_static! {
    static ref PAWN_ATTACKS: [[Bitboard; 64]; 2] = compute_pawn_attacks_table();
    static ref KNIGHT_ATTACKS: [Bitboard; 64] = compute_knight_attacks_table();
    static ref KING_ATTACKS: [Bitboard; 64] = compute_king_attacks_table();
}

#[inline(always)]
pub fn pawn_attacks(square: usize, color: Colore) -> Bitboard {
    PAWN_ATTACKS[color.indice()][square]
}

#[inline(always)]
pub fn knight_attacks(square: usize) -> Bitboard {
    KNIGHT_ATTACKS[square]
}

#[inline(always)]
pub fn king_attacks(square: usize) -> Bitboard {
    KING_ATTACKS[square]
}

#[inline(always)]
pub fn bishop_attacks(square: usize, occupancy: Bitboard) -> Bitboard {
    let mut attacks = 0;
    let rank = (square / 8) as i32;
    let file = (square % 8) as i32;
    for (dr, df) in [(1, 1), (1, -1), (-1, 1), (-1, -1)] {
        let mut r = rank + dr;
        let mut f = file + df;
        while r >= 0 && r < 8 && f >= 0 && f < 8 {
            let bit = 1u64 << (r * 8 + f);
            attacks |= bit;
            if (occupancy & bit) != 0 { break; }
            r += dr;
            f += df;
        }
    }
    attacks
}

#[inline(always)]
pub fn rook_attacks(square: usize, occupancy: Bitboard) -> Bitboard {
    let mut attacks = 0;
    let rank = (square / 8) as i32;
    let file = (square % 8) as i32;
    for (dr, df) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
        let mut r = rank + dr;
        let mut f = file + df;
        while r >= 0 && r < 8 && f >= 0 && f < 8 {
            let bit = 1u64 << (r * 8 + f);
            attacks |= bit;
            if (occupancy & bit) != 0 { break; }
            r += dr;
            f += df;
        }
    }
    attacks
}

#[inline(always)]
pub fn queen_attacks(square: usize, occupancy: Bitboard) -> Bitboard {
    bishop_attacks(square, occupancy) | rook_attacks(square, occupancy)
}

#[inline(always)] pub fn get_bishop_attacks(s: usize, o: Bitboard) -> Bitboard { bishop_attacks(s, o) }
#[inline(always)] pub fn get_rook_attacks(s: usize, o: Bitboard) -> Bitboard { rook_attacks(s, o) }
#[inline(always)] pub fn get_queen_attacks(s: usize, o: Bitboard) -> Bitboard { queen_attacks(s, o) }

fn compute_pawn_attacks_table() -> [[Bitboard; 64]; 2] {
    let mut table = [[0; 64]; 2];
    for sq in 0..64 {
        let file = sq % 8;
        let rank = sq / 8;
        if rank < 7 {
            if file > 0 { table[0][sq] |= 1u64 << (sq + 7); }
            if file < 7 { table[0][sq] |= 1u64 << (sq + 9); }
        }
        if rank > 0 {
            if file > 0 { table[1][sq] |= 1u64 << (sq - 9); }
            if file < 7 { table[1][sq] |= 1u64 << (sq - 7); }
        }
    }
    table
}

fn compute_knight_attacks_table() -> [Bitboard; 64] {
    let mut table = [0; 64];
    for sq in 0..64 {
        let rank = (sq / 8) as i32;
        let file = (sq % 8) as i32;
        for (dr, df) in [(-2, -1), (-2, 1), (-1, -2), (-1, 2), (1, -2), (1, 2), (2, -1), (2, 1)] {
            let nr = rank + dr;
            let nf = file + df;
            if nr >= 0 && nr < 8 && nf >= 0 && nf < 8 {
                table[sq] |= 1u64 << (nr * 8 + nf);
            }
        }
    }
    table
}

fn compute_king_attacks_table() -> [Bitboard; 64] {
    let mut table = [0; 64];
    for sq in 0..64 {
        let rank = (sq / 8) as i32;
        let file = (sq % 8) as i32;
        for dr in -1..=1 {
            for df in -1..=1 {
                if dr == 0 && df == 0 { continue; }
                let nr = rank + dr;
                let nf = file + df;
                if nr >= 0 && nr < 8 && nf >= 0 && nf < 8 {
                    table[sq] |= 1u64 << (nr * 8 + nf);
                }
            }
        }
    }
    table
}