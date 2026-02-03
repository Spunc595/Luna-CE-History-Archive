use crate::board::{Scacchiera, Mossa};
use crate::tt::{TranspositionTable, Bound};
use crate::zobrist::ZobristKeys;
use crate::nnue::LunaNNUE;
use std::time::Instant;

pub struct SearchInfo {
    pub start_time: Instant,
    pub hard_limit: u128, // Tempo massimo assoluto
    pub soft_limit: u128, // Tempo ottimale (se la mossa è stabile)
    pub depth_limit: i32,
    pub nodes: u64,
    pub stopped: bool,
}

impl SearchInfo {
    pub fn new(time_limit: u128, depth_limit: i32) -> Self {
        // Strategia: puntiamo a usare circa il 60% del tempo assegnato come "Soft Limit".
        // Se la mossa è chiara, ci fermiamo lì. Se è complicata, usiamo tutto fino all'Hard Limit.
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
        // Controlla ogni 2048 nodi per non pesare sulle performance
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
    
    // Variabili per la gestione della stabilità della mossa (Time Management)
    let mut last_best_move = Mossa::null();
    let mut stability_counter = 0;

    // Aspiration Windows iniziali
    let mut alpha = -50000;
    let mut beta = 50000;

    for depth in 1..=info.depth_limit {
        let (val, pv) = negamax(board, depth, alpha, beta, info, tt, z, nnue, true);
        
        if info.stopped {
            break;
        }

        score = val;

        // Gestione Aspiration Window (Fail Low / Fail High)
        if score <= alpha || score >= beta {
            // Se fallisce, allarga la finestra a infinito
            alpha = -50000;
            beta = 50000;
        } else {
            // Se ok, stringe la finestra per la prossima ricerca
            alpha = score - 50;
            beta = score + 50;
        }

        if !pv.is_empty() {
            best_move = pv[0];
            
            // --- TIME MANAGEMENT INTELLIGENTE ---
            let elapsed = info.start_time.elapsed().as_millis();
            
            // Controlliamo se la mossa migliore è stabile (non cambia da diverse profondità)
            if best_move.data == last_best_move.data {
                stability_counter += 1;
            } else {
                last_best_move = best_move;
                stability_counter = 0;
            }

            // Se abbiamo superato il tempo ottimale (Soft Limit)
            if elapsed > info.soft_limit {
                // Se la mossa è molto stabile o siamo già profondi, fermati.
                if stability_counter >= 3 || (depth > 8 && stability_counter >= 1) {
                    info.stopped = true;
                }
            }

            // Output UCI
            let nps = if elapsed > 0 { info.nodes as u128 * 1000 / elapsed } else { 0 };
            print!("info depth {} score cp {} nodes {} nps {} time {} seldepth {}", 
                depth, score, info.nodes, nps, elapsed, depth); 
            
            print!(" pv");
            for m in &pv {
                print!(" {}", m.to_uci());
            }
            println!();
        }
        
        if info.stopped {
            break;
        }
    }

    // --- SAFETY CHECK (Il Fix per il Crash) ---
    // Prima di restituire la mossa, verifichiamo ASSOLUTAMENTE che sia legale.
    // Se la TT è corrotta o c'è una collisione di hash, best_move potrebbe essere illegale.
    let legali = board.genera_mosse_legali(z);
    
    if legali.is_empty() {
        // Matto o Stallo
        return (Mossa::null(), score);
    }

    let mut is_legal = false;
    for m in &legali {
        if m.data == best_move.data {
            is_legal = true;
            break;
        }
    }

    if !is_legal {
        // Se best_move non è nella lista delle mosse legali, è un errore della TT.
        // Fallback: usiamo la prima mossa legale disponibile per non crashare.
        // (Opzionale: potremmo stampare un errore di debug qui)
        best_move = legali[0];
    }
    // ----------------------------------------

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
    
    if info.check_time() {
        return (0, Vec::new());
    }

    info.nodes += 1;
    let pv_node = beta - alpha > 1; 

    // 1. TT Probe
    let tt_move = if let Some(entry) = tt.probe(board.hash, depth, alpha, beta) {
        // Se non siamo in un nodo PV, possiamo accettare il cutoff
        if !pv_node {
            return (entry, Vec::new());
        }
        tt.get_move(board.hash)
    } else {
        tt.get_move(board.hash)
    };
    
    let in_check = board.in_scacco();
    let extension = if in_check { 1 } else { 0 };
    let new_depth = depth + extension;

    // 2. Quiescence Search (se profondità finita)
    if new_depth <= 0 {
        let eval = quiescence(board, alpha, beta, info, z, nnue);
        return (eval, Vec::new());
    }

    // 3. Null Move Pruning (NMP)
    // Non farlo se: siamo in PV, siamo sotto scacco, o la posizione è rischiosa (beta alto)
    if allow_null && !pv_node && new_depth >= 3 && !in_check && beta.abs() < 20000 {
        let static_eval = crate::evaluation::evaluate(board, nnue);
        if static_eval >= beta {
            let undo = board.fai_mossa_nulla(z);
            let r = 3 + new_depth / 6;
            let (null_val, _) = negamax(board, new_depth - r, -beta, -beta + 1, info, tt, z, nnue, false);
            board.annulla_mossa_nulla(undo, z);

            if info.stopped { return (0, Vec::new()); }

            if null_val >= beta {
                return (beta, Vec::new());
            }
        }
    }

    let mut legal_moves = board.genera_mosse_legali(z);
    if legal_moves.is_empty() {
        if in_check {
            return (-49000 + (board.ply as i32), Vec::new()); // Matto
        } else {
            return (0, Vec::new()); // Stallo
        }
    }

    // 4. Move Ordering
    crate::movegen::ordina_mosse(&mut legal_moves, board, tt_move);

    let mut best_val = -50000;
    let mut best_pv = Vec::new();
    let mut flag = Bound::Alpha;
    let mut moves_searched = 0;

    for m in legal_moves {
        let mut new_board = board.clone();
        if new_board.esegui_mossa(&m, z) {
            moves_searched += 1;
            
            let val;
            let child_pv;

            // 5. PVS & LMR
            if moves_searched == 1 {
                // Prima mossa: ricerca completa
                let res = negamax(&mut new_board, new_depth - 1, -beta, -alpha, info, tt, z, nnue, true);
                val = -res.0;
                child_pv = res.1;
            } else {
                // Late Move Reduction (LMR)
                let mut reduction = 0;
                if new_depth >= 3 
                   && moves_searched > 3 
                   && !m.is_cattura() 
                   && !m.is_promozione() 
                   && !in_check 
                   && !pv_node // Non ridurre mai nodi PV
                {
                    reduction = 1;
                    if new_depth > 8 && moves_searched > 20 { reduction = 2; }
                }

                // Ricerca Null Window con riduzione
                let mut res = negamax(&mut new_board, new_depth - 1 - reduction, -alpha - 1, -alpha, info, tt, z, nnue, true);
                let mut score = -res.0;

                // Re-Search se LMR fallisce (la mossa era buona)
                if reduction > 0 && score > alpha {
                    res = negamax(&mut new_board, new_depth - 1, -alpha - 1, -alpha, info, tt, z, nnue, true);
                    score = -res.0;
                }

                // Se la mossa migliora alpha, ricerca Full Window
                if score > alpha && score < beta {
                    res = negamax(&mut new_board, new_depth - 1, -beta, -alpha, info, tt, z, nnue, true);
                    score = -res.0;
                }
                
                val = score;
                child_pv = res.1;
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
                flag = Bound::Beta;
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