use crate::board::{Scacchiera, Colore};
use crate::nnue::LunaNNUE;

/// Valuta la posizione.
/// Priorità: 1. NNUE (se disponibile) -> 2. Valutazione Classica (PST + Materiale)
pub fn evaluate(board: &Scacchiera, nnue: Option<&LunaNNUE>) -> i32 {
    // 1. Tenta NNUE
    if let Some(net) = nnue {
        // Se la rete è caricata correttamente, usala
        return net.evaluate(board);
    }

    // 2. Fallback: Valutazione Classica
    // board.pst_val viene aggiornato incrementalmente in board.rs (make/unmake)
    let eval = board.pst_val;

    // L'eval deve essere relativa al lato che muove
    if board.turno == Colore::Bianco {
        eval
    } else {
        -eval
    }
}