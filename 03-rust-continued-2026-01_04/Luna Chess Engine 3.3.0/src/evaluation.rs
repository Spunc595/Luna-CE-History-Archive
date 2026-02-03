use crate::board::{Scacchiera, Colore, Pezzo, Bitboard};

// --- 1. VALORI MATERIALE ---
const MG_PAWN: i32 = 100;
const MG_KNIGHT: i32 = 320;
const MG_BISHOP: i32 = 330;
const MG_ROOK: i32 = 500;
const MG_QUEEN: i32 = 900;

// --- 2. COSTANTI SICUREZZA RE ---
const RE_ESPOSTO_CENTRO: i32 = 50;   // Penalità se il Re è al centro con Regine in gioco
const RE_SCUDO_MANCANTE: i32 = 30;   // Penalità per ogni pedone mancante davanti al Re arroccato
const RE_NO_ARROCCO: i32 = 40;       // Penalità se ha perso il diritto d'arrocco senza essere al sicuro

// --- 3. PIECE-SQUARE TABLES (PST) ---
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

    // Controlliamo se ci sono regine (serve per pesare la sicurezza del re)
    let white_queen_present = (board.pezzi[Pezzo::Regina.indice()] & board.colori[Colore::Bianco.indice()]) != 0;
    let black_queen_present = (board.pezzi[Pezzo::Regina.indice()] & board.colori[Colore::Nero.indice()]) != 0;

    for sq in 0..64 {
        if let Some((colore, pezzo)) = board.pezzo_e_colore_in(sq) {
            let is_white = colore == Colore::Bianco;
            let sq_idx = sq as usize;
            let pst_idx = if is_white { sq_idx } else { sq_idx ^ 56 };

            let mut val = 0;

            match pezzo {
                Pezzo::Pedone => {
                    val = MG_PAWN + PAWN_PST[pst_idx];
                    let rank = sq_idx / 8;
                    let advancement = if is_white { rank } else { 7 - rank };
                    if advancement >= 4 {
                        val += (advancement as i32).pow(2) * 5; 
                    }
                },
                Pezzo::Cavallo => { val = MG_KNIGHT + KNIGHT_PST[pst_idx]; },
                Pezzo::Alfiere => { val = MG_BISHOP + BISHOP_PST[pst_idx]; },
                Pezzo::Torre   => { val = MG_ROOK   + ROOK_PST[pst_idx]; },
                Pezzo::Regina  => { val = MG_QUEEN  + QUEEN_PST[pst_idx]; },
                Pezzo::Re => {
                    if is_white { white_king_sq = sq as i32; } 
                    else { black_king_sq = sq as i32; }
                    val = KING_PST[pst_idx]; 
                },
            };

            if is_white { score += val; } else { score -= val; }
        }
    }

    // --- AGGIUNTA: SICUREZZA DEL RE ---
    if white_king_sq != -1 {
        score += evaluate_king_safety(board, Colore::Bianco, white_king_sq as usize, black_queen_present);
    }
    if black_king_sq != -1 {
        score -= evaluate_king_safety(board, Colore::Nero, black_king_sq as usize, white_queen_present);
    }

    // --- MOP-UP ---
    if white_king_sq != -1 && black_king_sq != -1 {
        if score > 300 {
            score += evaluate_mop_up(white_king_sq, black_king_sq);
        } 
        else if score < -300 {
            score -= evaluate_mop_up(black_king_sq, white_king_sq);
        }
    }

    if board.turno == Colore::Bianco { score } else { -score }
}

fn evaluate_king_safety(board: &Scacchiera, colore: Colore, king_sq: usize, opponent_has_queen: bool) -> i32 {
    let mut safety_score = 0;

    // Se non ci sono regine avversarie, la sicurezza del re è meno prioritaria
    if !opponent_has_queen { return 0; }

    let file = king_sq % 8;
    let rank = king_sq / 8;
    let is_white = colore == Colore::Bianco;

    // 1. Penalità per Re al Centro (Colonne C, D, E, F)
    if file >= 2 && file <= 5 {
        safety_score -= RE_ESPOSTO_CENTRO;
    }

    // 2. Pawn Shield (Scudo di pedoni)
    // Se il Re è arroccato (lato re o lato regina), controlliamo i pedoni davanti
    if (is_white && rank <= 1) || (!is_white && rank >= 6) {
        let shield_rank = if is_white { rank + 1 } else { rank - 1 };
        
        // Controlliamo le 3 colonne attorno al re
        for f_offset in -1..=1 {
            let f = file as i32 + f_offset;
            if f >= 0 && f <= 7 {
                let check_sq = shield_rank * 8 + f as usize;
                let mut has_pawn = false;
                if let Some((c, p)) = board.pezzo_e_colore_in(check_sq) {
                    if c == colore && p == Pezzo::Pedone { has_pawn = true; }
                }
                
                if !has_pawn {
                    safety_score -= RE_SCUDO_MANCANTE;
                }
            }
        }
    }

    // 3. Penalità se non può più arroccare e non è al sicuro
    let castling_rights = board.diritti_arrocco;
    let can_castle = if is_white { (castling_rights & 3) != 0 } else { (castling_rights & 12) != 0 };
    if !can_castle && file >= 2 && file <= 5 {
        safety_score -= RE_NO_ARROCCO;
    }

    safety_score
}

fn evaluate_mop_up(winner_king_sq: i32, loser_king_sq: i32) -> i32 {
    let mut bonus = 0;
    let l_rank = loser_king_sq / 8;
    let l_file = loser_king_sq % 8;
    let w_rank = winner_king_sq / 8;
    let w_file = winner_king_sq % 8;

    let center_dist = (2 * l_rank - 7).abs() + (2 * l_file - 7).abs();
    bonus += center_dist * 25; 

    let dist_kings = (w_rank - l_rank).abs() + (w_file - l_file).abs();
    bonus += (14 - dist_kings) * 20;

    bonus
}