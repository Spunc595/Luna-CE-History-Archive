use crate::board::{Scacchiera, Mossa, Pezzo, MoveFlag};
use crate::evaluation;
use std::time::Instant;

pub const INFINITY: i32 = 50000;
pub const MATE_VALUE: i32 = 49000;
pub const MAX_PLY: usize = 64;

pub struct SearchInfo {
    pub start_time: Instant,
    pub time_limit_ms: u128,
    pub nodes: u64,
    pub stop: bool,
}

// Struttura per memorizzare la linea principale (PV)
#[derive(Clone, Copy)]
pub struct PvLine {
    pub moves: [Mossa; MAX_PLY],
    pub len: usize,
}

impl PvLine {
    pub fn new() -> Self {
        Self {
            moves: [Mossa { data: 0 }; MAX_PLY],
            len: 0,
        }
    }
}

// Funzione wrapper per UCI
pub fn search_root(board: &mut Scacchiera, depth: i32, info: &mut SearchInfo) -> (i32, Mossa) {
    let mut root_pv = PvLine::new();
    let score = search(board, depth, -INFINITY, INFINITY, 0, info, &mut root_pv);
    
    // Se abbiamo trovato una mossa, la ritorniamo, altrimenti ne prendiamo una a caso (non dovrebbe accadere)
    let best_move = if root_pv.len > 0 {
        root_pv.moves[0]
    } else {
        board.genera_mosse()[0]
    };

    (score, best_move)
}

// Nota: Aggiunto parametro 'ply' (distanza dalla radice) e 'pv' (linea di variante)
pub fn search(board: &mut Scacchiera, mut depth: i32, mut alpha: i32, beta: i32, ply: usize, info: &mut SearchInfo, pv: &mut PvLine) -> i32 {
    // 1. Controllo Tempo
    if info.nodes & 2047 == 0 {
        if info.start_time.elapsed().as_millis() > info.time_limit_ms {
            info.stop = true;
        }
    }
    if info.stop { return 0; }
    info.nodes += 1;

    // Reset PV per questo nodo
    pv.len = 0;

    if ply >= MAX_PLY { return evaluation::evaluate_classical(board); }

    // 2. Check Extension
    let in_check = board.re_in_scacco(board.turno);
    if in_check { depth += 1; }

    // 3. Quiescence
    if depth <= 0 {
        return quiescence(board, alpha, beta, info);
    }

    // 4. Null Move Pruning
    if depth >= 3 && !in_check && beta != INFINITY {
        let static_eval = evaluation::evaluate_classical(board);
        if static_eval >= beta {
            board.make_null_move();
            // Passiamo una pv dummy perché non ci interessa la linea del null move
            let mut null_pv = PvLine::new();
            let score = -search(board, depth - 3, -beta, -beta + 1, ply + 1, info, &mut null_pv);
            board.unmake_null_move();
            if info.stop { return 0; }
            if score >= beta { return beta; }
        }
    }

    // 5. Generazione Mosse
    let mut moves = board.genera_mosse();
    moves.sort_by_cached_key(|m| -score_move(board, m));

    let mut legal_moves_count = 0;
    let mut best_score = -INFINITY;
    
    // PV locale per i figli
    let mut child_pv = PvLine::new();

    for mv in moves {
        if board.esegui_mossa(&mv, None) {
            legal_moves_count += 1;
            
            let score = -search(board, depth - 1, -beta, -alpha, ply + 1, info, &mut child_pv);
            
            board.annulla_mossa(&mv, None, None);

            if info.stop { return 0; }

            if score > best_score {
                best_score = score;
            }

            if score > alpha {
                alpha = score;
                
                // --- AGGIORNAMENTO PV ---
                // Se questa mossa è la migliore trovata finora (ha alzato Alpha),
                // costruiamo la nuova linea PV: [Questa Mossa] + [PV del figlio]
                pv.moves[0] = mv;
                // Copia le mosse del figlio
                for i in 0..child_pv.len {
                    pv.moves[i + 1] = child_pv.moves[i];
                }
                pv.len = child_pv.len + 1;
                // ------------------------

                if score >= beta {
                    return beta; 
                }
            }
        }
    }

    if legal_moves_count == 0 {
        if in_check {
            return -MATE_VALUE + ply as i32; // Preferiamo matti più vicini alla radice
        } else {
            return 0;
        }
    }

    alpha
}

pub fn quiescence(board: &mut Scacchiera, mut alpha: i32, beta: i32, info: &mut SearchInfo) -> i32 {
    info.nodes += 1;
    if info.nodes & 2047 == 0 {
        if info.start_time.elapsed().as_millis() > info.time_limit_ms {
            info.stop = true;
        }
    }
    if info.stop { return 0; }

    let stand_pat = evaluation::evaluate_classical(board);
    if stand_pat >= beta { return beta; }
    if stand_pat > alpha { alpha = stand_pat; }

    let mut moves = board.genera_mosse();
    let mut captures = Vec::new();
    let enemy_bb = board.colori[board.turno.opposto().indice()];

    for m in moves {
        let to = m.a();
        let flag = m.move_flag();
        let is_capture_on_board = (enemy_bb & (1u64 << to)) != 0;
        if is_capture_on_board || flag == MoveFlag::Capture || flag == MoveFlag::EnPassant || m.is_promotion() { 
             captures.push(m);
        }
    }

    captures.sort_by_cached_key(|m| -score_move(board, m));

    for mv in captures {
        if board.esegui_mossa(&mv, None) {
            let score = -quiescence(board, -beta, -alpha, info);
            board.annulla_mossa(&mv, None, None);
            if info.stop { return 0; }
            if score >= beta { return beta; }
            if score > alpha { alpha = score; }
        }
    }
    alpha
}

pub fn score_move(board: &Scacchiera, mv: &Mossa) -> i32 {
    let to = mv.a();
    let from = mv.da();
    let move_flag = mv.move_flag();
    let attacker = board.get_piece_at(from).unwrap_or(Pezzo::Pedone);
    let victim = if move_flag == MoveFlag::EnPassant { Some(Pezzo::Pedone) } else { board.get_piece_at(to) };

    let mut score = 0;
    if let Some(v) = victim {
        let victim_val = piece_value(v);
        let attacker_val = piece_value(attacker);
        score = 10000 + (victim_val * 10) - attacker_val;
    }
    if mv.is_promotion() { score += 8000; }
    if move_flag == MoveFlag::Castle { score += 1000; }
    score
}

fn piece_value(p: Pezzo) -> i32 {
    match p { Pezzo::Pedone => 1, Pezzo::Cavallo => 3, Pezzo::Alfiere => 3, Pezzo::Torre => 5, Pezzo::Regina => 9, Pezzo::Re => 100 }
}

pub fn punteggio_mossa(board: &Scacchiera, mv: &Mossa) -> i32 { score_move(board, mv) }