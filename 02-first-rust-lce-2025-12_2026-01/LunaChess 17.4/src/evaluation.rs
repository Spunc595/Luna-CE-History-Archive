use crate::board::{Scacchiera, Colore, Pezzo, Bitboard};

pub const MG_VAL: [i32; 6] = [100, 320, 330, 500, 900, 0];
pub const EG_VAL: [i32; 6] = [120, 310, 340, 520, 950, 0];
pub const SEE_VAL: [i32; 6] = [100, 320, 330, 500, 900, 10000];

const PAWN_PST: [i32; 64] = [
    0, 0, 0, 0, 0, 0, 0, 0, 5, 10, 10, -20, -20, 10, 10, 5, 5, -5, -10, 0, 0, -10, -5, 5, 0, 0, 0, 20, 20, 0, 0, 0,
    5, 5, 10, 25, 25, 10, 5, 5, 10, 10, 20, 30, 30, 20, 10, 10, 50, 50, 50, 50, 50, 50, 50, 50, 0, 0, 0, 0, 0, 0, 0, 0
];

const KNIGHT_PST: [i32; 64] = [
    -50,-40,-30,-30,-30,-30,-40,-50, -40,-20, 0, 5, 5, 0,-20,-40, -30, 5, 10, 15, 15, 10, 5,-30, -30, 0, 15, 20, 20, 15, 0,-30,
    -30, 5, 15, 20, 20, 15, 5,-30, -30, 0, 10, 15, 15, 10, 0,-30, -40,-20, 0, 0, 0, 0,-20,-40, -50,-40,-30,-30,-30,-30,-40,-50
];

const KING_MG_PST: [i32; 64] = [
    20, 30, 10, 0, 0, 10, 30, 20, 20, 20, 0, 0, 0, 0, 20, 20, -10,-20,-20,-20,-20,-20,-20,-10, -20,-30,-30,-40,-40,-30,-30,-20,
    -30,-40,-40,-50,-50,-40,-40,-30, -30,-40,-40,-50,-50,-40,-40,-30, -30,-40,-40,-50,-50,-40,-40,-30, -30,-40,-40,-50,-50,-40,-40,-30
];

pub fn get_pst(p: usize, sq: usize, _mg: bool) -> i32 {
    match p {
        0 => PAWN_PST[sq],
        1 => KNIGHT_PST[sq],
        5 => KING_MG_PST[sq],
        _ => 0,
    }
}

pub fn evaluate(board: &Scacchiera) -> i32 {
    let mut score = board.pst_val;
    // Materiale base (se non già incluso nel PST incrementale)
    for c in 0..2 {
        for p in 0..5 {
            let count = (board.pezzi[p] & board.colori[c]).count_ones() as i32;
            if c == 0 { score += count * MG_VAL[p]; } else { score -= count * MG_VAL[p]; }
        }
    }
    if board.turno == Colore::Nero { score = -score; }
    score
}

pub fn see(board: &Scacchiera, sq: usize, target_val: i32, attacker_val: i32, mut side: Colore) -> i32 {
    let mut gain = [0i32; 32];
    gain[0] = target_val;
    // Versione super semplificata per velocità
    attacker_val - target_val
}