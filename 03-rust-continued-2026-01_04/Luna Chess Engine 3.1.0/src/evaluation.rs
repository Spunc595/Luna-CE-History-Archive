use crate::board::{Scacchiera, Colore, Pezzo};
use crate::nnue::LunaNNUE;

// Valori materiale classici per fallback (usati solo se la NNUE non carica)
const MG_PAWN: i32 = 100;
const MG_KNIGHT: i32 = 320;
const MG_BISHOP: i32 = 330;
const MG_ROOK: i32 = 500;
const MG_QUEEN: i32 = 900;

pub fn evaluate(board: &Scacchiera, nnue: Option<&LunaNNUE>) -> i32 {
    // Correzione: usiamo 'nnue' che è il nome del parametro passato alla funzione
    if let Some(net) = nnue {
        return net.evaluate(board);
    }
    
    // Fallback: Valutazione materiale semplicistica se la rete non è disponibile
    evaluate_hce(board)
}

fn evaluate_hce(board: &Scacchiera) -> i32 {
    let mut score = 0;
    let us = board.turno;

    for sq in 0..64 {
        if let Some((colore, pezzo)) = board.pezzo_e_colore_in(sq) {
            let val = match pezzo {
                Pezzo::Pedone => MG_PAWN,
                Pezzo::Cavallo => MG_KNIGHT,
                Pezzo::Alfiere => MG_BISHOP,
                Pezzo::Torre => MG_ROOK,
                Pezzo::Regina => MG_QUEEN,
                Pezzo::Re => 0,
            };
            if colore == Colore::Bianco { score += val; } else { score -= val; }
        }
    }

    // Restituisce il punteggio relativo al giocatore di turno (Negamax)
    if us == Colore::Bianco { score } else { -score }
}