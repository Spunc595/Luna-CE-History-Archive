use crate::board::{Scacchiera, Mossa, Pezzo, Colore};
use crate::evaluation;

pub const INFINITY: i32 = 1000000;

pub fn search(board: &mut Scacchiera, depth: i32, mut alpha: i32, beta: i32) -> i32 {
    if depth <= 0 {
        return quiescence(board, alpha, beta);
    }

    let mut moves = board.genera_mosse();
    if moves.is_empty() {
        if board.re_in_scacco(board.turno) { return -INFINITY + 100; }
        return 0;
    }

    // Ordinamento mosse per efficienza Alpha-Beta
    moves.sort_by_cached_key(|m| -punteggio_mossa(board, m));

    for mv in moves {
        if board.esegui_mossa(&mv, None) {
            let score = -search(board, depth - 1, -beta, -alpha);
            board.annulla_mossa(&mv, None, None);

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

    // 1. Catture (MVV-LVA) - Massimo punteggio
    if let Some(vittima) = board.get_piece_at(to_sq) {
        return 20000 + (vittima.indice() as i32 * 10) - pezzo.indice() as i32;
    }

    // 2. SVILUPPO CENTRALE (Forzatura per Luna)
    // Se un Cavallo o Alfiere si muove dalla casa iniziale, riceve un bonus enorme
    let is_white = board.turno == Colore::Bianco;
    let backrank_mask: u64 = if is_white { 0x00000000000000FF } else { 0xFF00000000000000 };
    if (1u64 << from_sq) & backrank_mask != 0 {
        if pezzo == Pezzo::Cavallo || pezzo == Pezzo::Alfiere {
            return 15000; // Priorità assoluta dopo le catture
        }
    }

    // 3. PST (Bonus posizionale normale)
    let table = evaluation::pst::get_table(pezzo);
    let flip = if is_white { 56 } else { 0 };
    (table[to_sq ^ flip] - table[from_sq ^ flip]) as i32
}

pub fn quiescence(board: &mut Scacchiera, mut alpha: i32, beta: i32) -> i32 {
    let standby_score = evaluation::evaluate_classical(board);
    if standby_score >= beta { return beta; }
    if standby_score > alpha { alpha = standby_score; }

    let mut moves = board.genera_mosse();
    moves.retain(|m| board.get_piece_at(m.a() as usize).is_some() || m.flag() == 5);
    moves.sort_by_cached_key(|m| -punteggio_mossa(board, m));

    for mv in moves {
        if board.esegui_mossa(&mv, None) {
            let score = -quiescence(board, -beta, -alpha);
            board.annulla_mossa(&mv, None, None);
            if score >= beta { return beta; }
            if score > alpha { alpha = score; }
        }
    }
    alpha
}