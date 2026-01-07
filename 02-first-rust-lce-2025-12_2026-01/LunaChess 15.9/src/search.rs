use crate::board::{Scacchiera, Mossa, MoveFlag, Pezzo};
use crate::transposition::{TranspositionTable, Flag}; 
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

const INF: i32 = 50000;
const MATE_SCORE: i32 = 49000;

pub struct SearchInfo {
    pub nodes: u64,
    pub start_time: Instant,
    pub time_limit_ms: u128,
    pub stop_signal: Arc<AtomicBool>,
}

pub struct Motore {
    pub tt: Arc<Mutex<TranspositionTable>>,
    stop_flag: Arc<AtomicBool>,
}

impl Motore {
    pub fn nuovo(tt: Arc<Mutex<TranspositionTable>>) -> Self {
        Motore { tt, stop_flag: Arc::new(AtomicBool::new(false)) }
    }

    pub fn set_stop_flag(&mut self, stop_flag: Arc<AtomicBool>) {
        self.stop_flag = stop_flag;
    }

    pub fn trova_mossa_migliore(&mut self, board: &mut Scacchiera, time_limit_ms: u64, is_main_thread: bool) -> Option<Mossa> {
        // DEBUG: Vediamo se l'hash iniziale è corretto
        if is_main_thread { println!("info string DEBUG: Hash iniziale scacchiera: {:X}", board.hash); }
        
        self.stop_flag.store(false, Ordering::Relaxed);
        let mut moves = board.genera_mosse();
        if moves.is_empty() { return None; }
        
        let mut best_move_found = moves[0];
        let mut info = SearchInfo {
            nodes: 0,
            start_time: Instant::now(),
            time_limit_ms: time_limit_ms as u128,
            stop_signal: self.stop_flag.clone(),
        };

        for depth in 1..=64 {
            let (score, mv) = self.as_search(board, depth, &mut info);
            if info.stop_signal.load(Ordering::Relaxed) { break; }
            if let Some(m) = mv { best_move_found = m; }

            if is_main_thread {
                let time_elapsed = info.start_time.elapsed().as_millis();
                let nps = if time_elapsed > 0 { (info.nodes as u128 * 1000) / time_elapsed } else { 0 };
                let hashfull_val = self.tt.lock().unwrap().hashfull();
                
                println!("info depth {} score cp {} nodes {} time {} nps {} hashfull {} pv {}", 
                    depth, score, info.nodes, time_elapsed, nps, hashfull_val, best_move_found);
            }
            if score.abs() > MATE_SCORE - 100 { break; }
        }
        Some(best_move_found)
    }

    fn as_search(&mut self, board: &mut Scacchiera, depth: u8, info: &mut SearchInfo) -> (i32, Option<Mossa>) {
        let mut alpha = -INF;
        let beta = INF;
        let mut best_move = None;
        let mut moves = board.genera_mosse();
        
        let mut tt_mv = None;
        if let Ok(tt) = self.tt.lock() {
            if let Some(entry) = tt.probe(board.hash) { tt_mv = entry.best_move; }
        }
        
        self.sort_moves(board, &mut moves, tt_mv);

        for mv in moves {
            let cap = board.get_piece_at(mv.a());
            if board.esegui_mossa(&mv, None) {
                if board.re_in_scacco(board.turno.opposto()) {
                    board.annulla_mossa(&mv, cap, None);
                    continue;
                }
                let score = -self.negamax(board, depth - 1, -beta, -alpha, info);
                board.annulla_mossa(&mv, cap, None);

                if info.stop_signal.load(Ordering::Relaxed) { break; }
                if score > alpha {
                    alpha = score;
                    best_move = Some(mv);
                }
            }
        }
        (alpha, best_move)
    }

    fn negamax(&mut self, board: &mut Scacchiera, depth: u8, mut alpha: i32, beta: i32, info: &mut SearchInfo) -> i32 {
        if info.nodes & 2047 == 0 && info.start_time.elapsed().as_millis() > info.time_limit_ms {
            info.stop_signal.store(true, Ordering::Relaxed);
        }
        if info.stop_signal.load(Ordering::Relaxed) { return 0; }
        info.nodes += 1;

        let mut tt_mossa = None;
        if let Ok(tt) = self.tt.lock() {
            if let Some(entry) = tt.probe(board.hash) {
                if entry.depth >= depth {
                    match entry.flag {
                        Flag::Exact => return entry.score,
                        Flag::Alpha if entry.score <= alpha => return alpha,
                        Flag::Beta if entry.score >= beta => return beta,
                        _ => {}
                    }
                }
                tt_mossa = entry.best_move;
            }
        }

        if depth == 0 { return self.quiescence(board, alpha, beta, info); }

        let mut moves = board.genera_mosse();
        self.sort_moves(board, &mut moves, tt_mossa);
        
        let mut best_score = -INF;
        let mut best_mv = None;
        let old_alpha = alpha;
        let mut legal_moves = 0;

        for mv in moves {
            let cap = board.get_piece_at(mv.a());
            if board.esegui_mossa(&mv, None) {
                if board.re_in_scacco(board.turno.opposto()) {
                    board.annulla_mossa(&mv, cap, None);
                    continue;
                }
                legal_moves += 1;
                let score = -self.negamax(board, depth - 1, -beta, -alpha, info);
                board.annulla_mossa(&mv, cap, None);

                if score > best_score {
                    best_score = score;
                    best_mv = Some(mv);
                }
                if score > alpha { alpha = score; }
                if alpha >= beta { break; }
            }
        }

        if legal_moves == 0 {
            return if board.re_in_scacco(board.turno) { -MATE_SCORE + (64 - depth) as i32 } else { 0 };
        }

        let flag = if best_score <= old_alpha { Flag::Alpha } 
                   else if best_score >= beta { Flag::Beta } 
                   else { Flag::Exact };
        
        if let Ok(mut tt) = self.tt.lock() {
            tt.store(board.hash, best_score, depth, flag, best_mv);
        }
        best_score
    }

    fn quiescence(&mut self, board: &mut Scacchiera, mut alpha: i32, beta: i32, info: &mut SearchInfo) -> i32 {
        let stand_pat = crate::nnue::evaluate_nnue(board, &crate::nnue::Network { feature_weights: vec![], feature_bias: vec![] });
        if stand_pat >= beta { return beta; }
        if alpha < stand_pat { alpha = stand_pat; }

        let mut moves = board.genera_mosse();
        self.sort_moves(board, &mut moves, None);
        for mv in moves {
            let cap = board.get_piece_at(mv.a());
            if cap.is_none() && mv.move_flag() != MoveFlag::Promotion { continue; }
            if board.esegui_mossa(&mv, None) {
                if board.re_in_scacco(board.turno.opposto()) {
                    board.annulla_mossa(&mv, cap, None);
                    continue;
                }
                let score = -self.quiescence(board, -beta, -alpha, info);
                board.annulla_mossa(&mv, cap, None);
                if score >= beta { return beta; }
                if score > alpha { alpha = score; }
            }
        }
        alpha
    }

    fn sort_moves(&self, board: &Scacchiera, moves: &mut Vec<Mossa>, tt_mossa: Option<Mossa>) {
        moves.sort_by_cached_key(|mv| {
            let mut score = 0;
            if let Some(m) = tt_mossa { if *mv == m { score += 1000000; } }
            if board.get_piece_at(mv.a()).is_some() { score += 1000; }
            if mv.move_flag() == MoveFlag::Promotion { score += 800; }
            -score
        });
    }
}