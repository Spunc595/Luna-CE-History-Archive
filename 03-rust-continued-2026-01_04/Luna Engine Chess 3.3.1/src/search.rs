use crate::board::{Scacchiera, Mossa, Colore};
use crate::tt::{TranspositionTable, Bound};
use crate::zobrist::ZobristKeys;
use crate::nnue::LunaNNUE;
use std::time::Instant;

const MAX_DEPTH: usize = 64;

pub struct SearchInfo {
    pub start_time: Instant,
    pub hard_limit: u128,
    pub soft_limit: u128,
    pub depth_limit: i32,
    pub nodes: u64,
    pub stopped: bool,
    // --- Move Ordering Tables ---
    pub killer_moves: [[Mossa; 2]; MAX_DEPTH],
    pub history_table: [[[i32; 64]; 64]; 2], 
}

impl SearchInfo {
    pub fn new(time_limit: u128, depth_limit: i32) -> Self {
        let soft = if time_limit > 500 { time_limit * 60 / 100 } else { time_limit };
        SearchInfo {
            start_time: Instant::now(),
            hard_limit: time_limit,
            soft_limit: soft,
            depth_limit,
            nodes: 0,
            stopped: false,
            killer_moves: [[Mossa::null(); 2]; MAX_DEPTH],
            history_table: [[[0; 64]; 64]; 2],
        }
    }

    #[inline(always)]
    pub fn check_time(&mut self) -> bool {
        if (self.nodes & 2047) == 0 {
            let elapsed = self.start_time.elapsed().as_millis();
            if elapsed >= self.hard_limit {
                self.stopped = true;
            }
        }
        self.stopped
    }
}

pub fn iterative_deepening(
    board: &mut Scacchiera, 
    info: &mut SearchInfo, 
    tt: &mut TranspositionTable,
    z: &ZobristKeys,
    nnue: Option<&LunaNNUE>
) -> (Mossa, i32) {
    let mut best_move = Mossa::null();
    let mut score = 0;
    let mut last_best_move = Mossa::null();
    let mut stability_counter = 0;

    let mut alpha = -50000;
    let mut beta = 50000;

    for depth in 1..=info.depth_limit {
        let (val, pv) = negamax(board, depth, alpha, beta, info, tt, z, nnue, true);
        
        if info.stopped && depth > 1 { break; }

        score = val;

        // Aspiration Windows
        if score <= alpha || score >= beta {
            alpha = -50000;
            beta = 50000;
        } else {
            alpha = score - 50;
            beta = score + 50;
        }

        if !pv.is_empty() {
            best_move = pv[0];
            let elapsed = info.start_time.elapsed().as_millis();
            
            if best_move.data == last_best_move.data {
                stability_counter += 1;
            } else {
                last_best_move = best_move;
                stability_counter = 0;
            }

            if elapsed > info.soft_limit && (stability_counter >= 3 || depth > 8) {
                info.stopped = true;
            }

            let nps = if elapsed > 0 { info.nodes as u128 * 1000 / elapsed } else { 0 };
            
            print!("info depth {} score cp {} nodes {} nps {} time {} pv", 
                depth, score, info.nodes, nps, elapsed);
            for m in &pv { print!(" {}", m.to_uci()); }
            println!();
        }
        
        if info.stopped { break; }
    }

    if best_move.is_null() {
        let legali = board.genera_mosse_legali(z);
        if !legali.is_empty() { best_move = legali[0]; }
    }

    (best_move, score)
}

fn negamax(
    board: &mut Scacchiera, 
    depth: i32, 
    mut alpha: i32, 
    beta: i32, 
    info: &mut SearchInfo,
    tt: &mut TranspositionTable,
    z: &ZobristKeys,
    nnue: Option<&LunaNNUE>,
    allow_null: bool,
) -> (i32, Vec<Mossa>) {
    
    if info.check_time() { return (0, Vec::new()); }

    info.nodes += 1;
    let pv_node = beta - alpha > 1; 

    // Repetizioni e Regola 50 mosse
    if board.ply > 0 && (board.is_repetition() || board.rule_50 >= 100) {
        return (0, Vec::new());
    }

    // Transposition Table Probe
    if let Some(entry) = tt.probe(board.hash, depth, alpha, beta) {
        if !pv_node { return (entry, Vec::new()); }
    }
    
    let tt_move = tt.get_move(board.hash);
    let in_check = board.in_scacco();
    let new_depth = if in_check { depth + 1 } else { depth };

    if new_depth <= 0 {
        return (quiescence(board, alpha, beta, info, z, nnue), Vec::new());
    }

    // --- NULL MOVE PRUNING ---
    if allow_null && !pv_node && new_depth >= 3 && !in_check {
        let static_eval = crate::evaluation::evaluate(board);
        if static_eval >= beta {
            let r = if new_depth > 6 { 4 } else { 3 };
            let undo = board.fai_mossa_nulla(z);
            let (null_val, _) = negamax(board, new_depth - r - 1, -beta, -beta + 1, info, tt, z, nnue, false);
            board.annulla_mossa_nulla(undo, z);
            
            if null_val >= beta { return (beta, Vec::new()); }
        }
    }

    let mut legal_moves = board.genera_mosse_legali(z);
    if legal_moves.is_empty() {
        return (if in_check { -49000 + (board.ply as i32) } else { 0 }, Vec::new());
    }

    // L'ordinamento ora riceve anche 'info' per Killer e History
    crate::movegen::ordina_mosse(&mut legal_moves, board, tt_move, depth, info);

    let mut best_val = -50000;
    let mut best_pv = Vec::new();
    let mut flag = Bound::Alpha;
    let mut moves_searched = 0;

    for m in legal_moves {
        let mut new_board = board.clone();
        if new_board.esegui_mossa(&m, z) {
            moves_searched += 1;
            let mut val = 0;
            let mut child_pv = Vec::new();

            if moves_searched == 1 {
                let res = negamax(&mut new_board, new_depth - 1, -beta, -alpha, info, tt, z, nnue, true);
                val = -res.0; child_pv = res.1;
            } else {
                let mut needs_full_search = true;
                
                // --- LMR (Late Move Reductions) ---
                if new_depth >= 3 && moves_searched >= 4 && !in_check && !m.is_cattura() && !m.is_promozione() {
                    let r = if moves_searched > 6 { 2 } else { 1 };
                    let res_lmr = negamax(&mut new_board, new_depth - 1 - r, -alpha - 1, -alpha, info, tt, z, nnue, true);
                    let val_lmr = -res_lmr.0;
                    
                    if val_lmr <= alpha {
                        val = val_lmr;
                        child_pv = res_lmr.1;
                        needs_full_search = false;
                    }
                }
                
                if needs_full_search {
                    let res = negamax(&mut new_board, new_depth - 1, -alpha - 1, -alpha, info, tt, z, nnue, true);
                    val = -res.0;
                    
                    if val > alpha && val < beta {
                        let res = negamax(&mut new_board, new_depth - 1, -beta, -alpha, info, tt, z, nnue, true);
                        val = -res.0; child_pv = res.1;
                    } else { 
                        child_pv = res.1; 
                    }
                }
            }

            if info.stopped { return (0, Vec::new()); }

            if val > best_val {
                best_val = val;
                let mut pv = vec![m];
                pv.extend(child_pv);
                best_pv = pv;
            }

            if val > alpha {
                alpha = val;
                flag = Bound::Exact;
            }

            // --- BETA CUTOFF (Mossa troppo forte) ---
            if alpha >= beta {
                // Se la mossa è "tranquilla" (non cattura), aggiorniamo Killer e History
                if !m.is_cattura() && !m.is_promozione() {
                    let d = depth as usize;
                    if d < MAX_DEPTH {
                        // Killer Moves
                        if m.data != info.killer_moves[d][0].data {
                            info.killer_moves[d][1] = info.killer_moves[d][0];
                            info.killer_moves[d][0] = m;
                        }
                        // History Heuristic
                        let side = board.turno.indice();
                        info.history_table[side][m.da()][m.a()] += (depth * depth) as i32;
                    }
                }

                tt.store(board.hash, depth, beta, Bound::Beta, m);
                return (beta, Vec::new());
            }
        }
    }

    tt.store(board.hash, depth, best_val, flag, if !best_pv.is_empty() { best_pv[0] } else { Mossa::null() });
    (best_val, best_pv)
}

fn quiescence(board: &mut Scacchiera, mut alpha: i32, beta: i32, info: &mut SearchInfo, z: &ZobristKeys, nnue: Option<&LunaNNUE>) -> i32 {
    info.nodes += 1;
    let stand_pat = crate::evaluation::evaluate(board);
    if stand_pat >= beta { return beta; }
    if stand_pat > alpha { alpha = stand_pat; }

    let mut moves = board.genera_mosse_legali(z);
    moves.retain(|m| m.is_cattura() || m.is_promozione()); 

    // Ordinamento semplificato per la quiescence
    crate::movegen::ordina_mosse(&mut moves, board, Mossa::null(), 0, info);

    for m in moves {
        let mut new_board = board.clone();
        if new_board.esegui_mossa(&m, z) {
            let score = -quiescence(&mut new_board, -beta, -alpha, info, z, nnue);
            if score >= beta { return beta; }
            if score > alpha { alpha = score; }
        }
    }
    alpha
}