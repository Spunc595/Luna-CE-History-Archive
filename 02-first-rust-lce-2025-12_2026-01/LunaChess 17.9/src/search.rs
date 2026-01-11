use std::time::Instant;
use crate::board::{Scacchiera, Mossa};
use crate::evaluation::evaluate;
use crate::tt::{TranspositionTable, Bound};
use crate::zobrist::ZobristKeys;

pub const MATE_VALUE: i32 = 49000;

pub struct SearchInfo {
    pub start_time: Instant,
    pub time_limit_ms: u128,
    pub nodes: u64,
    pub stop: bool,
    pub history: Vec<u64>,
}

pub fn iterative_deepening(board: &mut Scacchiera, info: &mut SearchInfo, tt: &mut TranspositionTable, z: &ZobristKeys) -> Mossa {
    let mut best_m = board.genera_mosse()[0];
    for depth in 1..64 {
        if info.stop { break; }
        let score = negamax(board, -MATE_VALUE, MATE_VALUE, depth, 0, true, info, tt, z);
        if info.stop { break; }
        if let Some(entry) = tt.probe(board.get_hash(z)) {
            best_m = Mossa { data: entry.move_data };
            let elapsed = info.start_time.elapsed().as_millis().max(1);
            println!("info depth {} score cp {} nodes {} nps {} pv {}", 
                depth, score, info.nodes, (info.nodes as u128 * 1000) / elapsed, best_m);
        }
        if score.abs() > MATE_VALUE - 100 { break; }
    }
    best_m
}

fn negamax(board: &mut Scacchiera, mut alpha: i32, mut beta: i32, depth: i32, ply: usize, allow_null: bool, info: &mut SearchInfo, tt: &mut TranspositionTable, z: &ZobristKeys) -> i32 {
    info.nodes += 1;
    if info.nodes % 2048 == 0 && info.start_time.elapsed().as_millis() >= info.time_limit_ms { info.stop = true; }
    if info.stop { return 0; }

    let hash = board.get_hash(z);
    if ply > 0 && info.history.contains(&hash) { return 0; }
    if depth <= 0 { return quiescence(board, alpha, beta, info, z); }

    let mut tt_m = Mossa { data: 0 };
    if let Some(entry) = tt.probe(hash) {
        tt_m = Mossa { data: entry.move_data };
        if entry.depth as i32 >= depth {
            let s = tt.get_score(&entry, ply);
            match entry.bound {
                1 => return s,
                2 => alpha = alpha.max(s),
                3 => beta = beta.min(s),
                _ => {}
            }
            if alpha >= beta { return s; }
        }
    }

    if allow_null && depth >= 3 && !board.re_in_scacco(board.turno) && evaluate(board) >= beta {
        board.make_null_move();
        let score = -negamax(board, -beta, -beta + 1, depth - 3, ply + 1, false, info, tt, z);
        board.make_null_move(); 
        if score >= beta { return beta; }
    }

    let mut moves = board.genera_mosse();
    moves.sort_by_key(|m| {
        if m.data == tt_m.data { -100000 }
        else if m.move_flag().is_capture() { -10000 }
        else { 0 }
    });

    let mut legal_moves = 0;
    let mut best_score = -MATE_VALUE;
    let mut best_move = Mossa { data: 0 };
    info.history.push(hash);

    for m in moves {
        let mut temp_board = board.clone();
        if !temp_board.esegui_mossa(&m, z) { continue; } // CORRETTO: rimosso Some()
        legal_moves += 1;
        let score = -negamax(&mut temp_board, -beta, -alpha, depth - 1, ply + 1, true, info, tt, z);
        if score > best_score { best_score = score; best_move = m; }
        alpha = alpha.max(score);
        if alpha >= beta {
            tt.store(hash, best_score, m, depth, Bound::Lower, ply);
            info.history.pop();
            return beta;
        }
    }
    info.history.pop();

    if legal_moves == 0 {
        return if board.re_in_scacco(board.turno) { -MATE_VALUE + ply as i32 } else { 0 };
    }
    let bound = if best_score >= beta { Bound::Lower } else if best_score > alpha { Bound::Exact } else { Bound::Upper };
    tt.store(hash, best_score, best_move, depth, bound, ply);
    best_score
}

fn quiescence(board: &mut Scacchiera, mut alpha: i32, beta: i32, info: &mut SearchInfo, z: &ZobristKeys) -> i32 {
    let stand_pat = evaluate(board);
    if stand_pat >= beta { return beta; }
    alpha = alpha.max(stand_pat);

    let mut moves = board.genera_mosse();
    moves.retain(|m| m.move_flag().is_capture());
    for m in moves {
        let mut temp_board = board.clone();
        if !temp_board.esegui_mossa(&m, z) { continue; } // CORRETTO: passato z reale
        info.nodes += 1;
        let score = -quiescence(&mut temp_board, -beta, -alpha, info, z);
        if score >= beta { return beta; }
        alpha = alpha.max(score);
    }
    alpha
}