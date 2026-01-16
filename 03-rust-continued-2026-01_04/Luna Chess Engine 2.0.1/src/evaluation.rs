use crate::board::{Scacchiera, Colore};
use crate::nnue::LunaNNUE;

// Valori materiali bilanciati per Midgame ed Endgame
pub const MG_VAL: [i32; 6] = [100, 320, 330, 500, 900, 0];
pub const EG_VAL: [i32; 6] = [120, 280, 300, 550, 950, 0];

// --- TABELLE POSIZIONALI (PST) ---
#[rustfmt::skip]
const KNIGHT_PST: [i32; 64] = [
    -50,-40,-30,-30,-30,-30,-40,-50,
    -40,-20,  0,  5,  5,  0,-20,-40,
    -30,  5, 10, 15, 15, 10,  5,-30,
    -30,  0, 15, 20, 20, 15,  0,-30,
    -30,  5, 15, 20, 20, 15,  5,-30,
    -30,  0, 10, 15, 15, 10,  0,-30,
    -40,-20,  0,  0,  0,  0,-20,-40,
    -50,-40,-30,-30,-30,-30,-40,-50,
];

#[rustfmt::skip]
const BISHOP_PST: [i32; 64] = [
    -20,-10,-10,-10,-10,-10,-10,-20,
    -10,  5,  0,  0,  0,  0,  5,-10,
    -10, 10, 10, 10, 10, 10, 10,-10,
    -10,  0, 10, 10, 10, 10,  0,-10,
    -10,  5,  5, 10, 10,  5,  5,-10,
    -10,  0,  5, 10, 10,  0,  5,-10,
    -10,  0,  0,  0,  0,  0,  0,-10,
    -20,-10,-10,-10,-10,-10,-10,-20,
];

#[rustfmt::skip]
const ROOK_PST: [i32; 64] = [
     0,  0,  0,  5,  5,  0,  0,  0,
     5, 10, 10, 10, 10, 10, 10,  5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
     0,  0,  0,  2,  2,  0,  0,  0
];

#[rustfmt::skip]
const KING_MG_PST: [i32; 64] = [
    25, 35, 15,  0,  0, 15, 35, 25,
    20, 20,  0,  0,  0,  0, 20, 20,
   -10,-20,-20,-20,-20,-20,-20,-10,
   -20,-30,-40,-40,-40,-40,-30,-20,
   -30,-40,-50,-50,-50,-50,-40,-30,
   -30,-40,-50,-50,-50,-50,-40,-30,
   -40,-50,-50,-50,-50,-50,-50,-40,
   -50,-50,-50,-50,-50,-50,-50,-50
];

#[rustfmt::skip]
const PAWN_PST: [i32; 64] = [
    0,  0,  0,  0,  0,  0,  0,  0,
    50, 50, 50, 50, 50, 50, 50, 50,
    10, 10, 20, 30, 30, 20, 10, 10,
     5,  5, 10, 25, 25, 10,  5,  5,
     0,  0,  0, 20, 20,  0,  0,  0,
     5, -5,-10,  0,  0,-10, -5,  5,
     5, 10, 10,-20,-20, 10, 10,  5,
     0,  0,  0,  0,  0,  0,  0,  0
];

// FUNZIONE RICHIESTA DA board.rs
#[inline(always)]
pub fn get_pst(p: usize, sq: usize, mg: bool) -> i32 {
    match p {
        0 => PAWN_PST[sq],
        1 => KNIGHT_PST[sq],
        2 => BISHOP_PST[sq],
        3 => ROOK_PST[sq],
        5 => if mg { KING_MG_PST[sq] } else { 0 },
        _ => 0,
    }
}

pub fn evaluate(board: &Scacchiera, nnue: Option<&LunaNNUE>) -> i32 {
    if let Some(net) = nnue {
        return net.evaluate(board);
    }

    let us = board.turno.indice();
    let them = 1 - us;

    let (score_us, phase_us) = evaluate_side(board, us);
    let (score_them, phase_them) = evaluate_side(board, them);

    let mg = score_us.0 - score_them.0;
    let eg = score_us.1 - score_them.1;
    let phase = (phase_us + phase_them).min(24);

    (mg * phase + eg * (24 - phase)) / 24
}

fn evaluate_side(board: &Scacchiera, side: usize) -> ((i32, i32), i32) {
    let mut mg = 0;
    let mut eg = 0;
    let mut phase = 0;
    let color_mask = board.colori[side];
    let enemy_pawns = board.pezzi[0] & board.colori[1 - side];
    let our_pawns = board.pezzi[0] & color_mask;

    for p in 0..6 {
        let mut bb = board.pezzi[p] & color_mask;
        while bb != 0 {
            let sq = bb.trailing_zeros() as usize;
            let pst_idx = if side == 0 { sq } else { sq ^ 56 };

            mg += MG_VAL[p];
            eg += EG_VAL[p];
            phase += [0, 1, 1, 2, 4, 0][p];

            mg += get_pst(p, pst_idx, true);
            eg += get_pst(p, pst_idx, false);

            if p == 3 { // Bonus Torri su colonne aperte
                let file_mask = 0x0101010101010101u64 << (sq % 8);
                if (our_pawns & file_mask) == 0 {
                    mg += 10;
                    if (enemy_pawns & file_mask) == 0 { mg += 15; }
                }
            }
            bb &= bb - 1;
        }
    }

    // Penalità pedoni doppi
    for f in 0..8 {
        let file_mask = 0x0101010101010101u64 << f;
        if (our_pawns & file_mask).count_ones() > 1 {
            mg -= 15; eg -= 20;
        }
    }

    ((mg, eg), phase)
}