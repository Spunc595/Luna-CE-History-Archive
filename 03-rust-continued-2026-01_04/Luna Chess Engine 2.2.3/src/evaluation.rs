use crate::board::{Scacchiera, Colore, Pezzo};
use crate::nnue::LunaNNUE;

// --- PESI MATERIALE ---
const MG_PAWN: i32 = 82;    const EG_PAWN: i32 = 94;
const MG_KNIGHT: i32 = 337; const EG_KNIGHT: i32 = 281;
const MG_BISHOP: i32 = 365; const EG_BISHOP: i32 = 297;
const MG_ROOK: i32 = 477;   const EG_ROOK: i32 = 512;
const MG_QUEEN: i32 = 1025; const EG_QUEEN: i32 = 936;

// --- BONUS PEDONI PASSATI (La chiave per vincere i finali) ---
// Più il pedone avanza, più punti riceve.
// Indice: Rank 0..7 (dal punto di vista di chi muove)
const PASSED_PAWN_BONUS: [i32; 8] = [ 0, 10, 20, 40, 70, 130, 220, 0 ];

// --- TABELLE PST ---
// Modificate per forzare l'arrocco e lo sviluppo

#[rustfmt::skip]
const PST_PAWN: [[i32; 64]; 2] = [
    [ // MG
         0,  0,  0,  0,  0,  0,  0,  0,
        50, 50, 50, 50, 50, 50, 50, 50,
        10, 10, 20, 30, 30, 20, 10, 10,
         5,  5, 10, 25, 25, 10,  5,  5,
         0,  0,  0, 20, 20,  0,  0,  0,
         5, -5,-10,  0,  0,-10, -5,  5,
         5, 10, 10,-25,-25, 10, 10,  5, // Penalizziamo pedoni f/g/h bloccati in difesa se non servono
         0,  0,  0,  0,  0,  0,  0,  0
    ],
    [ // EG
         0,  0,  0,  0,  0,  0,  0,  0,
        90, 90, 90, 90, 90, 90, 90, 90, // Voglia di promozione altissima
        50, 50, 50, 50, 50, 50, 50, 50,
        30, 30, 30, 30, 30, 30, 30, 30,
        20, 20, 20, 20, 20, 20, 20, 20,
        10, 10, 10, 10, 10, 10, 10, 10,
        10, 10, 10, 10, 10, 10, 10, 10,
         0,  0,  0,  0,  0,  0,  0,  0
    ]
];

#[rustfmt::skip]
const PST_KNIGHT: [[i32; 64]; 2] = [
    [ // MG
        -50,-40,-30,-30,-30,-30,-40,-50,
        -40,-20,  0,  0,  0,  0,-20,-40,
        -30,  0, 10, 15, 15, 10,  0,-30,
        -30,  5, 15, 20, 20, 15,  5,-30,
        -30,  0, 15, 20, 20, 15,  0,-30,
        -30,  5, 10, 15, 15, 10,  5,-30,
        -40,-20,  0,  5,  5,  0,-20,-40,
        -50,-40,-30,-30,-30,-30,-40,-50
    ],
    [ // EG
        -50,-40,-30,-30,-30,-30,-40,-50,
        -40,-20,  0,  0,  0,  0,-20,-40,
        -30,  0, 10, 15, 15, 10,  0,-30,
        -30,  5, 15, 20, 20, 15,  5,-30,
        -30,  0, 15, 20, 20, 15,  0,-30,
        -30,  5, 10, 15, 15, 10,  5,-30,
        -40,-20,  0,  5,  5,  0,-20,-40,
        -50,-40,-30,-30,-30,-30,-40,-50
    ]
];

#[rustfmt::skip]
const PST_BISHOP: [[i32; 64]; 2] = [
    [ // MG
        -20,-10,-10,-10,-10,-10,-10,-20,
        -10,  0,  0,  0,  0,  0,  0,-10,
        -10,  0,  5, 10, 10,  5,  0,-10,
        -10,  5,  5, 10, 10,  5,  5,-10,
        -10,  0, 10, 10, 10, 10,  0,-10,
        -10, 10, 10, 10, 10, 10, 10,-10,
        -10,  5,  0,  0,  0,  0,  5,-10,
        -20,-10,-10,-10,-10,-10,-10,-20
    ],
    [ // EG
        -20,-10,-10,-10,-10,-10,-10,-20,
        -10,  0,  0,  0,  0,  0,  0,-10,
        -10,  0,  5, 10, 10,  5,  0,-10,
        -10,  5,  5, 10, 10,  5,  5,-10,
        -10,  0, 10, 10, 10, 10,  0,-10,
        -10, 10, 10, 10, 10, 10, 10,-10,
        -10,  5,  0,  0,  0,  0,  5,-10,
        -20,-10,-10,-10,-10,-10,-10,-20
    ]
];

#[rustfmt::skip]
const PST_ROOK: [[i32; 64]; 2] = [
    [ // MG
         0,  0,  0,  0,  0,  0,  0,  0,
         5, 10, 10, 10, 10, 10, 10,  5,
        -5,  0,  0,  0,  0,  0,  0, -5,
        -5,  0,  0,  0,  0,  0,  0, -5,
        -5,  0,  0,  0,  0,  0,  0, -5,
        -5,  0,  0,  0,  0,  0,  0, -5,
        -5,  0,  0,  0,  0,  0,  0, -5,
         0,  0,  0,  5,  5,  0,  0,  0
    ],
    [ // EG
         0,  0,  0,  0,  0,  0,  0,  0,
         5, 10, 10, 10, 10, 10, 10,  5,
        -5,  0,  0,  0,  0,  0,  0, -5,
        -5,  0,  0,  0,  0,  0,  0, -5,
        -5,  0,  0,  0,  0,  0,  0, -5,
        -5,  0,  0,  0,  0,  0,  0, -5,
        -5,  0,  0,  0,  0,  0,  0, -5,
         0,  0,  0,  5,  5,  0,  0,  0
    ]
];

#[rustfmt::skip]
const PST_QUEEN: [[i32; 64]; 2] = [
    [ // MG
        -20,-10,-10, -5, -5,-10,-10,-20,
        -10,  0,  0,  0,  0,  0,  0,-10,
        -10,  0,  5,  5,  5,  5,  0,-10,
         -5,  0,  5,  5,  5,  5,  0, -5,
          0,  0,  5,  5,  5,  5,  0, -5,
        -10,  5,  5,  5,  5,  5,  0,-10,
        -10,  0,  5,  0,  0,  0,  0,-10,
        -20,-10,-10, -5, -5,-10,-10,-20
    ],
    [ // EG
        -20,-10,-10, -5, -5,-10,-10,-20,
        -10,  0,  0,  0,  0,  0,  0,-10,
        -10,  0,  5,  5,  5,  5,  0,-10,
         -5,  0,  5,  5,  5,  5,  0, -5,
          0,  0,  5,  5,  5,  5,  0, -5,
        -10,  5,  5,  5,  5,  5,  0,-10,
        -10,  0,  5,  0,  0,  0,  0,-10,
        -20,-10,-10, -5, -5,-10,-10,-20
    ]
];

#[rustfmt::skip]
const PST_KING: [[i32; 64]; 2] = [
    [ // MG - MODIFICATO PER FORZARE ARROCCO
      // Il centro (d1, e1) ora è pesantemente penalizzato (-50)
      // Gli angoli (g1, b1) sono premiati (+60)
        -30,-40,-40,-50,-50,-40,-40,-30,
        -30,-40,-40,-50,-50,-40,-40,-30,
        -30,-40,-40,-50,-50,-40,-40,-30,
        -30,-40,-40,-50,-50,-40,-40,-30,
        -20,-30,-30,-40,-40,-30,-30,-20,
        -10,-20,-20,-20,-20,-20,-20,-10,
         20, 20,  0,  0,  0,  0, 20, 20,
         20, 50, 20,-30,-30, 20, 60, 20 // Rank 1: Premia G1/B1, punisce E1/D1
    ],
    [ // EG - Re Attivo
        -50,-40,-30,-20,-20,-30,-40,-50,
        -30,-20,-10,  0,  0,-10,-20,-30,
        -30,-10, 20, 30, 30, 20,-10,-30,
        -30,-10, 30, 40, 40, 30,-10,-30,
        -30,-10, 30, 40, 40, 30,-10,-30,
        -30,-10, 20, 30, 30, 20,-10,-30,
        -30,-30,  0,  0,  0,  0,-30,-30,
        -50,-30,-30,-30,-30,-30,-30,-50
    ]
];

pub fn evaluate(board: &Scacchiera, nnue: Option<&LunaNNUE>) -> i32 {
    // Se c'è la rete neurale, usala (ma ora stiamo testando HCE)
    if let Some(net) = nnue {
        return net.evaluate(board);
    }
    evaluate_hce(board)
}

fn evaluate_hce(board: &Scacchiera) -> i32 {
    let mut mg_score = [0; 2]; 
    let mut eg_score = [0; 2]; 
    let mut game_phase = 0;

    let white_pawns = board.pezzi[Pezzo::Pedone.indice()] & board.colori[Colore::Bianco.indice()];
    let black_pawns = board.pezzi[Pezzo::Pedone.indice()] & board.colori[Colore::Nero.indice()];

    for sq in 0..64 {
        if let Some((colore, pezzo)) = board.pezzo_e_colore_in(sq) {
            let c = colore.indice();
            
            // Tabella flippata per il Nero
            let table_sq = if colore == Colore::Bianco { sq } else { sq ^ 56 };

            let (mut mg_val, mut eg_val, phase_w) = match pezzo {
                Pezzo::Pedone => (MG_PAWN + PST_PAWN[0][table_sq], EG_PAWN + PST_PAWN[1][table_sq], 0),
                Pezzo::Cavallo => (MG_KNIGHT + PST_KNIGHT[0][table_sq], EG_KNIGHT + PST_KNIGHT[1][table_sq], 1),
                Pezzo::Alfiere => (MG_BISHOP + PST_BISHOP[0][table_sq], EG_BISHOP + PST_BISHOP[1][table_sq], 1),
                Pezzo::Torre => (MG_ROOK + PST_ROOK[0][table_sq], EG_ROOK + PST_ROOK[1][table_sq], 2),
                Pezzo::Regina => (MG_QUEEN + PST_QUEEN[0][table_sq], EG_QUEEN + PST_QUEEN[1][table_sq], 4),
                Pezzo::Re => (MG_KING + PST_KING[0][table_sq], EG_KING + PST_KING[1][table_sq], 0),
            };

            // --- LOGICA PEDONI PASSATI ---
            if pezzo == Pezzo::Pedone {
                if is_passed(sq, colore, white_pawns, black_pawns) {
                    let rank = if colore == Colore::Bianco { sq / 8 } else { 7 - (sq / 8) };
                    // Bonus enorme in endgame per i pedoni che corrono
                    let bonus = PASSED_PAWN_BONUS[rank];
                    mg_val += bonus / 2;
                    eg_val += bonus;
                }
            }

            mg_score[c] += mg_val;
            eg_score[c] += eg_val;
            game_phase += phase_w;
        }
    }

    let phase = game_phase.min(24);
    let white_eval = (mg_score[0] * phase + eg_score[0] * (24 - phase)) / 24;
    let black_eval = (mg_score[1] * phase + eg_score[1] * (24 - phase)) / 24;

    let score = white_eval - black_eval;

    if board.turno == Colore::Bianco { score } else { -score }
}

// Controlla se un pedone è "Passato" (nessun pedone nemico davanti o sulle colonne adiacenti)
fn is_passed(sq: usize, color: Colore, white_pawns: u64, black_pawns: u64) -> bool {
    let file = sq % 8;
    let rank = sq / 8;
    
    // Maschera dei pedoni nemici
    let enemy_pawns = if color == Colore::Bianco { black_pawns } else { white_pawns };

    // Crea maschera "davanti"
    // Bitboard di colonne: File F, F-1, F+1
    let mut file_mask = 0x0101010101010101u64 << file;
    if file > 0 { file_mask |= file_mask >> 1; }
    if file < 7 { file_mask |= file_mask << 1; }

    // Rimuovi tutto ciò che sta dietro o sulla stessa riga
    let forward_mask = if color == Colore::Bianco {
        // Mantieni solo rank > corrente
        !((1u64 << ((rank + 1) * 8)) - 1)
    } else {
        // Mantieni solo rank < corrente (per il nero che va in giù, indici più bassi)
        (1u64 << (rank * 8)) - 1
    };

    // Se non ci sono pedoni nemici nel cono frontale, è passato!
    (enemy_pawns & file_mask & forward_mask) == 0
}