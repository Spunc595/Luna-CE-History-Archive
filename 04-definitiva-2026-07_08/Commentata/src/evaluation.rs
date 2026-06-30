use crate::board::{Scacchiera, Colore, Pezzo};

// --- 1. VALORI DEL MATERIALE ---
// Valori base espressi in centipedoni (100 punti = 1 pedone)[cite: 4].
const MG_PAWN: i32 = 100;
const MG_KNIGHT: i32 = 320;
const MG_BISHOP: i32 = 330;
const MG_ROOK: i32 = 500;
const MG_QUEEN: i32 = 900;

// --- 2. PIECE-SQUARE TABLES (PST) ---
// Matrici da 64 elementi che mappano il valore posizionale di ogni pezzo[cite: 4].
// Orientate dal punto di vista del Bianco (dalla riga 1 alla riga 8)[cite: 4].

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
     -5,  0,  5,  5,  5,  5,  0, -5,
      0,  0,  5,  5,  5,  5,  0, -5,
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

/// Valuta staticamente la posizione corrente sulla scacchiera[cite: 4].
/// Calcola il punteggio materiale aggregato a quello posizionale (PST)[cite: 4].
/// Applica bonus per pedoni avanzati ed euristiche di finale (Mop-Up)[cite: 4].
/// Il punteggio finale è normalizzato per la ricerca Negamax (positivo per il giocatore attivo)[cite: 4].
pub fn evaluate(board: &Scacchiera) -> i32 {
    let mut score = 0;
    
    // Variabili per tracciare la posizione dei Re durante l'iterazione unica delle caselle[cite: 4].
    let mut white_king_sq: i32 = -1;
    let mut black_king_sq: i32 = -1;

    // Scansione sequenziale delle 64 caselle[cite: 4].
    for sq in 0..64 {
        if let Some((colore, pezzo)) = board.pezzo_e_colore_in(sq) {
            let is_white = colore == Colore::Bianco;
            let sq_idx = sq as usize;

            // Specchia verticalmente l'indice se il pezzo è Nero per simmetria rispetto al Bianco[cite: 4].
            let pst_idx = if is_white { sq_idx } else { sq_idx ^ 56 };
            let mut val = 0;

            match pezzo {
                Pezzo::Pedone => {
                    val = MG_PAWN + PAWN_PST[pst_idx];
                    
                    // --- BONUS PEDONI AVANZATI ---
                    let rank = sq_idx / 8; 
                    let advancement = if is_white { rank } else { 7 - rank };
                    
                    // Applica un premio incrementale geometrico se il pedone supera la metà campo[cite: 4].
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

            // Somma algebrica: i valori del bianco aumentano il punteggio, quelli del nero lo riducono[cite: 4].
            if is_white { score += val; } else { score -= val; }
        }
    }

    // --- 3. EURISTICA DI MOP-UP ---
    // Attiva la logica di spinta del re nemico verso i bordi in condizioni di netto vantaggio[cite: 4].
    if white_king_sq != -1 && black_king_sq != -1 {
        if score > 300 {
            score += evaluate_mop_up(white_king_sq, black_king_sq);
        } 
        else if score < -300 {
            score -= evaluate_mop_up(black_king_sq, white_king_sq);
        }
    }

    // Adattamento al paradigma Negamax: inverte il segno se il turno corrente appartiene al Nero[cite: 4].
    if board.turno == Colore::Bianco { score } else { -score }
}

/// Calcola il bonus di Mop-Up per costringere il Re in svantaggio verso l'angolo e 
/// avvicinare il proprio Re per assistere al matto[cite: 4].
fn evaluate_mop_up(winner_king_sq: i32, loser_king_sq: i32) -> i32 {
    let mut bonus = 0;

    let l_rank = loser_king_sq / 8;
    let l_file = loser_king_sq % 8;
    let w_rank = winner_king_sq / 8;
    let w_file = winner_king_sq % 8;

    // 1. Spinta del Re perdente verso il perimetro (distanza di Manhattan rispetto al centro geometrico)[cite: 4].
    let center_dist = (2 * l_rank - 7).abs() + (2 * l_file - 7).abs();
    bonus += center_dist * 25; 

    // 2. Coesione tra i Re: premia l'avvicinamento del Re dominante[cite: 4].
    let dist_kings = (w_rank - l_rank).abs() + (w_file - l_file).abs();
    bonus += (14 - dist_kings) * 20;

    bonus
}