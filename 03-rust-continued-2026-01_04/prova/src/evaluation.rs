use crate::board::{Scacchiera, Colore, Pezzo};

// --- 1. VALORI MATERIALE ---
const MG_PAWN: i32 = 100;
const MG_KNIGHT: i32 = 320;
const MG_BISHOP: i32 = 330;
const MG_ROOK: i32 = 500;
const MG_QUEEN: i32 = 900;

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
    -40,-20,  0,  0,  0,  0,-20,-40,
    -30,  0, 10, 15, 15, 10,  0,-30,
    -30,  5, 15, 20, 20, 15,  5,-30,
    -30,  0, 15, 20, 20, 15,  0,-30,
    -30,  5, 10, 15, 15, 10,  5,-30,
    -40,-20,  0,  5,  5,  0,-20,-40,
    -50,-40,-30,-30,-30,-30,-40,-50
];

#[rustfmt::skip]
const BISHOP_PST: [i32; 64] = [
    -20,-10,-10,-10,-10,-10,-10,-20,
    -10,  0,  0,  0,  0,  0,  0,-10,
    -10,  0,  5, 10, 10,  5,  0,-10,
    -10,  5,  5, 10, 10,  5,  5,-10,
    -10,  0, 10, 10, 10, 10,  0,-10,
    -10, 10, 10, 10, 10, 10, 10,-10,
    -10,  5,  0,  0,  0,  0,  5,-10,
    -20,-10,-10,-10,-10,-10,-10,-20
];

#[rustfmt::skip]
const ROOK_PST: [i32; 64] = [
    0,  0,  0,  0,  0,  0,  0,  0,
    5, 10, 10, 10, 10, 10, 10,  5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    0,  0,  0,  5,  5,  0,  0,  0
];

#[rustfmt::skip]
const QUEEN_PST: [i32; 64] = [
    -20,-10,-10, -5, -5,-10,-10,-20,
    -10,  0,  0,  0,  0,  0,  0,-10,
    -10,  0,  5,  5,  5,  5,  0,-10,
    -5,   0,  5,  5,  5,  5,  0, -5,
    0,    0,  5,  5,  5,  5,  0, -5,
    -10,  5,  5,  5,  5,  5,  0,-10,
    -10,  0,  5,  0,  0,  0,  0,-10,
    -20,-10,-10, -5, -5,-10,-10,-20
];

#[rustfmt::skip]
const KING_PST: [i32; 64] = [
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -20,-30,-30,-40,-40,-30,-30,-20,
    -10,-20,-20,-20,-20,-20,-20,-10,
     20, 20,  0,  0,  0,  0, 20, 20,
     20, 30, 10,  0,  0, 10, 30, 20
];

pub fn evaluate(board: &Scacchiera) -> i32 {
    let mut score = 0;
    
    let mut white_king_sq: i32 = -1;
    let mut black_king_sq: i32 = -1;

    // [OTTIMIZZAZIONE]: Usiamo i Bitboard invece del ciclo 0..64
    for p_idx in 0..6 {
        let piece_val = match p_idx {
            0 => MG_PAWN, 1 => MG_KNIGHT, 2 => MG_BISHOP,
            3 => MG_ROOK, 4 => MG_QUEEN, _ => 0,
        };

        // --- BIANCO ---
        let mut bb_w = board.pezzi[p_idx] & board.colori[0];
        while bb_w != 0 {
            let sq = bb_w.trailing_zeros() as usize;
            let mut val = piece_val + PAWN_PST[sq]; // Fallback PST generalizzato
            
            match p_idx {
                0 => { // Pedone
                    val = MG_PAWN + PAWN_PST[sq];
                    let rank = sq / 8;
                    if rank >= 4 { val += (rank as i32).pow(2) * 5; }
                },
                1 => val = MG_KNIGHT + KNIGHT_PST[sq],
                2 => val = MG_BISHOP + BISHOP_PST[sq],
                3 => val = MG_ROOK + ROOK_PST[sq],
                4 => val = MG_QUEEN + QUEEN_PST[sq],
                5 => {
                    white_king_sq = sq as i32;
                    val = KING_PST[sq];
                },
                _ => {}
            }
            score += val;
            bb_w &= bb_w - 1;
        }

        // --- NERO ---
        let mut bb_b = board.pezzi[p_idx] & board.colori[1];
        while bb_b != 0 {
            let sq = bb_b.trailing_zeros() as usize;
            let pst_idx = sq ^ 56;
            let mut val = piece_val + PAWN_PST[pst_idx];
            
            match p_idx {
                0 => {
                    val = MG_PAWN + PAWN_PST[pst_idx];
                    let rank = sq / 8;
                    let advancement = 7 - rank;
                    if advancement >= 4 { val += (advancement as i32).pow(2) * 5; }
                },
                1 => val = MG_KNIGHT + KNIGHT_PST[pst_idx],
                2 => val = MG_BISHOP + BISHOP_PST[pst_idx],
                3 => val = MG_ROOK + ROOK_PST[pst_idx],
                4 => val = MG_QUEEN + QUEEN_PST[pst_idx],
                5 => {
                    black_king_sq = sq as i32;
                    val = KING_PST[pst_idx];
                },
                _ => {}
            }
            score -= val;
            bb_b &= bb_b - 1;
        }
    }

    if white_king_sq != -1 && black_king_sq != -1 {
        if score > 300 { score += evaluate_mop_up(white_king_sq, black_king_sq); }
        else if score < -300 { score -= evaluate_mop_up(black_king_sq, white_king_sq); }
    }

    if board.turno == Colore::Bianco { score } else { -score }
}

fn evaluate_mop_up(winner_king_sq: i32, loser_king_sq: i32) -> i32 {
    let l_rank = loser_king_sq / 8;
    let l_file = loser_king_sq % 8;
    let w_rank = winner_king_sq / 8;
    let w_file = winner_king_sq % 8;
    let center_dist = (2 * l_rank - 7).abs() + (2 * l_file - 7).abs();
    let dist_kings = (w_rank - l_rank).abs() + (w_file - l_file).abs();
    (center_dist * 25) + (14 - dist_kings) * 20
}