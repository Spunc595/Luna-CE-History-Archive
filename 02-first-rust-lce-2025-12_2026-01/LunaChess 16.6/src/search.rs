// src/search.rs
use crate::board::{Scacchiera, Mossa, Pezzo, Colore};
use crate::evaluation;
use std::time::Instant;

pub const INFINITY: i32 = 1000000;

// Aggiungiamo il supporto per l'interruzione temporale
pub struct SearchInfo {
    pub start_time: Instant,
    pub time_limit_ms: u128,
    pub nodes: u64,
    pub stop: bool,
}

pub fn search(board: &mut Scacchiera, depth: i32, mut alpha: i32, beta: i32, info: &mut SearchInfo) -> i32 {
    // Controllo del tempo ogni 2048 nodi per non rallentare troppo
    if info.nodes % 2048 == 0 {
        if info.start_time.elapsed().as_millis() > info.time_limit_ms {
            info.stop = true;
        }
    }
    if info.stop { return 0; } // Ritorna un valore dummy, verra scartato
    
    info.nodes += 1;

    if depth <= 0 {
        return quiescence(board, alpha, beta, info);
    }

    let mut moves = board.genera_mosse();
    if moves.is_empty() {
        if board.re_in_scacco(board.turno) { return -INFINITY + 100; }
        return 0;
    }

    moves.sort_by_cached_key(|m| -punteggio_mossa(board, m));

    for mv in moves {
        if board.esegui_mossa(&mv, None) {
            let score = -search(board, depth - 1, -beta, -alpha, info);
            board.annulla_mossa(&mv, None, None);
            
            if info.stop { return 0; }

            if score >= beta { return beta; }
            if score > alpha { alpha = score; }
        }
    }
    alpha
}

pub fn quiescence(board: &mut Scacchiera, mut alpha: i32, beta: i32, info: &mut SearchInfo) -> i32 {
    info.nodes += 1;
    if (info.nodes & 2047) == 0 && info.start_time.elapsed().as_millis() > info.time_limit_ms {
        info.stop = true;
    }
    if info.stop { return 0; }

    let standby_score = evaluation::evaluate_classical(board);
    if standby_score >= beta { return beta; }
    if standby_score > alpha { alpha = standby_score; }

    let mut moves = board.genera_mosse();
    // Solo catture
    moves.retain(|m| board.get_piece_at(m.a() as usize).is_some() || m.flag() == 5);
    moves.sort_by_cached_key(|m| -punteggio_mossa(board, m));

    for mv in moves {
        if board.esegui_mossa(&mv, None) {
            let score = -quiescence(board, -beta, -alpha, info);
            board.annulla_mossa(&mv, None, None);
            
            if info.stop { return 0; }
            
            if score >= beta { return beta; }
            if score > alpha { alpha = score; }
        }
    }
    alpha
}

pub fn punteggio_mossa(board: &Scacchiera, mv: &Mossa) -> i32 {
    let to_sq = mv.a() as usize;
    let from_sq = mv.da() as usize;
    let pezzo = board.get_piece_at(from_sq).unwrap_or(Pezzo::Pedone);

    // 1. Catture (MVV-LVA)
    if let Some(vittima) = board.get_piece_at(to_sq) {
        return 20000 + (vittima.indice() as i32 * 10) - pezzo.indice() as i32;
    }

    // 2. SVILUPPO INTELLIGENTE (Anti-Na3)
    let is_white = board.turno == Colore::Bianco;
    let backrank_mask: u64 = if is_white { 0x00000000000000FF } else { 0xFF00000000000000 };
    
    if (1u64 << from_sq) & backrank_mask != 0 {
        if pezzo == Pezzo::Cavallo || pezzo == Pezzo::Alfiere {
            let file = to_sq % 8;
            // BONUS SOLO SE NON SIAMO SULLE COLONNE A (0) o H (7)
            if file > 0 && file < 7 {
                return 15000; 
            }
        }
    }

    // 3. PST
    let table = evaluation::pst::get_table(pezzo);
    let flip = if is_white { 56 } else { 0 };
    (table[to_sq ^ flip] - table[from_sq ^ flip]) as i32
}