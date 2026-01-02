use crate::board::{Scacchiera, Mossa, Pezzo};
use crate::move_ordering::sort_moves;
use crate::nnue::Network; 
use crate::transposition::TranspositionTable;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
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
    tt: Arc<TranspositionTable>,
    stop_flag: Arc<AtomicBool>,
    net: Option<Network>,
}

impl Motore {
    pub fn nuovo(tt: Arc<TranspositionTable>) -> Self {
        Motore {
            tt,
            stop_flag: Arc::new(AtomicBool::new(false)),
            net: None,
        }
    }

    pub fn set_stop_flag(&mut self, stop_flag: Arc<AtomicBool>) {
        self.stop_flag = stop_flag;
    }

    pub fn set_nnue_config(&mut self, file_path: String, use_nnue: bool) {
        if !use_nnue {
            self.net = None;
            return;
        }
        self.net = Some(Network::carica(&file_path));
    }

    pub fn trova_mossa_migliore(
        &mut self, 
        board: &mut Scacchiera, 
        time_limit_ms: u64, 
        is_main_thread: bool
    ) -> Option<Mossa> {
        
        let mut info = SearchInfo {
            nodes: 0,
            start_time: Instant::now(),
            time_limit_ms: time_limit_ms as u128,
            stop_signal: self.stop_flag.clone(),
        };

        let mut best_move = None;

        for depth in 1..=64 {
            let mut search_board = board.clone();
            let (score, mv) = self.root_search(&mut search_board, depth, &mut info);
            
            if info.stop_signal.load(Ordering::Relaxed) { break; }

            if let Some(m) = mv { best_move = Some(m); }

            if is_main_thread {
                let time_elapsed = info.start_time.elapsed().as_millis();
                let nps = if time_elapsed > 0 { (info.nodes as u128 * 1000) / time_elapsed } else { 0 };
                let mv_str = best_move.map_or("none".to_string(), |m| m.to_string());
                
                println!("info depth {} score cp {} nodes {} time {} nps {} pv {}", 
                    depth, score, info.nodes, time_elapsed, nps, mv_str);
            }
        }
        best_move
    }

    fn root_search(&mut self, board: &mut Scacchiera, depth: u8, info: &mut SearchInfo) -> (i32, Option<Mossa>) {
        let mut alpha = -INF;
        let mut best_move = None;
        let mut best_score = -INF;

        let mut moves = board.genera_mosse(); 
        sort_moves(board, &mut moves);

        let mut legal_moves_count = 0;

        for mv in moves {
            let capture = board.get_piece_at(mv.da()); 
            if board.esegui_mossa(&mv, self.net.as_ref()) {
                if board.re_in_scacco(board.turno.opposto()) {
                    board.annulla_mossa(&mv, capture, self.net.as_ref());
                    continue;
                }

                legal_moves_count += 1;
                // Chiamata al negamax standard
                let score = -self.negamax(board, depth - 1, -INF, -alpha, info, true);
                board.annulla_mossa(&mv, capture, self.net.as_ref());

                if info.stop_signal.load(Ordering::Relaxed) { return (best_score, best_move); }

                if score > best_score {
                    best_score = score;
                    best_move = Some(mv);
                }
                if score > alpha { alpha = score; }
            }
        }
        if legal_moves_count == 0 { return (self.evaluate_endgame(board), None); }
        (best_score, best_move)
    }

    fn negamax(&mut self, board: &mut Scacchiera, depth: u8, mut alpha: i32, beta: i32, info: &mut SearchInfo, allow_nmp: bool) -> i32 {
        if info.nodes & 2047 == 0 {
            if info.start_time.elapsed().as_millis() > info.time_limit_ms {
                info.stop_signal.store(true, Ordering::Relaxed);
            }
        }
        info.nodes += 1;

        if info.stop_signal.load(Ordering::Relaxed) { return 0; }

        // --- 1. BASE CASE E QUIESCENCE ---
        if depth == 0 {
            return self.quiescence(board, alpha, beta, info);
        }

        // --- 2. NULL MOVE PRUNING (NMP) ---
        // Se non siamo sotto scacco e la profondità è sufficiente, proviamo a passare il turno
        if allow_nmp && depth >= 3 && !board.re_in_scacco(board.turno) {
            let R = 2; // Riduzione di profondità tipica per NMP
            let mut board_copy = board.clone();
            board_copy.turno = board_copy.turno.opposto(); // "Passa" il turno
            
            // Facciamo una ricerca ridotta per vedere se l'avversario riesce a punirci
            let score = -self.negamax(&mut board_copy, depth - 1 - R, -beta, -beta + 1, info, false);
            
            if score >= beta {
                return beta; // Cutoff: la nostra posizione è così forte che possiamo saltare il turno
            }
        }

        let mut moves = board.genera_mosse();
        sort_moves(board, &mut moves);
        let mut best_score = -INF;
        let mut legal_moves_count = 0;

        for mv in moves {
            let capture = board.get_piece_at(mv.da());
            if board.esegui_mossa(&mv, self.net.as_ref()) {
                if board.re_in_scacco(board.turno.opposto()) {
                    board.annulla_mossa(&mv, capture, self.net.as_ref());
                    continue;
                }
                legal_moves_count += 1;
                let score = -self.negamax(board, depth - 1, -beta, -alpha, info, true);
                board.annulla_mossa(&mv, capture, self.net.as_ref());

                if info.stop_signal.load(Ordering::Relaxed) { return 0; }
                if score > best_score { best_score = score; }
                if score > alpha { alpha = score; }
                if alpha >= beta { break; }
            }
        }

        if legal_moves_count == 0 { return self.evaluate_endgame(board); }
        best_score
    }

    // --- 3. QUIESCENCE SEARCH ---
    fn quiescence(&mut self, board: &mut Scacchiera, mut alpha: i32, beta: i32, info: &mut SearchInfo) -> i32 {
        let stand_pat = if let Some(net) = &self.net {
            crate::evaluation::evaluate(board, net)
        } else {
            crate::evaluation::evaluate_classical(board)
        };

        if stand_pat >= beta { return beta; }
        if alpha < stand_pat { alpha = stand_pat; }

        let mut moves = board.genera_mosse();
        sort_moves(board, &mut moves);

        for mv in moves {
            // Nella Quiescence guardiamo SOLO le catture
            let is_capture = board.get_piece_at(mv.a()).is_some() || mv.move_flag() == crate::board::MoveFlag::EnPassant;
            if !is_capture { continue; }

            let capture = board.get_piece_at(mv.da());
            if board.esegui_mossa(&mv, self.net.as_ref()) {
                if board.re_in_scacco(board.turno.opposto()) {
                    board.annulla_mossa(&mv, capture, self.net.as_ref());
                    continue;
                }
                
                let score = -self.quiescence(board, -beta, -alpha, info);
                board.annulla_mossa(&mv, capture, self.net.as_ref());

                if score >= beta { return beta; }
                if score > alpha { alpha = score; }
            }
        }
        alpha
    }

    fn evaluate_endgame(&self, board: &Scacchiera) -> i32 {
        if board.re_in_scacco(board.turno) { -MATE_SCORE } else { 0 }
    }
}