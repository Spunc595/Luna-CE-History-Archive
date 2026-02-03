use crate::board::{Scacchiera, Colore, Pezzo};
use crate::nnue::LunaNNUE;

// --- PESI MATERIALE ---
// Valori in centipawns.
const MG_PAWN: i32 = 82;    const EG_PAWN: i32 = 94;
const MG_KNIGHT: i32 = 337; const EG_KNIGHT: i32 = 281;
const MG_BISHOP: i32 = 365; const EG_BISHOP: i32 = 297;
const MG_ROOK: i32 = 477;   const EG_ROOK: i32 = 512;
const MG_QUEEN: i32 = 1025; const EG_QUEEN: i32 = 936;
const MG_KING: i32 = 0;     const EG_KING: i32 = 0; // Il Re non ha valore materiale di scambio

// --- BONUS PEDONI PASSATI ---
// Più il pedone avanza, più punti riceve.
// Indice: Rank 0..7 (dal punto di vista di chi muove)
const PASSED_PAWN_BONUS: [i32; 8] = [ 0, 10, 30, 50, 90, 160, 240, 0 ];

// --- TABELLE PST (Tapered) ---

#[rustfmt::skip]
const PST_PAWN: [[i32; 64]; 2] = [
    [ // MG
         0,  0,  0,  0,  0,  0,  0,  0,
        50, 50, 50, 50, 50, 50, 50, 50,
        10, 10, 20, 30, 30, 20, 10, 10,
         5,  5, 10, 25, 25, 10,  5,  5,
         0,  0,  0, 20, 20,  0,  0,  0,
         5, -5,-10,  0,  0,-10, -5,  5,
         5, 10, 10,-25,-25, 10, 10,  5,
         0,  0,  0,  0,  0,  0,  0,  0
    ],
    [ // EG
         0,  0,  0,  0,  0,  0,  0,  0,
        90, 90, 90, 90, 90, 90, 90, 90,
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
    [ // MG - Sicurezza Arrocco
        -30,-40,-40,-50,-50,-40,-40,-30,
        -30,-40,-40,-50,-50,-40,-40,-30,
        -30,-40,-40,-50,-50,-40,-40,-30,
        -30,-40,-40,-50,-50,-40,-40,-30,
        -20,-30,-30,-40,-40,-30,-30,-20,
        -10,-20,-20,-20,-20,-20,-20,-10,
         20, 20,  0,  0,  0,  0, 20, 20,
         20, 50, 20,-30,-30, 20, 60, 20 
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
    if let Some(net) = nnue {
        return net.evaluate(board);
    }
    evaluate_hce(board)
}

fn evaluate_hce(board: &Scacchiera) -> i32 {
    let mut mg_score = [0; 2]; 
    let mut eg_score = [0; 2]; 
    let mut game_phase = 0;

    let us = board.turno;
    let them = us.opposto();

    let white_pawns = board.pezzi[Pezzo::Pedone.indice()] & board.colori[Colore::Bianco.indice()];
    let black_pawns = board.pezzi[Pezzo::Pedone.indice()] & board.colori[Colore::Nero.indice()];

    for sq in 0..64 {
        if let Some((colore, pezzo)) = board.pezzo_e_colore_in(sq) {
            let c = colore.indice();
            let table_sq = if colore == Colore::Bianco { sq } else { sq ^ 56 };

            let (mut mg_val, mut eg_val, phase_w) = match pezzo {
                Pezzo::Pedone => (MG_PAWN + PST_PAWN[0][table_sq], EG_PAWN + PST_PAWN[1][table_sq], 0),
                Pezzo::Cavallo => (MG_KNIGHT + PST_KNIGHT[0][table_sq], EG_KNIGHT + PST_KNIGHT[1][table_sq], 1),
                Pezzo::Alfiere => (MG_BISHOP + PST_BISHOP[0][table_sq], EG_BISHOP + PST_BISHOP[1][table_sq], 1),
                Pezzo::Torre => (MG_ROOK + PST_ROOK[0][table_sq], EG_ROOK + PST_ROOK[1][table_sq], 2),
                Pezzo::Regina => (MG_QUEEN + PST_QUEEN[0][table_sq], EG_QUEEN + PST_QUEEN[1][table_sq], 4),
                Pezzo::Re => (MG_KING + PST_KING[0][table_sq], EG_KING + PST_KING[1][table_sq], 0),
            };

            // Bonus Pedoni Passati
            if pezzo == Pezzo::Pedone {
                if is_passed(sq, colore, white_pawns, black_pawns) {
                    let rank = if colore == Colore::Bianco { sq / 8 } else { 7 - (sq / 8) };
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

    // Calcolo Tapered Evaluation
    let phase = game_phase.min(24);
    let mut white_eval = (mg_score[0] * phase + eg_score[0] * (24 - phase)) / 24;
    let mut black_eval = (mg_score[1] * phase + eg_score[1] * (24 - phase)) / 24;

    // --- MOP-UP EVALUATION (Aiuto per dare matto) ---
    // Se stiamo vincendo nettamente (più materiale), aiutiamo il motore a chiudere
    // spingendo il Re avversario agli angoli.
    let score_material = white_eval - black_eval;
    
    // Posizioni dei Re
    let w_king_sq = (board.pezzi[Pezzo::Re.indice()] & board.colori[Colore::Bianco.indice()]).trailing_zeros() as i32;
    let b_king_sq = (board.pezzi[Pezzo::Re.indice()] & board.colori[Colore::Nero.indice()]).trailing_zeros() as i32;
    
    // Se il Bianco sta vincendo (+2 pedoni circa)
    if score_material > 200 {
        white_eval += mop_up_bonus(w_king_sq, b_king_sq);
    }
    // Se il Nero sta vincendo
    else if score_material < -200 {
        black_eval += mop_up_bonus(b_king_sq, w_king_sq);
    }

    let final_score = white_eval - black_eval;
    if board.turno == Colore::Bianco { final_score } else { -final_score }
}

fn is_passed(sq: usize, color: Colore, white_pawns: u64, black_pawns: u64) -> bool {
    let file = sq % 8;
    let rank = sq / 8;
    let enemy_pawns = if color == Colore::Bianco { black_pawns } else { white_pawns };
    
    let mut file_mask = 0x0101010101010101u64 << file;
    if file > 0 { file_mask |= file_mask >> 1; }
    if file < 7 { file_mask |= file_mask << 1; }

    let forward_mask = if color == Colore::Bianco {
        !((1u64 << ((rank + 1) * 8)) - 1)
    } else {
        (1u64 << (rank * 8)) - 1
    };

    (enemy_pawns & file_mask & forward_mask) == 0
}

// Incoraggia il re vincente ad avvicinarsi e spingere il re perdente ai bordi
fn mop_up_bonus(winning_king: i32, losing_king: i32) -> i32 {
    let losing_rank = losing_king / 8;
    let losing_file = losing_king % 8;

    // Distanza dal centro (Manhattan distance: 0 al centro, 6/7 agli angoli)
    // Vogliamo MAXIMIZZARE questo per il re perdente (spingerlo via)
    let center_dist_rank = (2 * losing_rank - 7).abs();
    let center_dist_file = (2 * losing_file - 7).abs();
    let center_distance = 10 * (center_dist_rank + center_dist_file);

    // Distanza tra i due Re (vogliamo MINIMIZZARE questo per avvicinarci)
    let dist_kings = (winning_king / 8 - losing_rank).abs() + (winning_king % 8 - losing_file).abs();
    let close_bonus = 14 - (2 * dist_kings); // Più siamo vicini, meglio è

    center_distance + close_bonus
}