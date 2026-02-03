use crate::board::{Scacchiera, Mossa};
use crate::tt::{TranspositionTable, Bound};
use crate::zobrist::ZobristKeys;
use crate::nnue::LunaNNUE;
use std::time::Instant;

pub struct SearchInfo {
    pub start_time: Instant,
    pub hard_limit: u128,
    pub soft_limit: u128,
    pub depth_limit: i32,
    pub nodes: u64,
    pub stopped: bool,
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

        // Aspiration Window (base)
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
            
            // Gestione del tempo soft
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
            
            // Stampa info UCI
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

    // --- CORREZIONE 1: Controllo Ripetizione e 50 Mosse ---
    // Verifica se siamo in una posizione ripetuta o regola 50 mosse.
    // Nota: 'board.ply' deve indicare la profondità dall'inizio della ricerca.
    // Se board.ply > 0 significa che non siamo alla radice (alla radice dobbiamo giocare per forza).
    // Assumiamo che Scacchiera abbia un metodo is_repetition() o is_draw().
    if board.ply > 0 && (board.is_repetition() || board.rule_50 >= 100) {
        return (0, Vec::new());
    }
    // ------------------------------------------------------

    if let Some(entry) = tt.probe(board.hash, depth, alpha, beta) {
        if !pv_node { return (entry, Vec::new()); }
    }
    
    let tt_move = tt.get_move(board.hash);
    let in_check = board.in_scacco();
    let new_depth = if in_check { depth + 1 } else { depth };

    if new_depth <= 0 {
        return (quiescence(board, alpha, beta, info, z, nnue), Vec::new());
    }

    // Null Move Pruning
    if allow_null && !pv_node && new_depth >= 3 && !in_check {
        let static_eval = crate::evaluation::evaluate(board, nnue);
        if static_eval >= beta {
            let undo = board.fai_mossa_nulla(z);
            let (null_val, _) = negamax(board, new_depth - 4, -beta, -beta + 1, info, tt, z, nnue, false);
            board.annulla_mossa_nulla(undo, z);
            if null_val >= beta { return (beta, Vec::new()); }
        }
    }

    let mut legal_moves = board.genera_mosse_legali(z);
    if legal_moves.is_empty() {
        return (if in_check { -49000 + (board.ply as i32) } else { 0 }, Vec::new());
    }

    crate::movegen::ordina_mosse(&mut legal_moves, board, tt_move);

    let mut best_val = -50000;
    let mut best_pv = Vec::new();
    let mut flag = Bound::Alpha;
    let mut moves_searched = 0;

    for m in legal_moves {
        let mut new_board = board.clone();
        if new_board.esegui_mossa(&m, z) {
            moves_searched += 1;
            let (mut val, child_pv);

            if moves_searched == 1 {
                let res = negamax(&mut new_board, new_depth - 1, -beta, -alpha, info, tt, z, nnue, true);
                val = -res.0; child_pv = res.1;
            } else {
                let res = negamax(&mut new_board, new_depth - 1, -alpha - 1, -alpha, info, tt, z, nnue, true);
                val = -res.0;
                if val > alpha && val < beta {
                    let res = negamax(&mut new_board, new_depth - 1, -beta, -alpha, info, tt, z, nnue, true);
                    val = -res.0; child_pv = res.1;
                } else { child_pv = res.1; }
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

            if alpha >= beta {
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
    let stand_pat = crate::evaluation::evaluate(board, nnue);
    if stand_pat >= beta { return beta; }
    if stand_pat > alpha { alpha = stand_pat; }

    let mut moves = board.genera_mosse_legali(z);
    
    // --- CORREZIONE 2: Promozioni ---
    // PRIMA: moves.retain(|m| m.is_cattura());
    // ORA: Manteniamo anche le promozioni, altrimenti il motore smette di calcolare
    // proprio quando deve promuovere!
    moves.retain(|m| m.is_cattura() || m.is_promozione()); 
    // --------------------------------

    crate::movegen::ordina_mosse(&mut moves, board, Mossa::null());

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