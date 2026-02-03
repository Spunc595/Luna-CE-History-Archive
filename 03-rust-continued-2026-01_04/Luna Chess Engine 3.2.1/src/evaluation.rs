use crate::board::{Scacchiera, Colore, Pezzo};

// --- 1. VALORI MATERIALE ---
const MG_PAWN: i32 = 100;
const MG_KNIGHT: i32 = 320;
const MG_BISHOP: i32 = 330;
const MG_ROOK: i32 = 500;
const MG_QUEEN: i32 = 900;

// --- 2. PIECE-SQUARE TABLES (PST) ---
// Bonus posizionali: dove piace stare ai pezzi (visti dal BIANCO)

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

// --- FUNZIONE PRINCIPALE ---
// Nota: Ho rimosso l'argomento 'nnue' per compatibilità con le modifiche fatte in main.rs e search.rs
pub fn evaluate(board: &Scacchiera) -> i32 {
    let mut score = 0;
    
    // Per il Mop-Up ci servono le posizioni dei re.
    // Li troviamo durante il ciclo principale per efficienza.
    let mut white_king_sq: i32 = -1;
    let mut black_king_sq: i32 = -1;

    // Ciclo su tutte le caselle (0..64) come nel tuo snippet originale
    for sq in 0..64 {
        if let Some((colore, pezzo)) = board.pezzo_e_colore_in(sq) {
            let is_white = colore == Colore::Bianco;
            let sq_idx = sq as usize;

            // 1. Calcolo Indice PST
            // Se è bianco usiamo l'indice normale. Se è nero, specchiamo verticalmente (^ 56).
            let pst_idx = if is_white { sq_idx } else { sq_idx ^ 56 };

            let mut val = 0;

            match pezzo {
                Pezzo::Pedone => {
                    val = MG_PAWN + PAWN_PST[pst_idx];
                    
                    // --- BONUS PEDONI AVANZATI (TURBO PROMOZIONE) ---
                    let rank = sq_idx / 8; // Riga 0-7
                    let advancement = if is_white { rank } else { 7 - rank };
                    
                    // Se il pedone è avanzato (riga 4, 5, 6), diamo punti esponenziali!
                    if advancement >= 4 {
                        val += (advancement as i32).pow(2) * 5; 
                    }
                },
                Pezzo::Cavallo => { val = MG_KNIGHT + KNIGHT_PST[pst_idx]; },
                Pezzo::Alfiere => { val = MG_BISHOP + BISHOP_PST[pst_idx]; },
                Pezzo::Torre   => { val = MG_ROOK   + ROOK_PST[pst_idx]; },
                Pezzo::Regina  => { val = MG_QUEEN  + QUEEN_PST[pst_idx]; },
                Pezzo::Re => {
                    // Salviamo la posizione del re per dopo
                    if is_white { white_king_sq = sq as i32; } 
                    else { black_king_sq = sq as i32; }
                    
                    val = KING_PST[pst_idx]; 
                },
            };

            if is_white { score += val; } else { score -= val; }
        }
    }

    // --- 3. MOP-UP (Solo se abbiamo trovato i Re) ---
    // Serve a dare scacco matto quando si ha un vantaggio netto
    if white_king_sq != -1 && black_king_sq != -1 {
        // Se il punteggio è alto positivo (Bianco vince)
        if score > 300 {
            score += evaluate_mop_up(white_king_sq, black_king_sq);
        } 
        // Se il punteggio è basso negativo (Nero vince)
        else if score < -300 {
            // Nota: passiamo prima il vincitore (Nero), poi il perdente (Bianco)
            score -= evaluate_mop_up(black_king_sq, white_king_sq);
        }
    }

    // Restituisce il punteggio relativo al giocatore di turno (Negamax)
    if board.turno == Colore::Bianco { score } else { -score }
}

// Logica per spingere il Re avversario all'angolo
fn evaluate_mop_up(winner_king_sq: i32, loser_king_sq: i32) -> i32 {
    let mut bonus = 0;

    let l_rank = loser_king_sq / 8;
    let l_file = loser_king_sq % 8;
    let w_rank = winner_king_sq / 8;
    let w_file = winner_king_sq % 8;

    // 1. Spingi il re nemico ai bordi (Manhattan distance dal centro)
    // Formula per calcolare distanza dal centro (3.5) usando interi:
    let center_dist = (2 * l_rank - 7).abs() + (2 * l_file - 7).abs();
    
    // Moltiplicatore aggressivo
    bonus += center_dist * 25; 

    // 2. Avvicina il tuo re per aiutare nel matto
    let dist_kings = (w_rank - l_rank).abs() + (w_file - l_file).abs();
    
    // Più siamo vicini, meglio è (14 è la distanza max possibile)
    bonus += (14 - dist_kings) * 20;

    bonus
}