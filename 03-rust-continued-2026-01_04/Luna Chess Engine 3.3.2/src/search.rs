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
        if (self.nodes & 4095) == 0 {
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
    let mut alpha = -50000;
    let mut beta = 50000;

    for depth in 1..=info.depth_limit {
        let (val, pv) = negamax(board, depth, alpha, beta, info, tt, z, nnue, true);
        
        if info.stopped && depth > 1 { break; }

        score = val;
        // Finestra di aspirazione
        if score <= alpha || score >= beta { 
            alpha = -50000; beta = 50000; 
        } else { 
            alpha = score - 40; beta = score + 40; 
        }

        if !pv.is_empty() {
            best_move = pv[0];
            let elapsed = info.start_time.elapsed().as_millis();
            let nps = if elapsed > 0 { info.nodes as u128 * 1000 / elapsed } else { 0 };
            
            println!("info depth {} score cp {} nodes {} nps {} time {} pv {}", 
                depth, score, info.nodes, nps, elapsed, best_move.to_uci());
            
            if elapsed > info.soft_limit && depth > 10 { info.stopped = true; }
        }
        if info.stopped { break; }
    }
    (best_move, score)
}

fn negamax(board: &mut Scacchiera, depth: i32, mut alpha: i32, beta: i32, info: &mut SearchInfo, tt: &mut TranspositionTable, z: &ZobristKeys, nnue: Option<&LunaNNUE>, allow_null: bool) -> (i32, Vec<Mossa>) {
    if info.check_time() { return (0, Vec::new()); }
    
    // Repetizioni e Rule 50
    if board.ply > 0 && (board.is_repetition() || board.rule_50 >= 100) { 
        return (0, Vec::new()); 
    }

    // TT Probe
    if let Some(entry) = tt.probe(board.hash, depth, alpha, beta) { 
        return (entry, Vec::new()); 
    }
    
    let in_check = board.in_scacco();
    if depth <= 0 { 
        return (quiescence(board, alpha, beta, info, z, nnue), Vec::new()); 
    }

    info.nodes += 1;
    let tt_move = tt.get_move(board.hash);

    // Null Move Pruning
    if allow_null && depth >= 3 && !in_check {
        let _undo = board.fai_mossa_nulla(z);
        let (null_val, _) = negamax(board, depth - 4, -beta, -beta + 1, info, tt, z, nnue, false);
        board.annulla_mossa_nulla(_undo, z);
        if -null_val >= beta { return (beta, Vec::new()); }
    }

    let mut legal_moves = board.genera_mosse_legali(z);
    crate::movegen::ordina_mosse(&mut legal_moves, board, tt_move, depth, info);

    let mut best_val = -50000;
    let mut best_pv = Vec::new();
    let mut moves_searched = 0;

    for m in legal_moves {
        // --- SOSTITUITO CLONE CON MAKE/UNMAKE ---
        if board.esegui_mossa(&m, z) {
            moves_searched += 1;
            
            let (mut val, mut child_pv);

            if moves_searched == 1 {
                // Ricerca completa per la prima mossa (PV)
                let res = negamax(board, depth - 1, -beta, -alpha, info, tt, z, nnue, true);
                val = -res.0;
                child_pv = res.1;
            } else {
                // LMR (Late Move Reductions)
                let r = if depth >= 3 && moves_searched > 4 && !m.is_cattura() { 1 } else { 0 };
                
                // Zero-Window Search
                let res = negamax(board, depth - 1 - r, -alpha - 1, -alpha, info, tt, z, nnue, true);
                val = -res.0;
                child_pv = Vec::new();

                // Re-search se la mossa sembra buona
                if val > alpha && (r > 0 || val < beta) {
                    let res = negamax(board, depth - 1, -beta, -alpha, info, tt, z, nnue, true);
                    val = -res.0;
                    child_pv = res.1;
                }
            }

            board.annulla_mossa(&m, z); // --- TORNA INDIETRO ---

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
                    let d = depth as usize;
                    if d < MAX_DEPTH { 
                        info.history_table[board.turno.indice()][m.da()][m.a()] += depth * depth; 
                    }
                }
                tt.store(board.hash, depth, beta, Bound::Beta, m);
                return (beta, Vec::new());
            }
        }
    }

    if moves_searched == 0 {
        return (if in_check { -49000 + (board.ply as i32) } else { 0 }, Vec::new());
    }

    tt.store(board.hash, depth, best_val, Bound::Exact, if !best_pv.is_empty() { best_pv[0] } else { Mossa::null() });
    (best_val, best_pv)
}

fn quiescence(board: &mut Scacchiera, mut alpha: i32, beta: i32, info: &mut SearchInfo, z: &ZobristKeys, nnue: Option<&LunaNNUE>) -> i32 {
    let stand_pat = crate::evaluation::evaluate(board);
    if stand_pat >= beta { return beta; }
    if stand_pat > alpha { alpha = stand_pat; }

    let mut moves = board.genera_mosse_legali(z);
    moves.retain(|m| m.is_cattura());
    
    // Ordinamento leggero MVV-LVA
    moves.sort_unstable_by_key(|m| {
        let victim = board.pezzo_in(m.a()).unwrap_or(0);
        -(victim as i32)
    });

    for m in moves {
        // --- ANCHE QUI: MAKE/UNMAKE INVECE DI CLONE ---
        if board.esegui_mossa(&m, z) {
            info.nodes += 1;
            let score = -quiescence(board, -beta, -alpha, info, z, nnue);
            board.annulla_mossa(&m, z);
            
            if score >= beta { return beta; }
            if score > alpha { alpha = score; }
        }
    }
    alpha
}