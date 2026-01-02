use crate::board::{Scacchiera, Mossa, Colore};
use crate::move_ordering::sort_moves;
use crate::nnue::Network; // Assicurati che questo path sia corretto
use std::time::Instant;

// Valori per l'infinito e scacco matto
const INF: i32 = 50000;
const MATE_SCORE: i32 = 49000;

pub struct SearchInfo {
    pub nodes: u64,
    pub start_time: Instant,
    pub time_limit_ms: u128,
    pub stop: bool,
}

// Funzione principale chiamata dall'esterno
pub fn search_position(
    board: &mut Scacchiera, 
    max_depth: u8, 
    time_limit_ms: u128,
    net: &Network // Passiamo la rete neurale
) -> Option<Mossa> {
    
    let mut info = SearchInfo {
        nodes: 0,
        start_time: Instant::now(),
        time_limit_ms,
        stop: false,
    };

    let mut best_move = None;

    // Iterative Deepening
    for depth in 1..=max_depth {
        // La Root Search restituisce il punteggio e la mossa migliore
        let (score, mv) = root_search(board, depth, &mut info, net);
        
        if info.stop {
            break;
        }

        best_move = mv;

        // Logging UCI
        let time_elapsed = info.start_time.elapsed().as_millis();
        let nps = if time_elapsed > 0 { (info.nodes as u128 * 1000) / time_elapsed } else { 0 };
        let mv_str = if let Some(m) = best_move { m.to_string() } else { "none".to_string() };
        
        println!("info depth {} score cp {} nodes {} time {} nps {} pv {}", 
            depth, score, info.nodes, time_elapsed, nps, mv_str);
    }

    best_move
}

fn root_search(
    board: &mut Scacchiera, 
    depth: u8, 
    info: &mut SearchInfo, 
    net: &Network
) -> (i32, Option<Mossa>) {
    
    let mut alpha = -INF;
    let beta = INF;
    let mut best_move = None;
    let mut best_score = -INF;

    // ATTENZIONE: Qui chiamo genera_mosse(). Se non esiste in board.rs, darà errore!
    let mut moves = board.genera_mosse(); 
    sort_moves(board, &mut moves);

    let mut legal_moves_count = 0;

    for mv in moves {
        let capture = board.get_piece_at(mv.a()); // Salviamo cosa stiamo per mangiare
        
        // 1. Eseguiamo la mossa
        if board.esegui_mossa(&mv, net) {
            
            // 2. CONTROLLO LEGALITÀ
            // Dopo aver mosso, il turno è passato all'avversario.
            // Quindi controlliamo se chi ha mosso (turno opposto) è sotto scacco.
            if board.re_in_scacco(board.turno.opposto()) {
                // Mossa illegale: lascia il Re in presa. Annulliamo e scartiamo.
                board.annulla_mossa(&mv, capture, net);
                continue;
            }

            legal_moves_count += 1;

            // 3. Ricorsione
            let score = -negamax(board, depth - 1, -beta, -alpha, info, net);
            
            // 4. Annulliamo
            board.annulla_mossa(&mv, capture, net);

            if info.stop {
                return (best_score, best_move);
            }

            if score > best_score {
                best_score = score;
                best_move = Some(mv);
            }

            if score > alpha {
                alpha = score;
            }
        }
    }

    // Se non abbiamo trovato mosse legali, controlliamo matto o stallo
    if legal_moves_count == 0 {
        return (evaluate_endgame(board), None);
    }

    (best_score, best_move)
}

fn negamax(
    board: &mut Scacchiera, 
    depth: u8, 
    mut alpha: i32, 
    beta: i32, 
    info: &mut SearchInfo,
    net: &Network
) -> i32 {
    
    // Check tempo ogni 2048 nodi
    if info.nodes & 2047 == 0 {
        if info.start_time.elapsed().as_millis() > info.time_limit_ms {
            info.stop = true;
        }
    }
    info.nodes += 1;

    if info.stop { return 0; }

    // Rilevamento patta per ripetizione andrebbe qui (opzionale per ora)

    if depth == 0 {
        // Qui andrebbe la Quiescence Search. Per ora usiamo la valutazione statica (NNUE)
        // Nota: evaluate_nnue deve esistere.
        return crate::evaluation::evaluate(board, net); 
    }

    let mut moves = board.genera_mosse();
    sort_moves(board, &mut moves);

    let mut best_score = -INF;
    let mut legal_moves_count = 0;

    for mv in moves {
        let capture = board.get_piece_at(mv.a());

        if board.esegui_mossa(&mv, net) {
            // Controllo legalità
            if board.re_in_scacco(board.turno.opposto()) {
                board.annulla_mossa(&mv, capture, net);
                continue;
            }

            legal_moves_count += 1;

            let score = -negamax(board, depth - 1, -beta, -alpha, info, net);
            
            board.annulla_mossa(&mv, capture, net);

            if info.stop { return 0; }

            if score > best_score {
                best_score = score;
            }

            if score > alpha {
                alpha = score;
            }
            if alpha >= beta {
                break; // Cutoff
            }
        }
    }

    if legal_moves_count == 0 {
        return evaluate_endgame(board);
    }

    best_score
}

fn evaluate_endgame(board: &Scacchiera) -> i32 {
    if board.re_in_scacco(board.turno) {
        // Scacco matto
        -MATE_SCORE 
    } else {
        // Stallo
        0
    }
}