use crate::board::{Scacchiera, Colore};
use crate::nnue::LunaNNUE;

// --- COSTANTI MATERIALE ---
pub const MG_VAL: [i32; 6] = [82, 337, 365, 477, 1025, 0];
pub const EG_VAL: [i32; 6] = [94, 281, 297, 512, 936, 0];

// --- TABELLE POSIZIONALI (PST) ---
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

#[rustfmt::skip]
const KING_MG_PST: [i32; 64] = [
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -20,-30,-30,-40,-40,-30,-30,-20,
    -10,-20,-20,-20,-20,-20,-20,-10,
     20, 30, 10,  0,  0, 10, 30, 20,
     20, 30, 10,-50,-50, 10, 30, 20  // Penalità brutale se il Re lascia la prima traversa
];

#[rustfmt::skip]
const KING_EG_PST: [i32; 64] = [
    -50,-40,-30,-20,-20,-30,-40,-50,
    -30,-20,-10,  0,  0,-10,-20,-30,
    -30,-10, 20, 30, 30, 20,-10,-30,
    -30,-10, 30, 40, 40, 30,-10,-30,
    -30,-10, 30, 40, 40, 30,-10,-30,
    -30,-10, 20, 30, 30, 20,-10,-30,
    -30,-30,  0,  0,  0,  0,-30,-30,
    -50,-30,-30,-30,-30,-30,-30,-50
];

#[inline(always)]
pub fn get_pst(p: usize, sq: usize, mg: bool) -> i32 {
    match p {
        0 => PAWN_PST[sq],
        5 => if mg { KING_MG_PST[sq] } else { KING_EG_PST[sq] },
        _ => 0, // Altri pezzi gestiti dalla NNUE o valori medi
    }
}

pub fn evaluate(board: &Scacchiera, nnue: Option<&LunaNNUE>) -> i32 {
    let mut score = 0;

    // 1. Applichiamo la NNUE se disponibile
    if let Some(net) = nnue {
        score = net.evaluate(board);
    } else {
        score = evaluate_classic(board);
    }

    // 2. CORREZIONE DI SICUREZZA (Safety Overlay)
    // Se siamo ancora in apertura/mediogioco, puniamo il Re fuori posto
    // anche se la NNUE dice il contrario.
    if !is_endgame(board) {
        let white_king_sq = (board.pezzi[5] & board.colori[0]).trailing_zeros() as usize;
        let black_king_sq = (board.pezzi[5] & board.colori[1]).trailing_zeros() as usize;
        
        if white_king_sq < 64 {
            score += KING_MG_PST[white_king_sq];
        }
        if black_king_sq < 64 {
            score -= KING_MG_PST[black_king_sq ^ 56];
        }
    }

    if board.turno == Colore::Nero { -score } else { score }
}

fn is_endgame(board: &Scacchiera) -> bool {
    // Definizione semplice di finale: niente regine o pochi pezzi
    (board.pezzi[4] == 0) || (board.occupazione().count_ones() < 10)
}

fn evaluate_classic(board: &Scacchiera) -> i32 {
    let mut mg = 0;
    let mut eg = 0;
    let mut phase = 0;
    
    for c in 0..2 {
        let us = board.colori[c];
        let side = if c == 0 { 1 } else { -1 };
        for p in 0..6 {
            let mut pieces = board.pezzi[p] & us;
            while pieces != 0 {
                let sq = pieces.trailing_zeros() as usize;
                phase += [0, 1, 1, 2, 4, 0][p];
                let pst_idx = if c == 0 { sq } else { sq ^ 56 };
                mg += (MG_VAL[p] + get_pst(p, pst_idx, true)) * side;
                eg += (EG_VAL[p] + get_pst(p, pst_idx, false)) * side;
                pieces &= pieces - 1;
            }
        }
    }
    let p = phase.min(24);
    (mg * p + eg * (24 - p)) / 24
}