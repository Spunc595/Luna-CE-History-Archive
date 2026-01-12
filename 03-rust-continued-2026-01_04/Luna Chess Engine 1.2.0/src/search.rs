use std::time::Instant;
use crate::board::{Scacchiera, Mossa};
use crate::evaluation::evaluate;
use crate::tt::{TranspositionTable, Bound};
use crate::zobrist::ZobristKeys;

pub const MATE_VALUE: i32 = 49000;
pub const TB_WIN_VALUE: i32 = 30000; // Valore per posizioni vinte da Tablebase
const ASPIRATION_WINDOW: i32 = 50;
const MAX_PLY: usize = 64;

pub struct SearchInfo {
    pub start_time: Instant,
    pub time_limit_ms: u128,
    pub nodes: u64,
    pub stop: bool,
    pub history: Vec<u64>,
    pub killer_moves: [[Mossa; 2]; MAX_PLY],
    pub history_scores: [[i32; 64]; 64],
    pub use_syzygy: bool, // Flag attivato via UCI
}

pub fn iterative_deepening(board: &mut Scacchiera, info: &mut SearchInfo, tt: &mut TranspositionTable, z: &ZobristKeys) -> Mossa {
    let all_legal_moves = board.genera_mosse();
    let mut best_m = all_legal_moves[0];
    let mut alpha = -MATE_VALUE;
    let mut beta = MATE_VALUE;
    let mut last_score = 0;

    // --- SYZYGY ROOT PROBE ---
    // Se la posizione ha pochi pezzi, controlliamo se è già risolta
    if info.use_syzygy && board.occupazione().count_ones() <= 5 {
        // Qui andrebbe la chiamata alla libreria Syzygy (es: probe_root)
        // Se restituisce un risultato certo, potremmo saltare la ricerca.
    }

    for depth in 1..MAX_PLY as i32 {
        if info.stop { break; }

        let mut score;
        if depth > 3 {
            alpha = last_score - ASPIRATION_WINDOW;
            beta = last_score + ASPIRATION_WINDOW;
        }

        loop {
            score = negamax(board, alpha, beta, depth, 0, true, info, tt, z);
            if info.stop { break; }

            if score <= alpha {
                alpha -= ASPIRATION_WINDOW * 4;
            } else if score >= beta {
                beta += ASPIRATION_WINDOW * 4;
            } else {
                break;
            }
        }
        
        if info.stop { break; }
        last_score = score;

        if let Some(entry) = tt.probe(board.hash) {
            let tt_move = Mossa { data: entry.move_data };
            if all_legal_moves.iter().any(|m| m.data == tt_move.data) {
                best_m = tt_move;
            }
            let elapsed = info.start_time.elapsed().as_millis().max(1);
            let nps = (info.nodes as u128 * 1000) / elapsed;
            println!("info depth {} score cp {} nodes {} nps {} pv {}", depth, score, info.nodes, nps, best_m);
        }
    }
    best_m
}

fn negamax(board: &mut Scacchiera, mut alpha: i32, mut beta: i32, depth: i32, ply: usize, allow_null: bool, info: &mut SearchInfo, tt: &mut TranspositionTable, z: &ZobristKeys) -> i32 {
    info.nodes += 1;
    
    if info.nodes & 2047 == 0 && info.start_time.elapsed().as_millis() >= info.time_limit_ms {
        info.stop = true;
    }
    if info.stop { return 0; }
    if ply >= MAX_PLY { return evaluate(board); }
    if ply > 0 && info.history.contains(&board.hash) { return 0; }

    // --- SYZYGY DEPTH PROBE ---
    if info.use_syzygy && depth > 0 && board.occupazione().count_ones() <= 5 {
        // Esempio logico: probe_wdl(board)
        // let tb_score = probe_syzygy_wdl(board); 
        // Se WIN: return TB_WIN_VALUE - ply as i32;
        // Se LOSS: return -TB_WIN_VALUE + ply as i32;
        // Se DRAW: return 0;
    }

    let mut tt_m = Mossa { data: 0 };
    if let Some(entry) = tt.probe(board.hash) {
        tt_m = Mossa { data: entry.move_data };
        if entry.depth as i32 >= depth {
            let s = tt.get_score(&entry, ply);
            match entry.bound {
                1 => return s,
                2 => if s >= beta { return s; },
                3 => if s <= alpha { return s; },
                _ => {}
            }
        }
    }

    if depth <= 0 { return quiescence(board, alpha, beta, info, z); }

    // Null Move Pruning
    if allow_null && depth >= 3 && !board.re_in_scacco(board.turno) && evaluate(board) >= beta {
        let mut temp_board = board.clone();
        temp_board.make_null_move_hash(z);
        let score = -negamax(&mut temp_board, -beta, -beta + 1, depth - 3, ply + 1, false, info, tt, z);
        if score >= beta { return beta; }
    }

    let mut moves = board.genera_mosse();
    
    // MOVE ORDERING: TT > Captures > Killer > History
    moves.sort_by_key(|m: &Mossa| {
        if m.data == tt_m.data { -3000000 }
        else if m.move_flag().is_capture() { -2000000 }
        else if m.data == info.killer_moves[ply][0].data { -1000000 }
        else if m.data == info.killer_moves[ply][1].data { -500000 }
        else { -info.history_scores[m.da()][m.a()] }
    });

    let mut legal_moves = 0;
    let mut best_score = -MATE_VALUE;
    let mut best_move = Mossa { data: 0 };
    let old_alpha = alpha;

    info.history.push(board.hash);

    for (i, m) in moves.iter().enumerate() {
        let mut temp_board = board.clone();
        if !temp_board.esegui_mossa(m, z) { continue; }
        legal_moves += 1;

        let score = if i == 0 {
            -negamax(&mut temp_board, -beta, -alpha, depth - 1, ply + 1, true, info, tt, z)
        } else {
            let mut s = -negamax(&mut temp_board, -alpha - 1, -alpha, depth - 1, ply + 1, true, info, tt, z);
            if s > alpha && s < beta {
                s = -negamax(&mut temp_board, -beta, -alpha, depth - 1, ply + 1, true, info, tt, z);
            }
            s
        };

        if info.stop { break; }

        if score > best_score {
            best_score = score;
            best_move = *m;
            if score > alpha { alpha = score; }
        }

        if alpha >= beta {
            if !m.move_flag().is_capture() {
                // Killer Moves
                if m.data != info.killer_moves[ply][0].data {
                    info.killer_moves[ply][1] = info.killer_moves[ply][0];
                    info.killer_moves[ply][0] = *m;
                }
                // NUOVA FORMULA HISTORY: depth * depth
                info.history_scores[m.da()][m.a()] += (depth * depth) as i32;
            }
            tt.store(board.hash, beta, *m, depth, Bound::Lower, ply);
            info.history.pop();
            return beta;
        }
    }

    info.history.pop();
    if legal_moves == 0 {
        return if board.re_in_scacco(board.turno) { -MATE_VALUE + ply as i32 } else { 0 };
    }

    let bound = if best_score >= beta { Bound::Lower } else if best_score > old_alpha { Bound::Exact } else { Bound::Upper };
    tt.store(board.hash, best_score, best_move, depth, bound, ply);
    best_score
}

fn quiescence(board: &mut Scacchiera, mut alpha: i32, beta: i32, info: &mut SearchInfo, z: &ZobristKeys) -> i32 {
    let stand_pat = evaluate(board);
    if stand_pat >= beta { return beta; }
    if stand_pat > alpha { alpha = stand_pat; }

    let mut moves = board.genera_mosse();
    moves.retain(|m: &Mossa| m.move_flag().is_capture());
    
    for m in moves {
        let mut temp_board = board.clone();
        if !temp_board.esegui_mossa(&m, z) { continue; }
        info.nodes += 1;
        let score = -quiescence(&mut temp_board, -beta, -alpha, info, z);
        if score >= beta { return beta; }
        if score > alpha { alpha = score; }
    }
    alpha
}