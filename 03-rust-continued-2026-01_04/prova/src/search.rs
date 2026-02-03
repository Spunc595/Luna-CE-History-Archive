use crate::board::{Scacchiera, Mossa, Colore};
use crate::tt::{TranspositionTable, Bound};
use crate::zobrist::ZobristKeys;
use crate::nnue::LunaNNUE;
use std::time::Instant;

const MAX_DEPTH: usize = 64;
const MATE_SCORE: i32 = 49000;
const INFINITY: i32 = 50000;

pub struct SearchInfo {
    pub start_time: Instant,
    pub hard_limit: u128,
    pub soft_limit: u128,
    pub depth_limit: i32,
    pub nodes: u64,
    pub stopped: bool,
    pub killer_moves: [[Mossa; 2]; MAX_DEPTH],
    pub history_table: [[[i32; 64]; 64]; 2], 
}

impl SearchInfo {
    pub fn new(time_limit: u128, depth_limit: i32) -> Self {
        SearchInfo {
            start_time: Instant::now(),
            hard_limit: time_limit,
            soft_limit: time_limit * 60 / 100,
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
            if self.start_time.elapsed().as_millis() >= self.hard_limit { 
                self.stopped = true; 
            }
        }
        self.stopped
    }
}

pub fn iterative_deepening(board: &mut Scacchiera, info: &mut SearchInfo, tt: &mut TranspositionTable, z: &ZobristKeys, nnue: Option<&LunaNNUE>) -> (Mossa, i32) {
    let mut best_move = Mossa::null();
    let mut score = 0;
    let mut alpha = -INFINITY;
    let mut beta = INFINITY;
    
    if let Some(net) = nnue {
        board.accumulator = net.refresh_accumulator(board);
    }

    let mut depth = 1;
    while depth <= info.depth_limit {
        let (val, pv) = negamax(board, depth, alpha, beta, info, tt, z, nnue, true);
        
        if info.stopped && depth > 1 { break; }

        score = val;
        
        if score <= alpha || score >= beta { 
            alpha = -INFINITY;
            beta = INFINITY;
            continue; 
        }

        alpha = score - 30; 
        beta = score + 30; 

        if !pv.is_empty() {
            best_move = pv[0];
            let elapsed = info.start_time.elapsed().as_millis();
            let nps = if elapsed > 0 { info.nodes as u128 * 1000 / elapsed } else { 0 };
            
            let pv_string = pv.iter().map(|m| m.to_uci()).collect::<Vec<String>>().join(" ");
            let score_str = if score.abs() > MATE_SCORE - 1000 {
                let plies_to_mate = MATE_SCORE - score.abs();
                let moves_to_mate = (plies_to_mate + 1) / 2;
                format!("mate {}", moves_to_mate * score.signum())
            } else { 
                format!("cp {}", score) 
            };

            println!("info depth {} score {} nodes {} nps {} time {} pv {}", 
                depth, score_str, info.nodes, nps, elapsed, pv_string);
            
            if elapsed > info.soft_limit && depth > 8 { info.stopped = true; }
        }
        
        if info.stopped { break; }
        depth += 1;
    }
    
    if best_move.is_null() {
        let mut moves = board.genera_mosse();
        crate::movegen::ordina_mosse(&mut moves, board, Mossa::null(), 1, info);
        for m in moves {
            if board.esegui_mossa(&m, z, nnue) {
                board.annulla_mossa(&m, z);
                best_move = m;
                break;
            }
        }
    }

    (best_move, score)
}

fn negamax(board: &mut Scacchiera, depth: i32, mut alpha: i32, beta: i32, info: &mut SearchInfo, tt: &mut TranspositionTable, z: &ZobristKeys, nnue: Option<&LunaNNUE>, allow_null: bool) -> (i32, Vec<Mossa>) {
    if info.check_time() { return (0, Vec::new()); }
    
    if board.ply > 0 && (board.is_repetition() || board.rule_50 >= 100) { 
        return (0, Vec::new()); 
    }

    let original_alpha = alpha;

    let tt_move = if let Some(tt_val) = tt.probe(board.hash, depth, alpha, beta, board.ply as i32) {
        return (tt_val, Vec::new()); 
    } else {
        tt.get_move(board.hash)
    };
    
    let in_check = board.in_scacco();

    if depth <= 0 { 
        return (quiescence(board, alpha, beta, info, z, nnue), Vec::new()); 
    }

    info.nodes += 1;

    if allow_null && depth >= 3 && !in_check && board.fase_gioco() > 2 {
        let _undo = board.fai_mossa_nulla(z);
        let (null_val, _) = negamax(board, depth - 4, -beta, -beta + 1, info, tt, z, nnue, false);
        board.annulla_mossa_nulla(_undo, z);
        if -null_val >= beta { return (beta, Vec::new()); }
    }

    let mut moves = board.genera_mosse();
    crate::movegen::ordina_mosse(&mut moves, board, tt_move, depth, info);

    let mut best_val = -INFINITY;
    let mut best_pv = Vec::new();
    let mut moves_searched = 0;
    let mut has_legal_move = false;

    for m in moves {
        if board.esegui_mossa(&m, z, nnue) {
            has_legal_move = true;
            moves_searched += 1;
            
            // RENDIAMO MUTABILE val PER RISOLVERE L'ERRORE E0384
            let mut val: i32;
            let mut child_pv: Vec<Mossa> = Vec::new();

            if moves_searched == 1 {
                let res = negamax(board, depth - 1, -beta, -alpha, info, tt, z, nnue, true);
                val = -res.0;
                child_pv = res.1;
            } else {
                let r = if depth >= 3 && moves_searched > 4 && !m.is_cattura() && !m.is_promozione() && !in_check { 1 } else { 0 };
                let res = negamax(board, depth - 1 - r, -alpha - 1, -alpha, info, tt, z, nnue, true);
                val = -res.0;
                
                if val > alpha && val < beta {
                    let res = negamax(board, depth - 1, -beta, -alpha, info, tt, z, nnue, true);
                    val = -res.0;
                    child_pv = res.1;
                }
            }

            board.annulla_mossa(&m, z);

            if info.stopped { return (0, Vec::new()); }

            if val > best_val {
                best_val = val;
                if val > alpha {
                    alpha = val; 
                    let mut pv = vec![m];
                    pv.extend(child_pv);
                    best_pv = pv;
                }
            }

            if alpha >= beta {
                if !m.is_cattura() {
                    let ply = board.ply as usize;
                    if ply < MAX_DEPTH { 
                        info.killer_moves[ply][1] = info.killer_moves[ply][0];
                        info.killer_moves[ply][0] = m;
                        info.history_table[board.turno.indice()][m.da()][m.a()] += depth * depth; 
                    }
                }
                tt.store(board.hash, depth, beta, Bound::Beta, m, board.ply as i32);
                return (beta, Vec::new());
            }
        }
    }

    if !has_legal_move {
        return (if in_check { -MATE_SCORE + (board.ply as i32) } else { 0 }, Vec::new());
    }

    let bound = if best_val <= original_alpha { Bound::Alpha } else { Bound::Exact };
    tt.store(board.hash, depth, best_val, bound, if !best_pv.is_empty() { best_pv[0] } else { Mossa::null() }, board.ply as i32);
    
    (best_val, best_pv)
}

fn quiescence(board: &mut Scacchiera, mut alpha: i32, beta: i32, info: &mut SearchInfo, z: &ZobristKeys, nnue: Option<&LunaNNUE>) -> i32 {
    if info.check_time() { return 0; }
    
    let stand_pat = if let Some(net) = nnue {
        net.evaluate_from_acc(&board.accumulator, board.turno)
    } else {
        crate::evaluation::evaluate(board)
    };

    if stand_pat >= beta { return beta; }
    if stand_pat > alpha { alpha = stand_pat; }

    let mut moves = board.genera_mosse();
    moves.retain(|m: &Mossa| m.is_cattura() || m.is_promozione());
    
    if moves.is_empty() { return stand_pat; }

    moves.sort_unstable_by_key(|m: &Mossa| {
        let victim = board.pezzo_in(m.a()).unwrap_or(0);
        -(victim as i32)
    });

    for m in moves {
        if board.esegui_mossa(&m, z, nnue) {
            info.nodes += 1;
            let score = -quiescence(board, -beta, -alpha, info, z, nnue);
            board.annulla_mossa(&m, z);
            
            if score >= beta { return beta; }
            if score > alpha { alpha = score; }
        }
    }
    alpha
}