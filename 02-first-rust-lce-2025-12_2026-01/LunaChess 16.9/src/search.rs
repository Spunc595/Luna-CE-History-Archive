use crate::board::{Scacchiera, Mossa, Pezzo, MoveFlag};
use crate::evaluation;
use std::time::Instant;

pub const INFINITY: i32 = 50000;
pub const MATE_VALUE: i32 = 49000;

pub struct SearchInfo {
    pub start_time: Instant,
    pub time_limit_ms: u128,
    pub nodes: u64,
    pub stop: bool,
}

pub fn search(board: &mut Scacchiera, mut depth: i32, mut alpha: i32, beta: i32, info: &mut SearchInfo) -> i32 {
    // Controllo tempo frequente
    if info.nodes & 2047 == 0 {
        if info.start_time.elapsed().as_millis() > info.time_limit_ms {
            info.stop = true;
        }
    }
    if info.stop { return 0; }
    info.nodes += 1;

    // Estensione Scacco
    let in_check = board.re_in_scacco(board.turno);
    if in_check { depth += 1; }

    if depth <= 0 {
        return quiescence(board, alpha, beta, info);
    }

    let mut moves = board.genera_mosse();
    
    // Ordinamento Veloce (MVV-LVA)
    moves.sort_by_cached_key(|m| -score_move(board, m));

    let mut legal_moves_count = 0;
    let mut best_score = -INFINITY;

    for mv in moves {
        if board.esegui_mossa(&mv, None) {
            legal_moves_count += 1;
            
            let score = -search(board, depth - 1, -beta, -alpha, info);
            
            board.annulla_mossa(&mv, None, None);

            if info.stop { return 0; }

            if score > best_score {
                best_score = score;
            }

            if score > alpha {
                alpha = score;
                if score >= beta {
                    return beta; 
                }
            }
        }
    }

    if legal_moves_count == 0 {
        if in_check {
            return -MATE_VALUE + (info.nodes as i32 % 100);
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
    
    for m in moves {
        let flag = m.move_flag();
        if flag == MoveFlag::Capture || flag == MoveFlag::EnPassant || m.is_promotion() { 
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

// MVV-LVA Veloce (Senza controlli lenti di legalità o attacco)
pub fn score_move(board: &Scacchiera, mv: &Mossa) -> i32 {
    let to = mv.a();
    let from = mv.da();
    let move_flag = mv.move_flag();
    
    let attacker = board.get_piece_at(from).unwrap_or(Pezzo::Pedone);
    let victim = if move_flag == MoveFlag::EnPassant {
        Some(Pezzo::Pedone)
    } else {
        board.get_piece_at(to)
    };

    let mut score = 0;

    if let Some(v) = victim {
        let victim_val = piece_value(v);
        let attacker_val = piece_value(attacker);
        // Priorità alle catture vantaggiose (PxQ)
        score = 10000 + (victim_val * 10) - attacker_val;
    }

    if mv.is_promotion() { score += 8000; }
    if move_flag == MoveFlag::Castle { score += 1000; }
    
    score
}

fn piece_value(p: Pezzo) -> i32 {
    match p {
        Pezzo::Pedone => 1, Pezzo::Cavallo => 3, Pezzo::Alfiere => 3,
        Pezzo::Torre => 5, Pezzo::Regina => 9, Pezzo::Re => 100,
    }
}

pub fn punteggio_mossa(board: &Scacchiera, mv: &Mossa) -> i32 {
    score_move(board, mv)
}