use crate::board::{Scacchiera, Mossa};
use crate::tt::{TranspositionTable, Bound}; // Uso Bound ora
use crate::zobrist::ZobristKeys;
use crate::nnue::LunaNNUE;
use std::time::Instant;

pub struct SearchInfo {
    pub start_time: Instant,
    pub time_limit: u128,
    pub depth_limit: i32,
    pub nodes: u64,
    pub stopped: bool,
}

impl SearchInfo {
    pub fn new(time_limit: u128, depth_limit: i32) -> Self {
        SearchInfo {
            start_time: Instant::now(),
            time_limit,
            depth_limit,
            nodes: 0,
            stopped: false,
        }
    }

    pub fn check_time(&mut self) -> bool {
        if (self.nodes & 2047) == 0 {
            let elapsed = self.start_time.elapsed().as_millis();
            if elapsed >= self.time_limit {
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

    for depth in 1..=info.depth_limit {
        let (val, pv) = negamax(board, depth, -50000, 50000, info, tt, z, nnue);
        
        if info.stopped {
            break;
        }

        score = val;
        if !pv.is_empty() {
            best_move = pv[0];
            
            let elapsed = info.start_time.elapsed().as_millis();
            let nps = if elapsed > 0 { info.nodes as u128 * 1000 / elapsed } else { 0 };
            
            print!("info depth {} score cp {} nodes {} nps {} time {} seldepth {}", 
                depth, score, info.nodes, nps, elapsed, depth); 
            
            print!(" pv");
            for m in &pv {
                print!(" {}", m.to_uci());
            }
            println!();
        }
    }

    if best_move.is_null() {
        let legali = board.genera_mosse_legali(z);
        if !legali.is_empty() {
            return (legali[0], score);
        }
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
    nnue: Option<&LunaNNUE>
) -> (i32, Vec<Mossa>) {
    if info.check_time() {
        return (0, Vec::new());
    }

    info.nodes += 1;

    // TT Probe
    if let Some(val) = tt.probe(board.hash, depth, alpha, beta) {
         // In un motore completo qui restituiremmo la mossa della TT se cutoff
         return (val, Vec::new()); 
    }

    if depth <= 0 {
        let eval = quiescence(board, alpha, beta, info, z, nnue);
        return (eval, Vec::new());
    }

    let mut legal_moves = board.genera_mosse_legali(z);
    if legal_moves.is_empty() {
        if board.in_scacco() { // Usa il nuovo helper
            return (-49000 + (board.ply as i32), Vec::new());
        } else {
            return (0, Vec::new());
        }
    }

    // Ordinamento mosse con mossa TT
    let tt_move = tt.get_move(board.hash);
    // (Qui potresti implementare l'ordinamento usando tt_move)
    crate::movegen::ordina_mosse(&mut legal_moves, board);

    let mut best_val = -50000;
    let mut best_pv = Vec::new();
    let mut flag = Bound::Alpha; // Usa Bound::Alpha

    for m in legal_moves {
        let mut new_board = board.clone();
        if new_board.esegui_mossa(&m, z) {
            let (val, child_pv) = negamax(&mut new_board, depth - 1, -beta, -alpha, info, tt, z, nnue);
            let val = -val;

            if info.stopped { return (0, Vec::new()); }

            if val > best_val {
                best_val = val;
                let mut pv = vec![m];
                pv.extend(child_pv);
                best_pv = pv;
            }

            if val > alpha {
                alpha = val;
                flag = Bound::Exact; // Usa Bound::Exact
            }

            if alpha >= beta {
                flag = Bound::Beta; // Usa Bound::Beta
                break;
            }
        }
    }

    // Store corretto
    tt.store(board.hash, depth, best_val, flag, if !best_pv.is_empty() { best_pv[0] } else { Mossa::null() });
    (best_val, best_pv)
}

fn quiescence(
    board: &mut Scacchiera, 
    mut alpha: i32, 
    beta: i32, 
    info: &mut SearchInfo,
    z: &ZobristKeys,
    nnue: Option<&LunaNNUE>
) -> i32 {
    info.nodes += 1;
    
    let stand_pat = crate::evaluation::evaluate(board, nnue);

    if stand_pat >= beta { return beta; }
    if stand_pat > alpha { alpha = stand_pat; }

    // CORRETTO: Rimosso 'z' da genera_mosse
    let mut moves = board.genera_mosse(); 
    moves.retain(|m| m.is_cattura());
    crate::movegen::ordina_mosse(&mut moves, board);

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