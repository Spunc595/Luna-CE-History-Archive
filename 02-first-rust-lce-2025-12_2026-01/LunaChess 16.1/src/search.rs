// src/search.rs
use crate::board::{Scacchiera, Mossa}; // Rimosso Pezzo e Colore se non usati per togliere i warning
use crate::evaluation;

pub const INFINITY: i32 = 1000000;

pub fn search(board: &mut Scacchiera, depth: i32, mut alpha: i32, beta: i32) -> i32 {
    if depth == 0 {
        return evaluation::evaluate_classical(board);
    }

    let moves = board.genera_mosse();
    
    if moves.is_empty() {
        if board.re_in_scacco(board.turno) {
            return -INFINITY + 100;
        }
        return 0;
    }

    for mv in moves {
        // Forza il tipo usize per risolvere E0282
        let to_sq = mv.a() as usize; 

        if board.esegui_mossa(&mv, None) {
            let score = -search(board, depth - 1, -beta, -alpha);
            board.annulla_mossa(&mv, None, None);

            if score >= beta {
                return beta;
            }
            if score > alpha {
                alpha = score;
            }
        }
    }
    alpha
}

pub fn quiescence(board: &mut Scacchiera, mut alpha: i32, beta: i32) -> i32 {
    let standby_score = evaluation::evaluate_classical(board);
    if standby_score >= beta { return beta; }
    if standby_score > alpha { alpha = standby_score; }

    let moves = board.genera_mosse();
    for mv in moves {
        let to_sq = mv.a() as usize;
        if board.get_piece_at(to_sq).is_some() || mv.flag() == 5 {
            if board.esegui_mossa(&mv, None) {
                let score = -quiescence(board, -beta, -alpha);
                board.annulla_mossa(&mv, None, None);
                if score >= beta { return beta; }
                if score > alpha { alpha = score; }
            }
        }
    }
    alpha
}