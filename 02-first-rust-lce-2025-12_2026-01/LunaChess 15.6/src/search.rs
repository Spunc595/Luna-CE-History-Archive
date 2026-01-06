use crate::board::{Scacchiera, Mossa, MoveFlag, Pezzo};
use crate::nnue::Network; 
use crate::transposition::{TranspositionTable, Flag}; 
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

const INF: i32 = 50000;
const MATE_SCORE: i32 = 49000;
const PIECE_VALUES: [i32; 6] = [100, 320, 330, 500, 900, 20000];

pub struct SearchInfo {
    pub nodes: u64,
    pub start_time: Instant,
    pub time_limit_ms: u128,
    pub stop_signal: Arc<AtomicBool>,
}

pub struct Motore {
    tt: Arc<Mutex<TranspositionTable>>,
    stop_flag: Arc<AtomicBool>,
    net: Option<Network>,
}

impl Motore {
    pub fn nuovo(tt: Arc<Mutex<TranspositionTable>>) -> Self {
        Motore {
            tt,
            stop_flag: Arc::new(AtomicBool::new(false)),
            net: None,
        }
    }

    // Metodi necessari per uci.rs
    pub fn set_stop_flag(&mut self, stop_flag: Arc<AtomicBool>) {
        self.stop_flag = stop_flag;
    }

    pub fn set_nnue_config(&mut self, file_path: String, _use_nnue: bool) {
        self.net = Some(Network::carica(&file_path));
    }

    pub fn trova_mossa_migliore(&mut self, board: &mut Scacchiera, time_limit_ms: u64, is_main_thread: bool) -> Option<Mossa> {
        self.stop_flag.store(false, Ordering::Relaxed);
        let mut moves = board.genera_mosse();
        if moves.is_empty() { return None; }
        
        self.sort_moves_mvv_lva(board, &mut moves, None);
        let mut best_move_found = Some(moves[0]); 

        let mut info = SearchInfo {
            nodes: 0,
            start_time: Instant::now(),
            time_limit_ms: time_limit_ms as u128,
            stop_signal: self.stop_flag.clone(),
        };

        for depth in 1..=64 {
            let mut search_board = board.clone();
            let (score, mv) = self.root_search(&mut search_board, depth, &mut info);
            if let Some(m) = mv { best_move_found = Some(m); }
            if info.stop_signal.load(Ordering::Relaxed) { break; }

            if is_main_thread {
                let time_elapsed = info.start_time.elapsed().as_millis();
                let nps = if time_elapsed > 0 { (info.nodes as u128 * 1000) / time_elapsed } else { 0 };
                println!("info depth {} score cp {} nodes {} time {} nps {} pv {}", 
                    depth, score, info.nodes, time_elapsed, nps, best_move_found.unwrap());
            }
            if score.abs() > MATE_SCORE - 100 { break; }
        }
        best_move_found
    }

    fn negamax(&mut self, board: &mut Scacchiera, depth: u8, mut alpha: i32, beta: i32, info: &mut SearchInfo, _allow_nmp: bool) -> i32 {
        if info.nodes & 2047 == 0 && info.start_time.elapsed().as_millis() > info.time_limit_ms {
            info.stop_signal.store(true, Ordering::Relaxed);
        }
        if info.stop_signal.load(Ordering::Relaxed) { return 0; }
        info.nodes += 1;

        if depth == 0 { return self.quiescence(board, alpha, beta, info); }

        let mut moves = board.genera_mosse();
        self.sort_moves_mvv_lva(board, &mut moves, None);
        
        let mut best_score = -INF;
        let mut moves_searched = 0;

        for mv in moves {
            let capture = board.get_piece_at(mv.da());
            if board.esegui_mossa(&mv, self.net.as_ref()) {
                if board.re_in_scacco(board.turno.opposto()) {
                    board.annulla_mossa(&mv, capture, self.net.as_ref());
                    continue;
                }
                moves_searched += 1;

                let mut score;
                if moves_searched > 4 && depth >= 3 && board.get_piece_at(mv.a()).is_none() {
                    score = -self.negamax(board, depth - 2, -alpha - 1, -alpha, info, true);
                    if score > alpha {
                        score = -self.negamax(board, depth - 1, -beta, -alpha, info, true);
                    }
                } else {
                    score = -self.negamax(board, depth - 1, -beta, -alpha, info, true);
                }

                board.annulla_mossa(&mv, capture, self.net.as_ref());
                if score > best_score { best_score = score; }
                if score > alpha { alpha = score; }
                if alpha >= beta { break; }
            }
        }
        if moves_searched == 0 { return if board.re_in_scacco(board.turno) { -MATE_SCORE } else { 0 }; }
        best_score
    }

    fn root_search(&mut self, board: &mut Scacchiera, depth: u8, info: &mut SearchInfo) -> (i32, Option<Mossa>) {
        let mut alpha = -INF;
        let mut best_move = None;
        let mut moves = board.genera_mosse();
        self.sort_moves_mvv_lva(board, &mut moves, None);

        for mv in moves {
            let capture = board.get_piece_at(mv.da());
            if board.esegui_mossa(&mv, self.net.as_ref()) {
                if board.re_in_scacco(board.turno.opposto()) {
                    board.annulla_mossa(&mv, capture, self.net.as_ref());
                    continue;
                }
                let score = -self.negamax(board, depth - 1, -INF, -alpha, info, true);
                board.annulla_mossa(&mv, capture, self.net.as_ref());
                if score > alpha { alpha = score; best_move = Some(mv); }
            }
        }
        (alpha, best_move)
    }

    fn quiescence(&mut self, board: &mut Scacchiera, mut alpha: i32, beta: i32, info: &mut SearchInfo) -> i32 {
        let stand_pat = crate::nnue::evaluate_nnue(board, self.net.as_ref().unwrap());
        if stand_pat >= beta { return beta; }
        if alpha < stand_pat { alpha = stand_pat; }

        let mut moves = board.genera_mosse();
        self.sort_moves_mvv_lva(board, &mut moves, None);
        for mv in moves {
            if board.get_piece_at(mv.a()).is_none() { continue; }
            let capture = board.get_piece_at(mv.da());
            if board.esegui_mossa(&mv, self.net.as_ref()) {
                let score = -self.quiescence(board, -beta, -alpha, info);
                board.annulla_mossa(&mv, capture, self.net.as_ref());
                if score >= beta { return beta; }
                if score > alpha { alpha = score; }
            }
        }
        alpha
    }

    fn sort_moves_mvv_lva(&self, board: &Scacchiera, moves: &mut Vec<Mossa>, _tt: Option<Mossa>) {
        moves.sort_by_cached_key(|mv| {
            let mut score = 0;
            if let Some(vic) = board.get_piece_at(mv.a()) {
                let att = board.get_piece_at(mv.da()).unwrap_or(Pezzo::Pedone);
                score = 10 * PIECE_VALUES[vic.indice()] - PIECE_VALUES[att.indice()];
            }
            let p = board.get_piece_at(mv.da()).unwrap_or(Pezzo::Pedone);
            if p != Pezzo::Pedone { score += 50; }
            -score
        });
    }
}