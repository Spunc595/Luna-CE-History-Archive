use std::time::Instant;
use crate::board::{Scacchiera, Mossa, Colore};

pub const MATE_VALUE: i32 = 49000;

pub struct SearchInfo {
    pub start_time: Instant,
    pub time_limit_ms: u128,
    pub nodes: u64,
    pub stop: bool,
}

pub fn search_root(board: &mut Scacchiera, depth: i32, info: &mut SearchInfo) -> (i32, Mossa) {
    let mut moves = board.genera_mosse();
    // Move ordering: analizza le catture per prime
    moves.sort_by_key(|m| !m.move_flag().is_capture());

    let mut best_move = Mossa { data: 0 };
    let mut alpha = -100000;
    let beta = 100000;
    let mut legal_found = false;

    for m in moves {
        let mut tmp = board.clone();
        if !tmp.esegui_mossa(&m, None) { continue; }
        if !legal_found { best_move = m; legal_found = true; }

        let score = -search(&mut tmp, depth - 1, -beta, -alpha, 1, info);
        if info.stop { break; }

        if score > alpha {
            alpha = score;
            best_move = m;
        }
    }
    (alpha, best_move)
}

fn search(board: &mut Scacchiera, depth: i32, mut alpha: i32, beta: i32, ply: usize, info: &mut SearchInfo) -> i32 {
    info.nodes += 1;
    if info.nodes % 2048 == 0 && info.start_time.elapsed().as_millis() >= info.time_limit_ms {
        info.stop = true;
    }
    if info.stop { return 0; }

    if depth <= 0 { return quiescence(board, alpha, beta, info); }

    let mut moves = board.genera_mosse();
    moves.sort_by_key(|m| !m.move_flag().is_capture());

    let mut legal_count = 0;
    let mut best_score = -100000;

    for m in moves {
        let mut tmp = board.clone();
        if !tmp.esegui_mossa(&m, None) { continue; }
        legal_count += 1;

        let score = -search(&mut tmp, depth - 1, -beta, -alpha, ply + 1, info);
        if score > best_score {
            best_score = score;
            if score > alpha {
                alpha = score;
                if alpha >= beta { return beta; }
            }
        }
    }

    if legal_count == 0 {
        return if board.re_in_scacco(board.turno) { -MATE_VALUE + ply as i32 } else { 0 };
    }
    alpha
}

fn quiescence(board: &mut Scacchiera, mut alpha: i32, beta: i32, info: &mut SearchInfo) -> i32 {
    let stand_pat = crate::evaluation::evaluate(board);
    if stand_pat >= beta { return beta; }
    if stand_pat > alpha { alpha = stand_pat; }

    let mut moves = board.genera_mosse();
    moves.retain(|m| m.move_flag().is_capture());
    
    for m in moves {
        let mut tmp = board.clone();
        if !tmp.esegui_mossa(&m, None) { continue; }
        info.nodes += 1;

        let score = -quiescence(&mut tmp, -beta, -alpha, info);
        if score >= beta { return beta; }
        if score > alpha { alpha = score; }
    }
    alpha
}