use crate::board::{Colore, Mossa, Scacchiera};
use crate::nnue::LunaNNUE;
use crate::tt::{Bound, TranspositionTable};
use crate::zobrist::ZobristKeys;
use std::time::Instant;
use std::sync::OnceLock;

pub const MAX_DEPTH: usize = 64;
const MATE_SCORE: i32 = 49000;
const INFINITY: i32 = 50000;

static LMR_TABLE: OnceLock<[[i32; 64]; 64]> = OnceLock::new();

#[inline(always)]
fn get_lmr(depth: i32, moves: i32) -> i32 {
    let d = (depth.max(1).min(63)) as usize;
    let m = (moves.max(1).min(63)) as usize;
    
    LMR_TABLE.get_or_init(|| {
        let mut table = [[0; 64]; 64];
        for d in 1..64 {
            for m in 1..64 {
                let ld = (d as f64).ln();
                let lm = (m as f64).ln();
                table[d][m] = (0.75 + 0.3 * ld * lm) as i32;
            }
        }
        table
    })[d][m]
}

pub struct SearchInfo {
    pub start_time: Instant,
    pub hard_limit: u128,
    pub soft_limit: u128,
    pub depth_limit: i32,
    pub nodes: u64,
    pub stopped: bool,
    pub killer: [[Mossa; 2]; MAX_DEPTH],
    pub history: [[[i32; 64]; 64]; 2],
    pub pv_table: [[Mossa; MAX_DEPTH]; MAX_DEPTH],
    pub pv_length: [usize; MAX_DEPTH],
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
            killer: [[Mossa::null(); 2]; MAX_DEPTH],
            history: [[[0; 64]; 64]; 2],
            pv_table: [[Mossa::null(); MAX_DEPTH]; MAX_DEPTH],
            pv_length: [0; MAX_DEPTH],
        }
    }

    #[inline(always)]
    pub fn check_time(&mut self) -> bool {
        if (self.nodes & 2047) == 0 {
            if self.start_time.elapsed().as_millis() >= self.hard_limit {
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
    nnue: Option<&LunaNNUE>,
) -> (Mossa, i32) {
    let mut best = Mossa::null();
    let mut score = 0;
    let mut last_best = Mossa::null();
    let mut stable = 0;

    for depth in 1..=info.depth_limit {
        let mut delta = 20;
        let mut search_alpha = if depth > 1 { score - delta } else { -INFINITY };
        let mut search_beta = if depth > 1 { score + delta } else { INFINITY };
        
        let mut current_score = 0;

        loop {
            let v = negamax(board, depth, search_alpha, search_beta, info, tt, z, nnue, true, 0);
            
            if info.stopped && depth > 1 { break; }
            
            current_score = v;

            if current_score <= search_alpha {
                search_alpha = (current_score - delta).max(-INFINITY);
                delta += delta / 2;
                info.stopped = false;
            } else if current_score >= search_beta {
                search_beta = (current_score + delta).min(INFINITY);
                delta += delta / 2;
                info.stopped = false;
            } else {
                break;
            }
            
            if delta > 1000 {
                search_alpha = -INFINITY;
                search_beta = INFINITY;
            }
        }

        if info.stopped && depth > 1 { break; }

        score = current_score;

        if info.pv_length[0] > 0 {
            best = info.pv_table[0][0];
            let elapsed = info.start_time.elapsed().as_millis();

            if best.data == last_best.data { stable += 1; } 
            else { last_best = best; stable = 0; }

            let scale_factor = if stable >= 3 { 0.5 } else { 1.0 };
            if elapsed > (info.soft_limit as f64 * scale_factor) as u128 && depth >= 8 {
                info.stopped = true;
            }

            let nps = if elapsed > 0 { info.nodes as u128 * 1000 / elapsed } else { 0 };

            print!("info depth {} score cp {} nodes {} nps {} time {} pv", depth, score, info.nodes, nps, elapsed);
            for i in 0..info.pv_length[0] { 
                print!(" {}", info.pv_table[0][i].to_uci()); 
            }
            println!();
        }

        if info.stopped { break; }
    }

    if best.is_null() {
        let mosse = board.genera_mosse_legali(z);
        if !mosse.is_empty() { best = mosse[0]; }
    }

    (best, score)
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
    ply: usize,
) -> i32 {
    info.pv_length[ply] = ply;

    if info.check_time() { return 0; }

    info.nodes += 1;
    let pv_node = beta - alpha > 1;

    if board.ply > 0 {
        if board.mezze_mosse >= 100 || board.is_repetition() { return 0; }
    }

    if ply >= MAX_DEPTH - 1 {
        return quiescence(board, alpha, beta, info, tt, z, nnue, 0);
    }

    if let Some(val) = tt.probe(board.hash, depth, alpha, beta, ply as i32) {
        if !pv_node { return val; }
    }

    let in_check = board.in_scacco();
    
    // FIX 2: Limitiamo le estensioni dello scacco per evitare loop infiniti!
    let new_depth = if in_check && ply < info.depth_limit as usize + 2 { 
        depth + 1 
    } else { 
        depth 
    };

    if new_depth <= 0 {
        return quiescence(board, alpha, beta, info, tt, z, nnue, 0);
    }

    let static_eval = crate::evaluation::evaluate(board, nnue);

    if !pv_node && !in_check && new_depth < 5 {
        let margin = 125 * new_depth;
        if static_eval - margin >= beta { return static_eval; }
    }

    if allow_null && !pv_node && new_depth >= 3 && !in_check && static_eval >= beta {
        let r = if new_depth > 6 { 3 } else { 2 };
        let undo = board.fai_mossa_nulla(z);
        let v = -negamax(board, new_depth - r - 1, -beta, -beta + 1, info, tt, z, nnue, false, ply + 1);
        board.annulla_mossa_nulla(undo, z);
        if v >= beta { return beta; }
    }

    let tt_move = tt.get_move(board.hash);
    let mut best_val = -INFINITY;
    let mut best_move = Mossa::null();
    let mut flag = Bound::Alpha;
    let mut moves_searched = 0;
    
    let mut captures = board.genera_catture_legali(z);
    crate::movegen::ordina_mosse(&mut captures, board, tt_move, depth, info);

    let mut quiets = board.genera_silenziose_legali(z);
    crate::movegen::ordina_mosse(&mut quiets, board, tt_move, depth, info);

    let all_moves = captures.into_iter().chain(quiets.into_iter());

    for m in all_moves {
        let undo = board.esegui_mossa(&m, z, nnue);
        let gives_check = board.in_scacco();
        moves_searched += 1;

        let mut score_val = -INFINITY;

        if moves_searched == 1 {
            score_val = -negamax(board, new_depth - 1, -beta, -alpha, info, tt, z, nnue, true, ply + 1);
        } else {
            let mut do_full = true;
            
            if new_depth >= 3 && moves_searched >= 4 && !in_check && !gives_check && !m.is_cattura() && !m.is_promozione() {
                let mut r = get_lmr(new_depth, moves_searched);
                if pv_node { r -= 1; }
                r = r.clamp(0, new_depth - 2);

                if r > 0 {
                    score_val = -negamax(board, new_depth - 1 - r, -alpha - 1, -alpha, info, tt, z, nnue, true, ply + 1);
                    if score_val <= alpha {
                        do_full = false; 
                    }
                }
            }

            if do_full {
                score_val = -negamax(board, new_depth - 1, -alpha - 1, -alpha, info, tt, z, nnue, true, ply + 1);
                
                if score_val > alpha && score_val < beta {
                    score_val = -negamax(board, new_depth - 1, -beta, -alpha, info, tt, z, nnue, true, ply + 1);
                }
            }
        }

        board.annulla_mossa(&m, undo, z);
        if info.stopped { return 0; }

        if score_val > best_val {
            best_val = score_val;
            if score_val > alpha {
                alpha = score_val;
                flag = Bound::Exact;
                best_move = m;

                info.pv_table[ply][ply] = m;
                for i in (ply + 1)..info.pv_length[ply + 1] {
                    info.pv_table[ply][i] = info.pv_table[ply + 1][i];
                }
                info.pv_length[ply] = info.pv_length[ply + 1];
            }
        }

        if alpha >= beta {
            if !m.is_cattura() && !m.is_promozione() {
                if ply < MAX_DEPTH {
                    if m.data != info.killer[ply][0].data {
                        info.killer[ply][1] = info.killer[ply][0];
                        info.killer[ply][0] = m;
                    }
                    let side = board.turno.opposto().indice();
                    info.history[side][m.da()][m.a()] += depth * depth;
                }
            }
            if !info.stopped {
                tt.store(board.hash, depth, best_val, Bound::Beta, m, ply as i32);
            }
            return best_val;
        }
    }

    if moves_searched == 0 {
        if in_check { return -MATE_SCORE + ply as i32; }
        return 0; 
    }

    if !info.stopped {
        tt.store(board.hash, depth, best_val, flag, best_move, ply as i32);
    }

    best_val
}

fn quiescence(
    board: &mut Scacchiera,
    mut alpha: i32,
    beta: i32,
    info: &mut SearchInfo,
    tt: &mut TranspositionTable,
    z: &ZobristKeys,
    nnue: Option<&LunaNNUE>,
    qply: usize, // FIX 1: Tracciamento profondità in Quiescence
) -> i32 {
    if info.check_time() { return 0; }
    info.nodes += 1;

    let stand_pat = crate::evaluation::evaluate(board, nnue);
    let mut best_val = stand_pat;
    
    if stand_pat >= beta { return stand_pat; }
    if stand_pat > alpha { alpha = stand_pat; }

    // IL FRENO A MANO: Stop forzato dopo 5 catture consecutive!
    if qply >= 5 {
        return best_val;
    }

    let delta_margin = 1050; 
    if stand_pat + delta_margin < alpha {
        return best_val;
    }

    let mut captures = board.genera_catture_legali(z);
    crate::movegen::ordina_mosse(&mut captures, board, Mossa::null(), 0, info);

    for m in captures {
        let undo = board.esegui_mossa(&m, z, nnue);
        let score = -quiescence(board, -beta, -alpha, info, tt, z, nnue, qply + 1);
        board.annulla_mossa(&m, undo, z);

        if score > best_val {
            best_val = score;
            if score >= beta { return score; }
            if score > alpha { alpha = score; }
        }
    }

    best_val
}