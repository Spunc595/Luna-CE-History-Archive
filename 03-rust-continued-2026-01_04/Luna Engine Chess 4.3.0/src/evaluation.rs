use crate::board::{Scacchiera, Colore, Pezzo};
use crate::nnue::LunaNNUE;

// --- 1. VALORI MATERIALE (Midgame e Endgame) ---
const MG_VALS: [i32; 6] = [100, 320, 330, 500, 900, 0];
const EG_VALS: [i32; 6] = [125, 315, 330, 510, 920, 0];

// --- 2. PIECE-SQUARE TABLES (PST) ---

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
const KNIGHT_PST: [i32; 64] = [
    -50,-40,-30,-30,-30,-30,-40,-50,
    -40,-20,  0,  5,  5,  0,-20,-40,
    -30,  5, 10, 15, 15, 10,  5,-30,
    -30,  0, 15, 20, 20, 15,  0,-30,
    -30,  5, 15, 20, 20, 15,  5,-30,
    -30,  0, 10, 15, 15, 10,  0,-30,
    -40,-20,  0,  0,  0,  0,-20,-40,
    -50,-40,-30,-30,-30,-30,-40,-50
];

#[rustfmt::skip]
const BISHOP_PST: [i32; 64] = [
    -20,-10,-10,-10,-10,-10,-10,-20,
    -10,  5,  0,  0,  0,  0,  5,-10,
    -10, 10, 10, 10, 10, 10, 10,-10,
    -10,  0, 10, 10, 10, 10,  0,-10,
    -10,  5,  5, 10, 10,  5,  5,-10,
    -10,  0,  5, 10, 10,  5,  0,-10,
    -10,  0,  0,  0,  0,  0,  0,-10,
    -20,-10,-10,-10,-10,-10,-10,-20
];

#[rustfmt::skip]
const ROOK_PST: [i32; 64] = [
     0,  0,  0,  5,  5,  0,  0,  0,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
     5, 10, 10, 10, 10, 10, 10,  5,
     0,  0,  0,  0,  0,  0,  0,  0
];

#[rustfmt::skip]
const KING_PST_MG: [i32; 64] = [
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -20,-30,-30,-40,-40,-30,-30,-20,
    -10,-20,-20,-20,-20,-20,-20,-10,
     20, 20,  0,  0,  0,  0, 20, 20,
     20, 30, 10,  0,  0, 10, 30, 20
];

#[rustfmt::skip]
const KING_PST_EG: [i32; 64] = [
    -50,-30,-30,-30,-30,-30,-30,-50,
    -30,-10,  0,  0,  0,  0,-10,-30,
    -30,  0, 20, 30, 30, 20,  0,-30,
    -30,  0, 30, 40, 40, 30,  0,-30,
    -30,  0, 30, 40, 40, 30,  0,-30,
    -30,  0, 20, 30, 30, 20,  0,-30,
    -30,-10,  0,  0,  0,  0,-10,-30,
    -50,-30,-30,-30,-30,-30,-30,-50
];

// --- FUNZIONE PRINCIPALE ---
pub fn evaluate(board: &Scacchiera, nnue: Option<&LunaNNUE>) -> i32 {
    // 1. TENTA LA VALUTAZIONE NNUE (Scommenta solo quando hai risolto il caricamento)
    /*
    if let Some(net) = nnue {
        return net.evaluate(board);
    }
    */

    let mut mg_score = 0;
    let mut eg_score = 0;
    let mut game_phase = 0;
    let mut white_king_sq: i32 = -1;
    let mut black_king_sq: i32 = -1;

    for sq in 0..64 {
        if let Some((colore, pezzo)) = board.pezzo_e_colore_in(sq) {
            let is_white = colore == Colore::Bianco;
            let sq_idx = sq as usize;
            let pst_idx = if is_white { sq_idx } else { sq_idx ^ 56 };
            
            let p_idx = pezzo_to_idx(pezzo);
            game_phase += phase_weight(pezzo);

            let mut mg = MG_VALS[p_idx];
            let mut eg = EG_VALS[p_idx];

            match pezzo {
                Pezzo::Pedone => { mg += PAWN_PST[pst_idx]; eg += PAWN_PST[pst_idx]; },
                Pezzo::Cavallo => { mg += KNIGHT_PST[pst_idx]; eg += KNIGHT_PST[pst_idx]; },
                Pezzo::Alfiere => { mg += BISHOP_PST[pst_idx]; eg += BISHOP_PST[pst_idx]; },
                Pezzo::Torre   => { mg += ROOK_PST[pst_idx]; eg += ROOK_PST[pst_idx]; },
                Pezzo::Re => {
                    if is_white { white_king_sq = sq as i32; } else { black_king_sq = sq as i32; }
                    mg += KING_PST_MG[pst_idx];
                    eg += KING_PST_EG[pst_idx];
                },
                _ => {}
            }

            if is_white { mg_score += mg; eg_score += eg; } 
            else { mg_score -= mg; eg_score -= eg; }
        }
    }

    // King Safety Semplice (Statica)
    mg_score += check_king_safety(board, white_king_sq, Colore::Bianco);
    mg_score -= check_king_safety(board, black_king_sq, Colore::Nero);

    // Calcolo Tapered Score
    let phase = (game_phase.min(24) * 256) / 24;
    let score = ((mg_score * (256 - phase)) + (eg_score * phase)) / 256;

    if board.turno == Colore::Bianco { score } else { -score }
}

fn pezzo_to_idx(p: Pezzo) -> usize {
    match p {
        Pezzo::Pedone => 0, Pezzo::Cavallo => 1, Pezzo::Alfiere => 2,
        Pezzo::Torre => 3, Pezzo::Regina => 4, Pezzo::Re => 5,
    }
}

fn phase_weight(p: Pezzo) -> i32 {
    match p {
        Pezzo::Cavallo => 1, Pezzo::Alfiere => 1,
        Pezzo::Torre => 2, Pezzo::Regina => 4,
        _ => 0,
    }
}

fn check_king_safety(board: &Scacchiera, king_sq: i32, color: Colore) -> i32 {
    if king_sq == -1 { return 0; }
    let mut safety_score = 0;
    let rank = king_sq / 8;
    let file = king_sq % 8;
    let target_rank = if color == Colore::Bianco { rank - 1 } else { rank + 1 };
    
    if target_rank >= 0 && target_rank < 8 {
        for f_offset in -1..=1 {
            let target_file = file + f_offset;
            if target_file >= 0 && target_file < 8 {
                let check_sq = (target_rank * 8 + target_file) as usize;
                match board.pezzo_e_colore_in(check_sq) {
                    Some((c, Pezzo::Pedone)) if c == color => safety_score += 15,
                    _ => safety_score -= 20, 
                }
            }
        }
    }
    safety_score
}