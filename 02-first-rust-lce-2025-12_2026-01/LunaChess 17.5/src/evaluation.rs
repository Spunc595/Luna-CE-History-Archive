use crate::board::{Scacchiera, Colore, Bitboard};

pub const MG_VAL: [i32; 6] = [100, 320, 330, 500, 900, 0];
pub const EG_VAL: [i32; 6] = [120, 310, 340, 520, 950, 0];
// Aggiunta costante mancante per risolvere l'errore
pub const SEE_VAL: [i32; 6] = [100, 320, 330, 500, 900, 20000]; 

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

const FILE_MASKS: [u64; 8] = [
    0x0101010101010101, 0x0202020202020202, 0x0404040404040404, 0x0808080808080808,
    0x1010101010101010, 0x2020202020202020, 0x4040404040404040, 0x8080808080808080
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
    for c in 0..2 {
        for p in 0..5 {
            let count = (board.pezzi[p] & board.colori[c]).count_ones() as i32;
            if c == 0 { score += count * MG_VAL[p]; } else { score -= count * MG_VAL[p]; }
        }
    }
    score += evaluate_queen_development(board);
    score += evaluate_king_safety(board);

    if board.turno == Colore::Nero { score = -score; }
    score
}

fn evaluate_queen_development(board: &Scacchiera) -> i32 {
    let mut penalty = 0;
    let minor_pieces = (board.pezzi[1] | board.pezzi[2]).count_ones();
    if minor_pieces > 4 {
        if (board.pezzi[4] & board.colori[0]) != 0 {
            if (board.pezzi[4] & board.colori[0] & (1 << 3)) == 0 { penalty -= 30; }
        }
        if (board.pezzi[4] & board.colori[1]) != 0 {
            if (board.pezzi[4] & board.colori[1] & (1 << 59)) == 0 { penalty += 30; }
        }
    }
    penalty
}

fn evaluate_king_safety(board: &Scacchiera) -> i32 {
    let mut score = 0;
    for c in 0..2 { 
        let king_bit = board.pezzi[5] & board.colori[c];
        if king_bit == 0 { continue; }
        let king_sq = king_bit.trailing_zeros() as usize;
        let king_file = king_sq % 8;
        let my_pawns = board.pezzi[0] & board.colori[c];
        let enemy_pawns = board.pezzi[0] & board.colori[1-c];
        let mut penalty = 0;
        let start_f = if king_file > 0 { king_file - 1 } else { 0 };
        let end_f = if king_file < 7 { king_file + 1 } else { 7 };

        for f in start_f..=end_f {
            let file_mask = FILE_MASKS[f];
            let has_my_pawn = (my_pawns & file_mask) != 0;
            let is_open_file = ((my_pawns | enemy_pawns) & file_mask) == 0;
            if !has_my_pawn { penalty += 15; }
            if is_open_file { penalty += 25; }
        }
        if (board.pezzi[4] | board.pezzi[3]).count_ones() > 4 {
            if king_file >= 3 && king_file <= 4 { penalty += 20; }
        }
        if c == 0 { score -= penalty; } else { score += penalty; }
    }
    score
}

pub fn see(_board: &Scacchiera, _sq: usize, target_val: i32, attacker_val: i32, side: Colore) -> i32 {
    if side == Colore::Bianco { attacker_val - target_val } else { target_val - attacker_val }
}