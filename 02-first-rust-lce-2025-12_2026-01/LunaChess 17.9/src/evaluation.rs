use crate::board::{Scacchiera, Colore};

pub const MG_VAL: [i32; 6] = [82, 337, 365, 477, 1025, 0];
pub const EG_VAL: [i32; 6] = [94, 281, 297, 512, 936, 0];
const MAX_PHASE: i32 = 24;
const PHASE_WEIGHTS: [i32; 6] = [0, 1, 1, 2, 4, 0];
const BISHOP_PAIR_BONUS: i32 = 50;
const PAWN_STORM_PENALTY: i32 = 60;
const BLOCKED_C_PAWN_PENALTY: i32 = 45;
const KING_EXPOSED_PENALTY: i32 = 80;

const PAWN_MG: [i32; 64] = [
    0, 0, 0, 0, 0, 0, 0, 0, 50, 50, 50, 50, 50, 50, 50, 50,
    10, 10, 20, 30, 30, 20, 10, 10, 5, 5, 10, 25, 25, 10, 5, 5,
    0, 0, 0, 20, 20, 0, 0, 0, 5, -5, -10, 0, 0, -10, -5, 5,
    5, 10, 10, -20, -20, 10, 10, 5, 0, 0, 0, 0, 0, 0, 0, 0
];

const FILE_MASKS: [u64; 8] = [
    0x0101010101010101, 0x0202020202020202, 0x0404040404040404, 0x0808080808080808,
    0x1010101010101010, 0x2020202020202020, 0x4040404040404040, 0x8080808080808080
];

pub fn get_pst(p: usize, sq: usize, _mg: bool) -> i32 {
    match p {
        0 => PAWN_MG[sq],
        _ => 0,
    }
}

pub fn evaluate(board: &Scacchiera) -> i32 {
    let mut mg = 0;
    let mut eg = 0;
    let mut phase = 0;

    for c in 0..2 {
        let mult = if c == 0 { 1 } else { -1 };
        for p in 0..6 {
            let mut pieces = board.pezzi[p] & board.colori[c];
            while pieces != 0 {
                let sq = pieces.trailing_zeros() as usize;
                phase += PHASE_WEIGHTS[p];
                let pst_idx = if c == 0 { sq } else { sq ^ 56 };
                mg += (MG_VAL[p] + get_pst(p, pst_idx, true)) * mult;
                eg += (EG_VAL[p] + get_pst(p, pst_idx, false)) * mult;
                pieces &= pieces - 1;
            }
        }
    }

    phase = phase.min(MAX_PHASE);
    mg += evaluate_king_safety(board) + evaluate_strategic(board);
    let score = (mg * phase + eg * (MAX_PHASE - phase)) / MAX_PHASE;
    if board.turno == Colore::Nero { -score } else { score }
}

fn evaluate_king_safety(board: &Scacchiera) -> i32 {
    let mut score = 0;
    for c in 0..2 {
        let k_bb = board.pezzi[5] & board.colori[c];
        if k_bb == 0 { continue; }
        let k_sq = k_bb.trailing_zeros() as usize;
        let file = k_sq % 8;
        let mult = if c == 0 { 1 } else { -1 };
        let opp_pawns = board.pezzi[0] & board.colori[1 - c];
        let mut penalty = 0;

        for f in (file.saturating_sub(1))..=(file + 1).min(7) {
            let mask = FILE_MASKS[f];
            let storm_row = if c == 0 { 0x0000FF0000000000 } else { 0x0000000000FF0000 };
            if (opp_pawns & mask & storm_row) != 0 { penalty += PAWN_STORM_PENALTY; }
        }
        if (file > 2 && file < 5) && (board.occupazione().count_ones() > 20) { penalty += KING_EXPOSED_PENALTY; }
        score -= penalty * mult;
    }
    score
}

fn evaluate_strategic(board: &Scacchiera) -> i32 {
    let mut score = 0;
    if (board.pezzi[1] & board.colori[0] & (1 << 18)) != 0 && (board.pezzi[0] & board.colori[0] & (1 << 10)) != 0 {
        if (board.pezzi[0] & board.colori[0] & (1 << 27)) != 0 { score -= BLOCKED_C_PAWN_PENALTY; }
    }
    score
}