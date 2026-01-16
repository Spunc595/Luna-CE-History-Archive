use std::time::Instant;
use crate::board::{Scacchiera, Mossa};
use crate::evaluation::evaluate;
use crate::tt::{TranspositionTable, Bound};
use crate::zobrist::ZobristKeys;
use crate::nnue::LunaNNUE;

pub const MATE_VALUE: i32 = 32000;

pub struct SearchInfo {
    pub start_time: Instant,
    pub time_limit_ms: u128,
    pub nodes: u64,
    pub stop: bool,
    pub history: Vec<u64>,
    pub killer_moves: [[Mossa; 2]; 64],
    pub history_scores: [[i32; 64]; 64],
    pub depth_limit: i32,
}

pub fn iterative_deepening(board: &mut Scacchiera, info: &mut SearchInfo, tt: &mut TranspositionTable, z: &ZobristKeys, nnue: Option<&LunaNNUE>) -> (Mossa, i32) {
    let mut best_m = Mossa { data: 0 };
    let mut last_score = 0;
    for depth in 1..=64 {
        if info.stop || (info.depth_limit > 0 && depth > info.depth_limit) { break; }
        let score = negamax(board, -MATE_VALUE, MATE_VALUE, depth, 0, info, tt, z, nnue);
        if !info.stop {
            if let Some(entry) = tt.probe(board.hash) {
                best_m = Mossa { data: entry.move_data }; last_score = score;
            }
            println!("info depth {} score cp {} nodes {} nps {} pv {}", depth, last_score, info.nodes, (info.nodes as u128 * 1000) / info.start_time.elapsed().as_millis().max(1), best_m.to_uci());
        }
    }
    (best_m, last_score)
}

fn negamax(board: &mut Scacchiera, mut alpha: i32, mut beta: i32, depth: i32, ply: usize, info: &mut SearchInfo, tt: &mut TranspositionTable, z: &ZobristKeys, nnue: Option<&LunaNNUE>) -> i32 {
    if info.nodes & 2047 == 0 && info.start_time.elapsed().as_millis() >= info.time_limit_ms { info.stop = true; }
    if info.stop { return 0; }
    if depth <= 0 { return evaluate(board, nnue); }

    let mut tt_move = 0;
    if let Some(entry) = tt.probe(board.hash) {
        tt_move = entry.move_data;
        if entry.depth as i32 >= depth {
            let s = tt.get_score(&entry, ply);
            match entry.bound { Bound::Exact => return s, Bound::Lower => if s >= beta { return s; }, Bound::Upper => if s <= alpha { return s; }, _ => {} }
        }
    }

    let mut moves = board.genera_mosse();
    moves.sort_by_key(|m| if m.data == tt_move { -1000000 } else { -m.priority(board) });

    let (mut legal_moves, mut best_score, mut best_m) = (0, -MATE_VALUE, Mossa { data: 0 });
    for m in moves {
        let backup = board.clone(); // Temporaneo per stabilità
        if !board.esegui_mossa(&m, z) { *board = backup; continue; }
        legal_moves += 1; info.nodes += 1;
        let score = -negamax(board, -beta, -alpha, depth - 1, ply + 1, info, tt, z, nnue);
        *board = backup;
        if info.stop { return 0; }
        if score > best_score {
            best_score = score; best_m = m;
            if score > alpha { alpha = score; if score >= beta { break; } }
        }
    }
    if legal_moves == 0 { return if board.re_in_scacco(board.turno) { -MATE_VALUE + ply as i32 } else { 0 }; }
    let b = if best_score >= beta { Bound::Lower } else if best_score > alpha { Bound::Exact } else { Bound::Upper };
    tt.store(board.hash, best_score, best_m, depth, b, ply);
    best_score
}