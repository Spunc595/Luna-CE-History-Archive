use crate::board::{Scacchiera, Pezzo, Colore};

// Pesi Materiali
const PAWN_VAL: i32 = 100;
const KNIGHT_VAL: i32 = 320;
const BISHOP_VAL: i32 = 330;
const ROOK_VAL: i32 = 500;
const QUEEN_VAL: i32 = 900;
const KING_VAL: i32 = 20000;

// PST (Piece-Square Tables)
const PAWN_PST: [i32; 64] = [
    0,  0,  0,  0,  0,  0,  0,  0,
    50, 50, 50, 50, 50, 50, 50, 50,
    10, 10, 20, 30, 30, 20, 10, 10,
    5,  5, 10, 25, 25, 10,  5,  5,
    0,  0,  0, 20, 20,  0,  0,  0,
    5, -5,-10,  0,  0,-10, -5,  5,
    5, 10, 10,-20,-20, 10, 10,  5,
    0,  0,  0,  0,  0,  0,  0,  0,
];

const KNIGHT_PST: [i32; 64] = [
    -50,-40,-30,-30,-30,-30,-40,-50,
    -40,-20,  0,  0,  0,  0,-20,-40,
    -30,  0, 10, 15, 15, 10,  0,-30,
    -30,  5, 15, 20, 20, 15,  5,-30,
    -30,  0, 15, 20, 20, 15,  0,-30,
    -30,  5, 10, 15, 15, 10,  5,-30,
    -40,-20,  0,  5,  5,  0,-20,-40,
    -50,-40,-30,-30,-30,-30,-40,-50,
];

pub fn evaluate_classical(board: &Scacchiera) -> i32 {
    let mut score = 0;
    let mut white_material = 0;
    let mut black_material = 0;

    // 1. MATERIALE & PST
    // Correzione: Usiamo get_piece_at invece di accedere direttamente all'array
    for i in 0..64 {
        if let Some(pezzo) = board.get_piece_at(i) {
            let val = piece_value(pezzo);
            // Verifica colore usando le bitmask
            let is_white = (board.colori[Colore::Bianco.indice()] & (1u64 << i)) != 0;
            
            let pst_idx = if is_white { i } else { 63 - i };
            let pst_score = match pezzo {
                Pezzo::Pedone => PAWN_PST[pst_idx],
                Pezzo::Cavallo => KNIGHT_PST[pst_idx],
                _ => 0,
            };

            if is_white {
                score += val + pst_score;
                white_material += val;
            } else {
                score -= val + pst_score;
                black_material += val;
            }
        }
    }

    // 2. MOBILITÀ & SICUREZZA
    let is_opening = white_material > 2000 && black_material > 2000;

    if is_opening {
        // Correzione: Usiamo 'pezzi' invece di 'pezzi_tipo'
        let w_pawns = board.pezzi[Pezzo::Pedone.indice()] & board.colori[Colore::Bianco.indice()];
        let b_pawns = board.pezzi[Pezzo::Pedone.indice()] & board.colori[Colore::Nero.indice()];

        // d3/d4 e3/e4
        if (w_pawns & ((1<<19) | (1<<27))) != 0 { score += 15; }
        if (w_pawns & ((1<<20) | (1<<28))) != 0 { score += 15; }
        // d6/d5 e6/e5
        if (b_pawns & ((1<<43) | (1<<35))) != 0 { score -= 15; }
        if (b_pawns & ((1<<44) | (1<<36))) != 0 { score -= 15; }

        let w_knights = board.pezzi[Pezzo::Cavallo.indice()] & board.colori[Colore::Bianco.indice()];
        let b_knights = board.pezzi[Pezzo::Cavallo.indice()] & board.colori[Colore::Nero.indice()];

        if (w_knights & 0x0000FFFF00000000) != 0 { score -= 10; }
        if (b_knights & 0x00000000FFFF0000) != 0 { score += 10; }
    }

    if board.turno == Colore::Bianco { score } else { -score }
}

fn piece_value(p: Pezzo) -> i32 {
    match p {
        Pezzo::Pedone => PAWN_VAL,
        Pezzo::Cavallo => KNIGHT_VAL,
        Pezzo::Alfiere => BISHOP_VAL,
        Pezzo::Torre => ROOK_VAL,
        Pezzo::Regina => QUEEN_VAL,
        Pezzo::Re => KING_VAL,
    }
}