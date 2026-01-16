use std::time::Instant;
use crate::board::{Scacchiera, Mossa};
use crate::tt::{TranspositionTable, Bound};
use crate::zobrist::ZobristKeys;
use crate::nnue::LunaNNUE;
use crate::evaluation::evaluate as eval_classic; 

pub const MATE_VALUE: i32 = 32000;
pub const INFINITY: i32 = 32001;

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

impl SearchInfo {
    pub fn clear_heuristics(&mut self) {
        self.killer_moves = [[Mossa { data: 0 }; 2]; 64];
        self.history_scores = [[0; 64]; 64];
    }
}

pub fn iterative_deepening(board: &mut Scacchiera, info: &mut SearchInfo, tt: &mut TranspositionTable, z: &ZobristKeys, nnue: Option<&LunaNNUE>) -> (Mossa, i32) {
    let mut best_m = Mossa { data: 0 };
    let mut last_score = 0;
    
    info.history.clear();
    info.history.push(board.hash);
    info.nodes = 0;
    info.stop = false;

    let mut alpha = -INFINITY;
    let mut beta = INFINITY;

    for depth in 1..=64 {
        if info.stop { break; }
        if info.depth_limit > 0 && depth > info.depth_limit { break; }
        
        let score = negamax(board, alpha, beta, depth, 0, info, tt, z, nnue);
        
        if info.stop { break; }

        // Aspiration Window
        if score <= alpha || score >= beta {
            alpha = -INFINITY;
            beta = INFINITY;
            let full_score = negamax(board, -INFINITY, INFINITY, depth, 0, info, tt, z, nnue);
            if info.stop { break; }
            last_score = full_score;
        } else {
            last_score = score;
            alpha = score - 50;
            beta = score + 50;
        }

        if let Some(entry) = tt.probe(board.hash) {
            best_m = Mossa { data: entry.move_data };
        }

        let elapsed = info.start_time.elapsed().as_millis().max(1);
        let nps = (info.nodes as u128 * 1000) / elapsed;
        
        println!("info depth {} score cp {} nodes {} nps {} pv {}", 
                 depth, last_score, info.nodes, nps, best_m.to_uci());
    }

    (best_m, last_score)
}

fn negamax(board: &mut Scacchiera, mut alpha: i32, mut beta: i32, depth: i32, ply: usize, info: &mut SearchInfo, tt: &mut TranspositionTable, z: &ZobristKeys, nnue: Option<&LunaNNUE>) -> i32 {
    
    if info.nodes & 2047 == 0 {
        if info.start_time.elapsed().as_millis() >= info.time_limit_ms { 
            info.stop = true; 
        }
    }
    if info.stop { return 0; }

    let is_root = ply == 0;
    let in_check = board.re_in_scacco(board.turno);

    let extension = if in_check { 1 } else { 0 };
    let new_depth = depth + extension;

    if new_depth <= 0 { 
        return quiescence(board, alpha, beta, info, nnue, z); 
    }

    let mut tt_move = 0;
    if let Some(entry) = tt.probe(board.hash) {
        tt_move = entry.move_data;
        if !is_root && entry.depth as i32 >= new_depth {
            let s = tt.get_score(&entry, ply);
            match entry.bound {
                Bound::Exact => return s,
                Bound::Lower => if s >= beta { return s; },
                Bound::Upper => if s <= alpha { return s; },
                _ => {}
            }
        }
    }

    let mut moves = board.genera_mosse();
    
    // Ordering
    moves.sort_by_key(|m| {
        if m.data == tt_move { return -2_000_000; }
        if m.is_cattura() { return -1_000_000 - m.priority(board); }
        
        let ply_idx = ply.min(63);
        if info.killer_moves[ply_idx][0].data == m.data { return -900_000; }
        if info.killer_moves[ply_idx][1].data == m.data { return -800_000; }
        
        let from = m.da();
        let to = m.a();
        -(info.history_scores[from][to] as i32)
    });

    let mut legal_moves = 0;
    let mut best_score = -MATE_VALUE;
    let mut best_m = Mossa { data: 0 };

    for (i, m) in moves.iter().enumerate() {
        let backup = board.clone(); 
        if !board.esegui_mossa(m, z) { 
            *board = backup; 
            continue; 
        }
        
        legal_moves += 1; 
        info.nodes += 1;
        
        let mut score;
        let gives_check = board.re_in_scacco(board.turno); // Controlla se abbiamo dato scacco
        
        // PVS & LMR (Late Move Reduction) - CALMIERATO
        if i == 0 {
            score = -negamax(board, -beta, -alpha, new_depth - 1, ply + 1, info, tt, z, nnue);
        } else {
            let mut reduction = 0;
            // LMR FIX:
            // 1. Solo se depth >= 3 (prima era 3, ok)
            // 2. NO catture, NO promozioni
            // 3. NO se siamo sotto scacco (in_check)
            // 4. NUOVO: NO se la mossa DA scacco (gives_check) -> Importante per vedere le forchette!
            if new_depth >= 3 && !m.is_cattura() && !m.is_promozione() && !in_check && !gives_check {
                // Formula più gentile: depth / 4 invece di depth / 3
                reduction = 1 + new_depth / 4;
                if i > 8 { reduction += 1; } // Riduciamo extra solo molto dopo
            }

            score = -negamax(board, -alpha - 1, -alpha, new_depth - 1 - reduction, ply + 1, info, tt, z, nnue);

            if score > alpha && reduction > 0 {
                score = -negamax(board, -alpha - 1, -alpha, new_depth - 1, ply + 1, info, tt, z, nnue);
            }

            if score > alpha && score < beta {
                score = -negamax(board, -beta, -alpha, new_depth - 1, ply + 1, info, tt, z, nnue);
            }
        }
        
        *board = backup; 
        
        if info.stop { return 0; }

        if score > best_score {
            best_score = score;
            best_m = *m;
            if score > alpha {
                alpha = score;
                if score >= beta {
                    if !m.is_cattura() {
                        let ply_idx = ply.min(63);
                        info.killer_moves[ply_idx][1] = info.killer_moves[ply_idx][0];
                        info.killer_moves[ply_idx][0] = *m;
                        let bonus = (new_depth * new_depth) as i32;
                        info.history_scores[m.da()][m.a()] += bonus;
                    }
                    break;
                } 
            }
        }
    }

    if legal_moves == 0 { 
        return if in_check { -MATE_VALUE + ply as i32 } else { 0 }; 
    }
    
    let b = if best_score >= beta { Bound::Lower } else if best_score > alpha { Bound::Exact } else { Bound::Upper };
    tt.store(board.hash, best_score, best_m, new_depth, b, ply);
    
    best_score
}

fn quiescence(board: &mut Scacchiera, mut alpha: i32, beta: i32, info: &mut SearchInfo, nnue: Option<&LunaNNUE>, z: &ZobristKeys) -> i32 {
    
    if info.nodes & 2047 == 0 {
        if info.start_time.elapsed().as_millis() >= info.time_limit_ms { 
            info.stop = true; 
            return 0;
        }
    }

    let stand_pat = if let Some(net) = nnue {
        net.evaluate(board)
    } else {
        eval_classic(board, None) 
    };

    if stand_pat >= beta { return beta; }
    if alpha < stand_pat { alpha = stand_pat; }

    let mut moves = board.genera_mosse();
    
    let mut i = 0;
    while i < moves.len() {
        let m = &moves[i];
        let is_capture = board.pezzo_in(m.a()).is_some();
        if is_capture || m.is_promozione() {
            i += 1;
        } else {
            moves.swap_remove(i);
        }
    }

    moves.sort_by_key(|m| -m.priority(board));

    for m in moves {
        let backup = board.clone();
        if !board.esegui_mossa(&m, z) { *board = backup; continue; }
        
        info.nodes += 1;
        
        let score = -quiescence(board, -beta, -alpha, info, nnue, z);
        
        *board = backup;

        if score >= beta { return beta; }
        if score > alpha { alpha = score; }
    }
    alpha
}