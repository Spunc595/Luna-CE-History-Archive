use crate::board::{Scacchiera, Colore, Pezzo};

// Valori materiali standard (bilanciati)
const MATERIAL_VAL: [i32; 6] = [
    100,   // Pedone
    320,   // Cavallo
    330,   // Alfiere
    500,   // Torre
    900,   // Regina
    20000  // Re
];

// Tabelle Posizionali Semplificate (PST)
// Orientate per il BIANCO. Il nero userà l'indice specchiato (sq ^ 56).

#[rustfmt::skip]
const PST_PAWN: [i32; 64] = [
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
const PST_KNIGHT: [i32; 64] = [
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
const PST_BISHOP: [i32; 64] = [
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
const PST_ROOK: [i32; 64] = [
    0,  0,  0,  0,  0,  0,  0,  0,
    5, 10, 10, 10, 10, 10, 10,  5,
   -5,  0,  0,  0,  0,  0,  0, -5,
   -5,  0,  0,  0,  0,  0,  0, -5,
   -5,  0,  0,  0,  0,  0,  0, -5,
   -5,  0,  0,  0,  0,  0,  0, -5,
    5, 10, 10, 10, 10, 10, 10,  5,
    0,  0,  0,  0,  0,  0,  0,  0
];

#[rustfmt::skip]
const PST_QUEEN: [i32; 64] = [
   -20,-10,-10, -5, -5,-10,-10,-20,
   -10,  0,  0,  0,  0,  0,  0,-10,
   -10,  0,  5,  5,  5,  5,  0,-10,
    -5,  0,  5,  5,  5,  5,  0, -5,
     0,  0,  5,  5,  5,  5,  0, -5,
   -10,  5,  5,  5,  5,  5,  0,-10,
   -10,  0,  5,  0,  0,  0,  0,-10,
   -20,-10,-10, -5, -5,-10,-10,-20
];

#[rustfmt::skip]
const PST_KING: [i32; 64] = [
   -30,-40,-40,-50,-50,-40,-40,-30,
   -30,-40,-40,-50,-50,-40,-40,-30,
   -30,-40,-40,-50,-50,-40,-40,-30,
   -30,-40,-40,-50,-50,-40,-40,-30,
   -20,-30,-30,-40,-40,-30,-30,-20,
   -10,-20,-20,-20,-20,-20,-20,-10,
    20, 20,  0,  0,  0,  0, 20, 20,
    20, 30, 10,  0,  0, 10, 30, 20
];

pub fn evaluate_classical(board: &Scacchiera) -> i32 {
    let mut score_mg = 0;

    for p in 0..6 { // Include anche il Re (indice 5)
        let piece = match p {
            0 => Pezzo::Pedone, 1 => Pezzo::Cavallo, 2 => Pezzo::Alfiere,
            3 => Pezzo::Torre, 4 => Pezzo::Regina, _ => Pezzo::Re
        };
        
        let val = MATERIAL_VAL[p];
        let pst = get_pst(piece);

        // Somma pezzi Bianchi
        let mut w_bb = board.pezzi[p] & board.colori[Colore::Bianco.indice()];
        while w_bb != 0 {
            let sq = w_bb.trailing_zeros() as usize;
            score_mg += val + pst[sq];
            w_bb &= w_bb - 1;
        }

        // Sottrai pezzi Neri
        let mut b_bb = board.pezzi[p] & board.colori[Colore::Nero.indice()];
        while b_bb != 0 {
            let sq = b_bb.trailing_zeros() as usize;
            // Usa sq ^ 56 per ribaltare la scacchiera per il nero
            score_mg -= val + pst[sq ^ 56]; 
            b_bb &= b_bb - 1;
        }
    }

    if board.turno == Colore::Bianco {
        score_mg
    } else {
        -score_mg
    }
}

fn get_pst(p: Pezzo) -> &'static [i32; 64] {
    match p {
        Pezzo::Pedone => &PST_PAWN,
        Pezzo::Cavallo => &PST_KNIGHT,
        Pezzo::Alfiere => &PST_BISHOP,
        Pezzo::Torre => &PST_ROOK,
        Pezzo::Regina => &PST_QUEEN,
        Pezzo::Re => &PST_KING,
    }
}