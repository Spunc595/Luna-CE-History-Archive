use crate::board::{Colore, Pezzo, Scacchiera};

#[derive(Clone)]
pub struct Network {
    pub feature_weights: Vec<i16>,
    pub feature_bias: Vec<i16>,
}

impl Network {
    pub fn carica(_path: &str) -> Self {
        // Restituisce una rete "vuota" (pesi a zero) per ora
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

// --- TABELLE DI POSIZIONAMENTO (PSQT) ---
// Valori positivi incoraggiano il pezzo a stare in quella casa.

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
    
    // Calcoliamo il punteggio dal punto di vista del Bianco
    score += evaluate_side(board, Colore::Bianco);
    score -= evaluate_side(board, Colore::Nero);

    // Restituiamo il punteggio relativo al giocatore di turno
    if board.turno == Colore::Nero { -score } else { score }
}

fn evaluate_side(board: &Scacchiera, c: Colore) -> i32 {
    let mut score = 0;
    let my_pieces = board.colori[c.indice()];

    // Valori dei pezzi standard
    const VAL_PAWN: i32 = 100;
    const VAL_KNIGHT: i32 = 320;
    const VAL_BISHOP: i32 = 330;
    const VAL_ROOK: i32 = 500;
    const VAL_QUEEN: i32 = 900;

    // Bonus Mobilità: un motore attivo è un motore forte
    // Nota: genera_mosse() qui potrebbe essere pesante, 
    // ma per ora lo teniamo per la qualità del mediogioco.
    score += board.genera_mosse().len() as i32 * 2;

    for sq in 0..64 {
        let bit = 1u64 << sq;
        if (my_pieces & bit) == 0 { continue; }

        let rank = sq / 8;
        let file = sq % 8;
        
        // Specchiamo l'indice della tabella per il Nero
        let table_idx = if c == Colore::Bianco {
            (7 - rank) * 8 + file
        } else {
            rank * 8 + file
        };

        if (board.pezzi[0] & bit) != 0 { // PEDONE
            score += VAL_PAWN + PAWN_TABLE[table_idx];
            // Penalità pedone isolato
            if (board.pezzi[0] & my_pieces & get_adjacent_files_mask(file)) == 0 {
                score -= 15;
            }
        } 
        else if (board.pezzi[1] & bit) != 0 { score += VAL_KNIGHT + KNIGHT_TABLE[table_idx]; }
        else if (board.pezzi[2] & bit) != 0 { score += VAL_BISHOP + BISHOP_TABLE[table_idx]; }
        else if (board.pezzi[3] & bit) != 0 { score += VAL_ROOK; }
        else if (board.pezzi[4] & bit) != 0 { score += VAL_QUEEN; }
        else if (board.pezzi[5] & bit) != 0 { // RE
            score += KING_TABLE_MG[table_idx];
            score += check_king_safety(sq, board.pezzi[0] & my_pieces, c);
        }
    }
    score
}

fn get_adjacent_files_mask(file: usize) -> u64 {
    let mut mask = 0u64;
    if file > 0 { mask |= 0x0101010101010101 << (file - 1); }
    if file < 7 { mask |= 0x0101010101010101 << (file + 1); }
    mask
}

fn check_king_safety(king_sq: usize, my_pawns: u64, c: Colore) -> i32 {
    let mut safety = 0;
    let rank = king_sq / 8;
    
    // Verifichiamo se il re è arroccato o vicino alla base
    if (c == Colore::Bianco && rank <= 1) || (c == Colore::Nero && rank >= 6) {
        let shield_rank = if c == Colore::Bianco { 1 } else { 6 };
        let shield_mask = 0xFF << (shield_rank * 8);
        let pawns_in_front = (my_pawns & shield_mask).count_ones() as i32;
        
        // 10 punti per ogni pedone che fa da scudo
        safety += pawns_in_front * 10;
    }
    safety
}