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

    #[inline(always)]
    pub fn check_time(&mut self) -> bool {
        // Controlla il tempo ogni 2048 nodi
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

    // Finestre di aspirazione iniziali (Infinity)
    let mut alpha = -50000;
    let mut beta = 50000;

    for depth in 1..=info.depth_limit {
        // Chiamata alla ricerca principale
        let (val, pv) = negamax(board, depth, alpha, beta, info, tt, z, nnue, true);
        
        if info.stopped {
            break;
        }

        score = val;

        // Gestione Aspiration Window (Fail Low / Fail High)
        if score <= alpha || score >= beta {
            // Se il punteggio è fuori dalle aspettative, la finestra era troppo stretta.
            // Resettiamo a infinito per la prossima profondità per essere sicuri di trovare la verità.
            alpha = -50000;
            beta = 50000;
            
            // Nota: I motori avanzati qui farebbero una ri-ricerca immediata alla STESSA depth.
            // Per semplicità e stabilità, noi allarghiamo per la PROSSIMA depth, 
            // ma stampiamo comunque il PV trovato se valido.
        } else {
            // Se la ricerca è andata bene, stringiamo la finestra per la prossima volta (più veloce)
            alpha = score - 50;
            beta = score + 50;
        }

        if !pv.is_empty() {
            best_move = pv[0];
            
            let elapsed = info.start_time.elapsed().as_millis();
            let nps = if elapsed > 0 { info.nodes as u128 * 1000 / elapsed } else { 0 };
            
            // Output UCI standard
            print!("info depth {} score cp {} nodes {} nps {} time {} seldepth {}", 
                depth, score, info.nodes, nps, elapsed, depth); 
            
            print!(" pv");
            for m in &pv {
                print!(" {}", m.to_uci());
            }
            println!();
        }
    }

    // Fallback di sicurezza se non abbiamo trovato nulla (raro)
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
    allow_null: bool,
) -> (i32, Vec<Mossa>) {
    
    // 1. Controllo Tempo
    if info.check_time() {
        return (0, Vec::new());
    }

    info.nodes += 1;
    let pv_node = beta - alpha > 1; // Siamo in un nodo critico?

    // 2. Transposition Table Probe
    let tt_move = if let Some(entry) = tt.probe(board.hash, depth, alpha, beta) {
        // Se non siamo in un PV node, possiamo accettare il cutoff della TT
        if !pv_node {
            return (entry, Vec::new());
        }
        // In un PV node usiamo la TT solo per l'ordinamento delle mosse
        tt.get_move(board.hash)
    } else {
        tt.get_move(board.hash)
    };
    
    let in_check = board.in_scacco();

    // 3. Check Extension (Se siamo sotto scacco, estendiamo la ricerca)
    let extension = if in_check { 1 } else { 0 };
    let new_depth = depth + extension;

    // 4. Quiescence Search (se profondità esaurita)
    if new_depth <= 0 {
        let eval = quiescence(board, alpha, beta, info, z, nnue);
        return (eval, Vec::new());
    }

    // 5. Null Move Pruning (NMP)
    // DISABILITATO se in PV-Node o sotto scacco (per sicurezza tattica)
    if allow_null && !pv_node && new_depth >= 3 && !in_check && beta.abs() < 20000 {
        let static_eval = crate::evaluation::evaluate(board, nnue);
        if static_eval >= beta {
            let undo = board.fai_mossa_nulla(z);
            let r = 3 + new_depth / 6;
            // Ricerca con finestra nulla
            let (null_val, _) = negamax(board, new_depth - r, -beta, -beta + 1, info, tt, z, nnue, false);
            board.annulla_mossa_nulla(undo, z);

            if info.stopped { return (0, Vec::new()); }

            if null_val >= beta {
                return (beta, Vec::new()); // Cutoff
            }
        }
    }

    let mut legal_moves = board.genera_mosse_legali(z);
    
    // Gestione Matto / Stallo
    if legal_moves.is_empty() {
        if in_check {
            return (-49000 + (board.ply as i32), Vec::new()); // Matto
        } else {
            return (0, Vec::new()); // Stallo
        }
    }

    // 6. Ordinamento Mosse
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

            // 7. PVS (Principal Variation Search) e LMR (Late Move Reduction)
            if moves_searched == 1 {
                // Prima mossa (più promettente): Ricerca completa
                let res = negamax(&mut new_board, new_depth - 1, -beta, -alpha, info, tt, z, nnue, true);
                val = -res.0;
                child_pv = res.1;
            } else {
                // Mosse successive: Proviamo a ridurle (LMR)
                
                // Calcolo Riduzione
                let mut reduction = 0;
                // CONDIZIONI DI SICUREZZA PER LMR:
                // - Profondità deve essere decente
                // - Non deve essere una cattura
                // - Non deve essere una promozione
                // - Non deve essere sotto scacco
                // - NON deve essere un PV-Node (Novità importante!)
                if new_depth >= 3 
                   && moves_searched > 3 
                   && !m.is_cattura() 
                   && !m.is_promozione() 
                   && !in_check 
                   && !pv_node 
                {
                    reduction = 1;
                    if new_depth > 8 && moves_searched > 20 { reduction = 2; }
                }

                // Ricerca con finestra nulla (Null Window Search)
                // Usiamo depth ridotta
                let mut res = negamax(&mut new_board, new_depth - 1 - reduction, -alpha - 1, -alpha, info, tt, z, nnue, true);
                let mut score = -res.0;

                // Re-Search se LMR ha fallito (la mossa era buona!)
                if reduction > 0 && score > alpha {
                    res = negamax(&mut new_board, new_depth - 1, -alpha - 1, -alpha, info, tt, z, nnue, true);
                    score = -res.0;
                }

                // Se la mossa migliora alpha, dobbiamo fare la ricerca completa (Full Window)
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
                flag = Bound::Beta; // Cutoff Beta
                break; 
            }
        }
    }

    // Salva in Transposition Table
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
    
    // Stand-pat: valutazione statica corrente
    let stand_pat = crate::evaluation::evaluate(board, nnue);

    if stand_pat >= beta { return beta; }
    if stand_pat > alpha { alpha = stand_pat; }

    // Genera solo catture
    let mut moves = board.genera_mosse(); 
    moves.retain(|m| m.is_cattura());
    
    // Ordina (MVV-LVA è cruciale qui)
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