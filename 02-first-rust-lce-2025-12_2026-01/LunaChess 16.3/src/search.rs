// src/search.rs
use crate::board::{Scacchiera, Mossa, Pezzo, Colore};
use crate::evaluation;

pub const INFINITY: i32 = 1000000;

pub fn search(board: &mut Scacchiera, depth: i32, mut alpha: i32, beta: i32) -> i32 {
    if depth <= 0 {
        return quiescence(board, alpha, beta);
    }

    let mut moves = board.genera_mosse();
    
    if moves.is_empty() {
        if board.re_in_scacco(board.turno) {
            return -INFINITY + 100; // Penalità matto
        }
        return 0; // Patta
    }

    // --- MOVE ORDERING ---
    // Ordiniamo le mosse: catture e mosse PST-positive per prime
    moves.sort_by_cached_key(|m| -punteggio_mossa(board, m));

    for mv in moves {
        if board.esegui_mossa(&mv, None) {
            let score = -search(board, depth - 1, -beta, -alpha);
            board.annulla_mossa(&mv, None, None);

            if score >= beta {
                return beta; // Cut-off (Beta fail-high)
            }
            if score > alpha {
                alpha = score;
            }
        }
    }
    alpha
}

pub fn quiescence(board: &mut Scacchiera, mut alpha: i32, beta: i32) -> i32 {
    let standby_score = evaluation::evaluate_classical(board);
    
    if standby_score >= beta {
        return beta;
    }
    if standby_score > alpha {
        alpha = standby_score;
    }

    let mut moves = board.genera_mosse();
    // In Quiescence consideriamo solo catture e promozioni
    moves.retain(|m| board.get_piece_at(m.a() as usize).is_some() || m.flag() == 5 || m.flag() == 6);
    moves.sort_by_cached_key(|m| -punteggio_mossa(board, m));

    for mv in moves {
        if board.esegui_mossa(&mv, None) {
            let score = -quiescence(board, -beta, -alpha);
            board.annulla_mossa(&mv, None, None);

            if score >= beta {
                return beta;
            }
            if score > alpha {
                alpha = score;
            }
        }
    }
    alpha
}

/// Assegna un punteggio euristico per ordinare le mosse
fn punteggio_mossa(board: &Scacchiera, mv: &Mossa) -> i32 {
    let to_sq = mv.a() as usize;
    let from_sq = mv.da() as usize;
    let pezzo = board.get_piece_at(from_sq).unwrap_or(Pezzo::Pedone);

    // 1. Catture: Priorità assoluta (MVV-LVA)
    if let Some(vittima) = board.get_piece_at(to_sq) {
        return 10000 + (vittima.indice() as i32 * 10) - pezzo.indice() as i32;
    }

    // 2. Promozioni
    if mv.flag() == 6 { return 9000; }

    // 3. Mosse silenziose: Priorità in base al guadagno PST
    // Questo spinge Nf3 o e4 sopra a3/b3
    let table = evaluation::pst::get_table(pezzo);
    let flip = if board.turno == Colore::Bianco { 56 } else { 0 };
    
    let score_from = table[from_sq ^ flip];
    let score_to = table[to_sq ^ flip];
    
    // Bonus basato sulla tabella dei pesi definita in params.rs
    (score_to - score_from) as i32
}