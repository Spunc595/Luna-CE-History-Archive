use crate::board::{Scacchiera, Colore, Pezzo};

// --- 1. VALORI MATERIALE (MG = Mediogioco, EG = Finale) ---
const MG_WEIGHTS: [i32; 6] = [100, 320, 330, 500, 900, 0];
const EG_WEIGHTS: [i32; 6] = [120, 280, 300, 550, 950, 0];

// Pesi per calcolare la fase del gioco (Totale: 24)
// Knight=1, Bishop=1, Rook=2, Queen=4
const PHASE_WEIGHTS: [i32; 6] = [0, 1, 1, 2, 4, 0];

// --- 2. PIECE-SQUARE TABLES (MG e EG) ---

#[rustfmt::skip]
const PST_PAWN_MG: [i32; 64] = [
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
const PST_PAWN_EG: [i32; 64] = [
    0,  0,  0,  0,  0,  0,  0,  0,
    80, 80, 80, 80, 80, 80, 80, 80,
    50, 50, 50, 50, 50, 50, 50, 50,
    30, 30, 30, 30, 30, 30, 30, 30,
    20, 20, 20, 20, 20, 20, 20, 20,
    10, 10, 10, 10, 10, 10, 10, 10,
    0,  0,  0,  0,  0,  0,  0,  0,
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
const PST_KING_MG: [i32; 64] = [
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
const PST_KING_EG: [i32; 64] = [
    -50,-40,-30,-20,-20,-30,-40,-50,
    -30,-20,-10,  0,  0,-10,-20,-30,
    -30,-10, 20, 30, 30, 20,-10,-30,
    -30,-10, 30, 40, 40, 30,-10,-30,
    -30,-10, 30, 40, 40, 30,-10,-30,
    -30,-10, 20, 30, 30, 20,-10,-30,
    -30,-30,  0,  0,  0,  0,-30,-30,
    -50,-30,-30,-30,-30,-30,-30,-50
];

// --- FUNZIONE PRINCIPALE ---

pub fn evaluate(board: &Scacchiera) -> i32 {
    let mut mg_score = 0;
    let mut eg_score = 0;
    let mut game_phase = 0;

    let mut white_king_sq = 0;
    let mut black_king_sq = 0;

    for sq in 0..64 {
        if let Some((colore, pezzo)) = board.pezzo_e_colore_in(sq) {
            let is_white = colore == Colore::Bianco;
            let p_idx = pezzo.indice();
            let pst_idx = if is_white { sq as usize } else { (sq as usize) ^ 56 };

            // Accumuliamo la fase (materiale pesante rimasto)
            game_phase += PHASE_WEIGHTS[p_idx];

            let mut mg_val = MG_WEIGHTS[p_idx];
            let mut eg_val = EG_WEIGHTS[p_idx];

            match pezzo {
                Pezzo::Pedone => {
                    mg_val += PST_PAWN_MG[pst_idx];
                    eg_val += PST_PAWN_EG[pst_idx];
                    
                    // Bonus per pedoni passati
                    if is_passed_pawn(board, sq as usize, colore) {
                        let rank = if is_white { sq / 8 } else { 7 - (sq / 8) };
                        eg_val += (rank as i32 * rank as i32) * 10;
                    }
                },
                Pezzo::Cavallo => {
                    mg_val += PST_KNIGHT[pst_idx];
                    eg_val += PST_KNIGHT[pst_idx];
                },
                Pezzo::Re => {
                    if is_white { white_king_sq = sq as usize; }
                    else { black_king_sq = sq as usize; }
                    mg_val += PST_KING_MG[pst_idx];
                    eg_val += PST_KING_EG[pst_idx];
                },
                _ => {
                    // Per Alfiere, Torre, Regina usiamo valori materiale standard
                    // Puoi aggiungere PST specifiche anche per loro
                }
            }

            if is_white {
                mg_score += mg_val;
                eg_score += eg_val;
            } else {
                mg_score -= mg_val;
                eg_score -= eg_val;
            }
        }
    }

    // --- INTERPOLAZIONE TAPERED ---
    // La fase va da 0 (Endgame) a 24 (Opening)
    let phase = (game_phase.min(24) as i32);
    let score = ((mg_score * phase) + (eg_score * (24 - phase))) / 24;

    // --- MOP-UP (Solo in finale) ---
    let mut final_score = score;
    if phase < 12 { // Se siamo in finale
        if final_score > 300 {
            final_score += evaluate_mop_up(white_king_sq, black_king_sq);
        } else if final_score < -300 {
            final_score -= evaluate_mop_up(black_king_sq, white_king_sq);
        }
    }

    if board.turno == Colore::Bianco { final_score } else { -final_score }
}

// Rileva se un pedone è passato (nessun pedone nemico davanti o nelle colonne adiacenti)
fn is_passed_pawn(board: &Scacchiera, sq: usize, colore: Colore) -> bool {
    let file = sq % 8;
    let rank = sq / 8;
    let them = colore.opposto();
    let enemy_pawns = board.pezzi[Pezzo::Pedone.indice()] & board.colori[them.indice()];

    // Maschera semplificata: controlliamo le colonne file-1, file, file+1
    for f in (file as i32 - 1)..=(file as i32 + 1) {
        if f < 0 || f > 7 { continue; }
        
        // Controlliamo tutte le case davanti al pedone in quella colonna
        let start_r = if colore == Colore::Bianco { rank + 1 } else { 0 };
        let end_r = if colore == Colore::Bianco { 7 } else { rank - 1 };
        
        for r in start_r..=end_r {
            let check_sq = r * 8 + f as usize;
            if (enemy_pawns & (1 << check_sq)) != 0 {
                return false;
            }
        }
    }
    true
}

fn evaluate_mop_up(winner_king_sq: usize, loser_king_sq: usize) -> i32 {
    let mut bonus = 0;
    let l_rank = (loser_king_sq / 8) as i32;
    let l_file = (loser_king_sq % 8) as i32;
    let w_rank = (winner_king_sq / 8) as i32;
    let w_file = (winner_king_sq % 8) as i32;

    // Spingi il re nemico ai bordi
    let center_dist = (2 * l_rank - 7).abs() + (2 * l_file - 7).abs();
    bonus += center_dist * 20;

    // Avvicina il tuo re
    let dist_kings = (w_rank - l_rank).abs() + (w_file - l_file).abs();
    bonus += (14 - dist_kings) * 10;

    bonus
}