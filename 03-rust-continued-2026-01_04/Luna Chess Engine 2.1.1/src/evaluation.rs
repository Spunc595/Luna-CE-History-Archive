use crate::board::{Scacchiera, Colore};
use crate::nnue::LunaNNUE;

pub const MG_VAL: [i32; 6] = [100, 320, 330, 500, 900, 0];
pub const EG_VAL: [i32; 6] = [120, 280, 300, 550, 950, 0];

// PST standard come le tue, ma con King Safety migliorata
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

// ... (Manteniamo le altre tue PST: KNIGHT, BISHOP, ROOK, PAWN)

#[inline(always)]
pub fn get_pst(p: usize, sq: usize, mg: bool) -> i32 {
    // Usa le tue tabelle PST qui (PAWN, KNIGHT, BISHOP, ROOK, KING_MG)
    match p {
        0 => 0, // Inserisci riferimento a PAWN_PST
        1 => 0, // Inserisci riferimento a KNIGHT_PST
        2 => 0, // Inserisci riferimento a BISHOP_PST
        3 => 0, // Inserisci riferimento a ROOK_PST
        5 => if mg { KING_MG_PST[sq] } else { 0 },
        _ => 0,
    }
}

pub fn evaluate(board: &Scacchiera, nnue: Option<&LunaNNUE>) -> i32 {
    if let Some(net) = nnue { return net.evaluate(board); }
    let us = board.turno.indice();
    let (s_us, p_us) = evaluate_side(board, us);
    let (s_them, p_them) = evaluate_side(board, 1 - us);
    let mg = s_us.0 - s_them.0;
    let eg = s_us.1 - s_them.1;
    let phase = (p_us + p_them).min(24);
    (mg * phase + eg * (24 - phase)) / 24
}

fn evaluate_side(board: &Scacchiera, side: usize) -> ((i32, i32), i32) {
    let mut mg = 0; let mut eg = 0; let mut phase = 0;
    let color_mask = board.colori[side];
    let our_pawns = board.pezzi[0] & color_mask;
    let minor_pieces = (board.pezzi[1] | board.pezzi[2]).count_ones();

    for p in 0..6 {
        let mut bb = board.pezzi[p] & color_mask;
        while bb != 0 {
            let sq = bb.trailing_zeros() as usize;
            let pst_idx = if side == 0 { sq } else { sq ^ 56 };
            mg += MG_VAL[p]; eg += EG_VAL[p];
            phase += [0, 1, 1, 2, 4, 0][p];
            
            // Logica Donna Prudente (Qb1 style)
            if p == 4 && minor_pieces > 3 {
                let start_sq = if side == 0 { 3 } else { 59 };
                if sq != start_sq { mg -= 15; } // Penalità leggera per uscita precoce
            }
            
            bb &= bb - 1;
        }
    }
    ((mg, eg), phase)
}