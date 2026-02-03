use crate::board::{Scacchiera, Mossa};
use crate::tt::{TranspositionTable, Bound};
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
        let (val, pv) = negamax(board, depth, -50000, 50000, info, tt, z, nnue, true);
        
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
    nnue: Option<&LunaNNUE>,
    allow_null: bool, // Parametro per evitare null move ricorsive infinite
) -> (i32, Vec<Mossa>) {
    if info.check_time() {
        return (0, Vec::new());
    }

    info.nodes += 1;

    // TT Probe
    let tt_move = if let Some(entry) = tt.probe(board.hash, depth, alpha, beta) {
        // Se abbiamo un cutoff dalla TT, ritorniamo (per ora senza PV per semplicità)
        // Ma per il move ordering ci serve la mossa
        tt.get_move(board.hash)
    } else {
        Mossa::null()
    };
    
    // Ricalcolo check per sicurezza
    let in_check = board.in_scacco();

    if depth <= 0 {
        let eval = quiescence(board, alpha, beta, info, z, nnue);
        return (eval, Vec::new());
    }

    // --- NULL MOVE PRUNING (NMP) ---
    // Se la profondità è abbastanza alta, non siamo in check, e la valutazione statica è buona...
    if allow_null && depth >= 3 && !in_check {
        let static_eval = crate::evaluation::evaluate(board, nnue);
        if static_eval >= beta {
            // Proviamo a passare il turno
            let undo = board.fai_mossa_nulla(z);
            // Ricerca a profondità ridotta (R=3)
            let (null_val, _) = negamax(board, depth - 3, -beta, -beta + 1, info, tt, z, nnue, false);
            board.annulla_mossa_nulla(undo, z);

            if info.stopped { return (0, Vec::new()); }

            if null_val >= beta {
                return (beta, Vec::new()); // Cutoff! Posizione troppo forte
            }
        }
    }

    let mut legal_moves = board.genera_mosse_legali(z);
    if legal_moves.is_empty() {
        if in_check {
            return (-49000 + (board.ply as i32), Vec::new());
        } else {
            return (0, Vec::new());
        }
    }

    // --- MOVE ORDERING ---
    // Usiamo la mossa della TT e MVV-LVA
    crate::movegen::ordina_mosse(&mut legal_moves, board, tt_move);

    let mut best_val = -50000;
    let mut best_pv = Vec::new();
    let mut flag = Bound::Alpha;

    for m in legal_moves {
        let mut new_board = board.clone();
        if new_board.esegui_mossa(&m, z) {
            let (val, child_pv) = negamax(&mut new_board, depth - 1, -beta, -alpha, info, tt, z, nnue, true);
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
                flag = Bound::Exact;
            }

            if alpha >= beta {
                flag = Bound::Beta;
                // Store killer moves here if implemented
                break;
            }
        }
    }

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

    let mut moves = board.genera_mosse(); 
    moves.retain(|m| m.is_cattura());
    
    // Anche in quiescenza ordiniamo le mosse (MVV-LVA è cruciale qui!)
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