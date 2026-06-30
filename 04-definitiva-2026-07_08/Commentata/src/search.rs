use crate::board::{Scacchiera, Mossa};
use crate::tt::{TranspositionTable, Bound};
use crate::zobrist::ZobristKeys;
use crate::nnue::LunaNNUE;
use std::time::Instant;

const MAX_PLY: usize = 64;

/// Rappresenta la linea principale (PV - Principal Variation).
/// Utilizza un array a dimensione fissa per evitare allocazioni dinamiche (heap) 
/// durante la ricerca, massimizzando le prestazioni.
#[derive(Clone)]
pub struct PvLine {
    pub moves: [Mossa; MAX_PLY],
    pub len: usize,
}

impl PvLine {
    pub fn new() -> Self {
        PvLine {
            moves: [Mossa::null(); MAX_PLY],
            len: 0,
        }
    }
}

/// Contiene le informazioni sullo stato corrente della ricerca.
pub struct SearchInfo {
    pub start_time: Instant,
    pub hard_limit: u128,
    pub soft_limit: u128,
    pub depth_limit: i32,
    pub nodes: u64,
    pub stopped: bool,
    /// Array bi-dimensionale per le Killer Moves. 
    /// Memorizza fino a due mosse che hanno causato un cutoff per ogni livello (ply) dell'albero.
    pub killer_moves: [[Mossa; 2]; MAX_PLY],
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
            // Inizializza tutte le killer moves a mossa nulla
            killer_moves: [[Mossa::null(); 2]; MAX_PLY],
        }
    }

    /// Verifica periodicamente (ogni 2048 nodi) se il tempo a disposizione è esaurito.
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

/// Punto di ingresso principale della ricerca.
/// Utilizza l'Iterative Deepening per esplorare l'albero progressivamente.
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

    // Inizializzazione della finestra di ricerca (Aspiration Window)
    let mut alpha = -50000;
    let mut beta = 50000;

    for depth in 1..=info.depth_limit {
        let mut pv_line = PvLine::new();
        
        // Loop per l'Aspiration Window: se il punteggio esce dai limiti previsti,
        // allarghiamo i bound e ripetiamo la ricerca alla stessa profondità.
        loop {
            score = negamax(board, depth, 0, alpha, beta, info, tt, z, nnue, true, &mut pv_line);
            
            if info.stopped { break; }

            // Controllo dei bound per la finestra di aspirazione
            if score <= alpha || score >= beta {
                alpha = -50000;
                beta = 50000;
                continue; // Finestra fallita, riprova con bound infiniti
            }
            
            // Finestra esatta: prepariamo i bound ristretti per la prossima profondità
            alpha = score - 50;
            beta = score + 50;
            break; 
        }

        if info.stopped && depth > 1 { break; }

        // Gestione dell'output e della logica di stabilità del tempo
        if pv_line.len > 0 {
            best_move = pv_line.moves[0];
            let elapsed = info.start_time.elapsed().as_millis();
            
            if best_move.data == last_best_move.data {
                stability_counter += 1;
            } else {
                last_best_move = best_move;
                stability_counter = 0;
            }

            // Time management flessibile: si ferma se la mossa migliore è stabile
            if elapsed > info.soft_limit && (stability_counter >= 3 || depth > 8) {
                info.stopped = true;
            }

            let nps = if elapsed > 0 { info.nodes as u128 * 1000 / elapsed } else { 0 };
            
            print!("info depth {} score cp {} nodes {} nps {} time {} pv", 
                depth, score, info.nodes, nps, elapsed);
            for i in 0..pv_line.len { 
                print!(" {}", pv_line.moves[i].to_uci()); 
            }
            println!();
        }
        
        if info.stopped { break; }
    }

    // Fallback di sicurezza: se non troviamo nulla, restituiamo la prima mossa legale
    if best_move.is_null() {
        let legali = board.genera_mosse_legali(z);
        if !legali.is_empty() { best_move = legali[0]; }
    }

    (best_move, score)
}

/// Algoritmo Negamax ricorsivo con potatura Alpha-Beta e Principal Variation Search (PVS).
fn negamax(
    board: &mut Scacchiera, 
    depth: i32,
    ply: usize, 
    mut alpha: i32, 
    mut beta: i32, 
    info: &mut SearchInfo,
    tt: &mut TranspositionTable,
    z: &ZobristKeys,
    nnue: Option<&LunaNNUE>,
    allow_null: bool,
    pv_line: &mut PvLine
) -> i32 {
    pv_line.len = 0;

    if info.check_time() { return 0; }
    info.nodes += 1;

    let pv_node = beta - alpha > 1; 

    // Condizioni di patta
    if board.ply > 0 && (board.is_repetition() || board.rule_50 >= 100) {
        return 0;
    }

    // Interrogazione della Transposition Table (TT)
    if let Some(entry) = tt.probe(board.hash, depth, alpha, beta) {
        if !pv_node { return entry; }
    }
    
    let tt_move = tt.get_move(board.hash);
    let in_check = board.in_scacco();
    let new_depth = if in_check { depth + 1 } else { depth }; // Check Extension

    // Se raggiungiamo il limite di profondità, passiamo alla Quiescence Search
    if new_depth <= 0 {
        return quiescence(board, alpha, beta, info, z, nnue);
    }

    // Null Move Pruning (Taglio della mossa nulla)
    if allow_null && !pv_node && new_depth >= 3 && !in_check {
        let static_eval = crate::evaluation::evaluate(board);
        if static_eval >= beta {
            let undo = board.fai_mossa_nulla(z);
            let mut null_pv = PvLine::new(); 
            let null_val = -negamax(board, new_depth - 4, ply + 1, -beta, -beta + 1, info, tt, z, nnue, false, &mut null_pv);
            board.annulla_mossa_nulla(undo, z);
            if null_val >= beta { return beta; }
        }
    }

    let mut legal_moves = board.genera_mosse_legali(z);
    
    // Scacco matto o Stallo
    if legal_moves.is_empty() {
        return if in_check { -49000 + (board.ply as i32) } else { 0 };
    }

    // Limite di sicurezza per l'indicizzazione delle killer moves
    let safe_ply = if ply < MAX_PLY { ply } else { MAX_PLY - 1 };
    
    // Ordinamento delle mosse per massimizzare l'efficienza della potatura
    crate::movegen::ordina_mosse(&mut legal_moves, board, tt_move, &info.killer_moves[safe_ply]);

    let mut best_val = -50000;
    let mut flag = Bound::Alpha;
    let mut moves_searched = 0;
    let mut child_pv = PvLine::new();

    for m in legal_moves {
        if board.esegui_mossa(&m, z) {
            moves_searched += 1;
            let mut val;

            // Principal Variation Search (PVS)
            if moves_searched == 1 {
                // Ricerca a finestra intera per la prima mossa (presunta migliore)
                val = -negamax(board, new_depth - 1, ply + 1, -beta, -alpha, info, tt, z, nnue, true, &mut child_pv);
            } else {
                // Ricerca a finestra chiusa (Zero Window Search) per dimostrare che le altre mosse sono peggiori
                val = -negamax(board, new_depth - 1, ply + 1, -alpha - 1, -alpha, info, tt, z, nnue, true, &mut child_pv);
                
                // Se la mossa si rivela migliore del previsto, ricerchiamo con la finestra intera
                if val > alpha && val < beta {
                    val = -negamax(board, new_depth - 1, ply + 1, -beta, -alpha, info, tt, z, nnue, true, &mut child_pv);
                }
            }

            board.annulla_mossa(&m, z);

            if info.stopped { return 0; }

            if val > best_val {
                best_val = val;
                
                // Aggiornamento efficiente della PV Line
                pv_line.moves[0] = m;
                pv_line.moves[1..child_pv.len + 1].copy_from_slice(&child_pv.moves[0..child_pv.len]);
                pv_line.len = child_pv.len + 1;
            }

            if val > alpha {
                alpha = val;
                flag = Bound::Exact;
            }

            // Beta Cutoff (Potatura)
            if alpha >= beta {
                // Salvataggio della Killer Move (se non è cattura/promozione)
                if !m.is_cattura() && !m.is_promozione() && safe_ply < MAX_PLY {
                    if info.killer_moves[safe_ply][0].data != m.data {
                        info.killer_moves[safe_ply][1] = info.killer_moves[safe_ply][0];
                        info.killer_moves[safe_ply][0] = m;
                    }
                }

                tt.store(board.hash, depth, beta, Bound::Beta, m);
                return beta;
            }
        }
    }

    let best_move_to_store = if pv_line.len > 0 { pv_line.moves[0] } else { Mossa::null() };
    tt.store(board.hash, depth, best_val, flag, best_move_to_store);
    
    best_val
}

/// Quiescence Search: esplora solo le mosse forzanti (catture/promozioni) 
/// per evitare l'effetto orizzonte e stabilizzare la valutazione.
fn quiescence(
    board: &mut Scacchiera, 
    mut alpha: i32, 
    beta: i32, 
    info: &mut SearchInfo, 
    z: &ZobristKeys, 
    nnue: Option<&LunaNNUE>
) -> i32 {
    info.nodes += 1;
    
    // Stand Pat: la valutazione statica prima di provare qualsiasi mossa
    let stand_pat = crate::evaluation::evaluate(board);
    if stand_pat >= beta { return beta; }
    if stand_pat > alpha { alpha = stand_pat; }

    let mut moves = board.genera_mosse_legali(z);
    moves.retain(|m| m.is_cattura() || m.is_promozione()); 
    
    // In Q-Search passiamo array vuoti per le killer moves (non usate qui)
    crate::movegen::ordina_mosse(&mut moves, board, Mossa::null(), &[Mossa::null(); 2]);

    for m in moves {
        if board.esegui_mossa(&m, z) {
            let score = -quiescence(board, -beta, -alpha, info, z, nnue);
            board.annulla_mossa(&m, z);
            
            if score >= beta { return beta; }
            if score > alpha { alpha = score; }
        }
    }
    alpha
}