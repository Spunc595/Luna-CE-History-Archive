use crate::board::{Colore, Pezzo, Scacchiera, Bitboard};

#[derive(Clone)]
pub struct Network {
    pub feature_weights: Vec<i16>,
    pub feature_bias: Vec<i16>,
}

impl Network {
    pub fn carica(_path: &str) -> Self {
        Network {
            feature_weights: vec![0; 768 * 256],
            feature_bias: vec![0; 256],
        }
    }
}

pub fn get_feature_index(pezzo: Pezzo, colore: Colore, sq: usize) -> usize {
    let piece_offset = pezzo.indice();
    let color_offset = if colore == Colore::Bianco { 0 } else { 6 };
    (color_offset + piece_offset) * 64 + sq
}

// Tabelle PSQT (Pawn, Knight, Bishop, King)
const PAWN_TABLE: [i32; 64] = [
     0,  0,  0,  0,  0,  0,  0,  0,
    50, 50, 50, 50, 50, 50, 50, 50,
    10, 10, 20, 30, 30, 20, 10, 10,
     5,  5, 10, 25, 25, 10,  5,  5,
     0,  0,  0, 20, 20,  0,  0,  0,
     5, -5,-10,  0,  0,-10, -5,  5,
     5, 10, 10,-20,-20, 10, 10,  5,
     0,  0,  0,  0,  0,  0,  0,  0
];

const KNIGHT_TABLE: [i32; 64] = [
    -50,-40,-30,-30,-30,-30,-40,-50,
    -40,-20,  0,  5,  5,  0,-20,-40,
    -30,  5, 10, 15, 15, 10,  5,-30,
    -30,  0, 15, 20, 20, 15,  0,-30,
    -30,  5, 15, 20, 20, 15,  5,-30,
    -30,  0, 10, 15, 15, 10,  0,-30,
    -40,-20,  0,  0,  0,  0,-20,-40,
    -50,-40,-30,-30,-30,-30,-40,-50,
];

const BISHOP_TABLE: [i32; 64] = [
    -20,-10,-10,-10,-10,-10,-10,-20,
    -10,  5,  0,  0,  0,  0,  5,-10,
    -10, 10, 10, 10, 10, 10, 10,-10,
    -10,  0, 10, 10, 10, 10,  0,-10,
    -10,  5,  5, 10, 10,  5,  5,-10,
    -10,  0,  5, 10, 10,  5,  0,-10,
    -10,  0,  0,  0,  0,  0,  0,-10,
    -20,-10,-10,-10,-10,-10,-10,-20,
];

const KING_TABLE_MG: [i32; 64] = [
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -20,-30,-30,-40,-40,-30,-30,-20,
    -10,-20,-20,-20,-20,-20,-20,-10,
     10, 10, -5, -10,-10, -5, 10, 10,
     20, 30, 10,  0,  0, 10, 30, 20
];

pub fn evaluate_nnue(board: &Scacchiera, _net: &Network) -> i32 {
    let mut score = 0;
    score += evaluate_side(board, Colore::Bianco);
    score -= evaluate_side(board, Colore::Nero);
    if board.turno == Colore::Nero { -score } else { score }
}

fn evaluate_side(board: &Scacchiera, c: Colore) -> i32 {
    let mut score = 0;
    let my_pieces = board.colori[c.indice()];
    let enemy_pawns = board.pezzi[Pezzo::Pedone.indice()] & board.colori[c.opposto().indice()];

    for sq in 0..64 {
        let bit = 1u64 << sq;
        if (my_pieces & bit) == 0 { continue; }

        let rank = sq / 8;
        let file = sq % 8;
        let table_idx = if c == Colore::Bianco { (7 - rank) * 8 + file } else { rank * 8 + file };

        let p_type = match board.get_piece_at(sq) {
            Some(p) => p,
            None => continue,
        };

        match p_type {
            Pezzo::Pedone => {
                score += 100 + PAWN_TABLE[table_idx];
                if is_passed_pawn(sq, c, enemy_pawns) {
                    let rel_rank = if c == Colore::Bianco { rank } else { 7 - rank };
                    score += (rel_rank as i32 * rel_rank as i32) * 5;
                }
            },
            Pezzo::Cavallo => score += 320 + KNIGHT_TABLE[table_idx],
            Pezzo::Alfiere => score += 330 + BISHOP_TABLE[table_idx],
            Pezzo::Torre   => score += 500,
            Pezzo::Regina  => score += 900,
            Pezzo::Re      => {
                score += KING_TABLE_MG[table_idx];
                score += check_king_safety(sq, board.pezzi[0] & my_pieces, c);
            },
        }
    }
    score
}

fn is_passed_pawn(sq: usize, color: Colore, enemy_pawns: Bitboard) -> bool {
    let file = sq % 8;
    let rank = sq / 8;
    let mut mask: u64 = 0;
    for f in (file as isize - 1)..=(file as isize + 1) {
        if f < 0 || f > 7 { continue; }
        let f_mask = 0x0101010101010101u64 << f;
        if color == Colore::Bianco {
            mask |= f_mask & (!0u64 << ((rank + 1) * 8));
        } else {
            mask |= f_mask & (!0u64 >> ((8 - rank) * 8));
        }
    }
    (enemy_pawns & mask) == 0
}

fn check_king_safety(king_sq: usize, my_pawns: u64, c: Colore) -> i32 {
    let rank = king_sq / 8;
    if (c == Colore::Bianco && rank <= 1) || (c == Colore::Nero && rank >= 6) {
        let shield_rank = if c == Colore::Bianco { 1 } else { 6 };
        let shield_mask = 0xFFu64 << (shield_rank * 8);
        return (my_pawns & shield_mask).count_ones() as i32 * 10;
    }
    0
}