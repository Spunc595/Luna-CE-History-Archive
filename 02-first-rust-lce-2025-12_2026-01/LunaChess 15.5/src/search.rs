use crate::board::{Scacchiera, Mossa, MoveFlag, Pezzo};
use crate::nnue::Network; 
use crate::transposition::{TranspositionTable, Flag}; 
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

const INF: i32 = 50000;
const MATE_SCORE: i32 = 49000;
const PIECE_VALUES: [i32; 6] = [100, 300, 300, 500, 900, 20000];

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
        
        // 1. Reset Stop Flag (FONDAMENTALE)
        self.stop_flag.store(false, Ordering::Relaxed);

        // 2. Fallback Move immediata
        let mut moves = board.genera_mosse();
        if moves.is_empty() { return None; }
        
        // Ordinamento preliminare per avere una mossa decente come fallback
        self.sort_moves_mvv_lva(board, &mut moves, None);
        let mut best_move = Some(moves[0]); 

        let mut info = SearchInfo {
            nodes: 0,
            start_time: Instant::now(),
            time_limit_ms: time_limit_ms as u128,
            stop_signal: self.stop_flag.clone(),
        };

        for depth in 1..=64 {
            let mut search_board = board.clone();
            
            // root_search ora ritorna sempre qualcosa di sensato se possibile
            let (score, mv) = self.root_search(&mut search_board, depth, &mut info);
            
            // Aggiorniamo best_move solo se abbiamo un risultato valido
            if let Some(m) = mv {
                best_move = Some(m);
            }

            // Se il tempo è scaduto, usciamo, ma teniamo l'ultimo best_move valido (quello del depth precedente o parziale)
            if info.stop_signal.load(Ordering::Relaxed) { break; }

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
        let mut beta = INF;
        let mut best_move = None;
        let mut best_score = -INF;

        let mut moves = board.genera_mosse(); 
        self.sort_moves_mvv_lva(board, &mut moves, None);

        let mut legal_moves_count = 0;

        for mv in moves {
            let capture = board.get_piece_at(mv.da()); 
            if board.esegui_mossa(&mv, self.net.as_ref()) {
                if board.re_in_scacco(board.turno.opposto()) {
                    board.annulla_mossa(&mv, capture, self.net.as_ref());
                    continue;
                }

                legal_moves_count += 1;
                
                // Se è la prima mossa, la salviamo come candidata anche se il tempo scade subito dopo
                if best_move.is_none() {
                    best_move = Some(mv);
                }

                let score = -self.negamax(board, depth - 1, -beta, -alpha, info, true);
                board.annulla_mossa(&mv, capture, self.net.as_ref());

                // Se stop, ritorniamo quello che abbiamo
                if info.stop_signal.load(Ordering::Relaxed) { return (best_score, best_move); }

                if score > best_score {
                    best_score = score;
                    best_move = Some(mv);
                }
                if score > alpha { alpha = score; }
            }
        }
        
        if legal_moves_count == 0 { return (self.evaluate_endgame(board), None); }
        
        if let Some(bm) = best_move {
            if let Ok(mut tt) = self.tt.lock() {
                tt.store(board.hash, best_score, depth, Flag::Exact, Some(bm));
            }
        }

        (best_score, best_move)
    }

    fn negamax(&mut self, board: &mut Scacchiera, depth: u8, mut alpha: i32, mut beta: i32, info: &mut SearchInfo, allow_nmp: bool) -> i32 {
        if info.nodes & 2047 == 0 {
            if info.start_time.elapsed().as_millis() > info.time_limit_ms {
                info.stop_signal.store(true, Ordering::Relaxed);
            }
        }
        info.nodes += 1;
        if info.stop_signal.load(Ordering::Relaxed) { return 0; }

        let original_alpha = alpha;
        let mut tt_move = None;

        if let Ok(tt) = self.tt.lock() {
            if let Some(entry) = tt.probe(board.hash) {
                if entry.depth >= depth {
                    match entry.flag {
                        Flag::Exact => return entry.score,
                        Flag::Alpha => { if entry.score <= alpha { return entry.score; } }
                        Flag::Beta => { if entry.score >= beta { return entry.score; } }
                    }
                }
                tt_move = entry.best_move;
            }
        }

        if depth == 0 {
            return self.quiescence(board, alpha, beta, info);
        }

        if allow_nmp && depth >= 3 && !board.re_in_scacco(board.turno) {
            let static_eval = if let Some(net) = &self.net {
                crate::nnue::evaluate_nnue(board, net)
            } else { 0 };

            if static_eval >= beta {
                let r = 2; 
                let mut board_copy = board.clone();
                board_copy.turno = board_copy.turno.opposto(); 
                let score = -self.negamax(&mut board_copy, depth.saturating_sub(1 + r), -beta, -beta + 1, info, false);
                if score >= beta { return beta; }
            }
        }

        let mut moves = board.genera_mosse();
        self.sort_moves_mvv_lva(board, &mut moves, tt_move);
        
        let mut best_score = -INF;
        let mut legal_moves_count = 0;
        let mut best_move = None;

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
                
                if score > best_score {
                    best_score = score;
                    best_move = Some(mv);
                }
                
                if score > alpha { alpha = score; }
                if alpha >= beta { break; }
            }
        }

        if legal_moves_count == 0 { return self.evaluate_endgame(board); }

        let flag = if best_score <= original_alpha {
            Flag::Alpha
        } else if best_score >= beta {
            Flag::Beta
        } else {
            Flag::Exact
        };

        if let Ok(mut tt) = self.tt.lock() {
            tt.store(board.hash, best_score, depth, flag, best_move);
        }

        best_score
    }

    fn quiescence(&mut self, board: &mut Scacchiera, mut alpha: i32, beta: i32, info: &mut SearchInfo) -> i32 {
        info.nodes += 1;
        
        let stand_pat = if let Some(net) = &self.net {
            crate::nnue::evaluate_nnue(board, net)
        } else { 0 };

        if stand_pat >= beta { return beta; }
        
        const DELTA_MARGIN: i32 = 950;
        if stand_pat < alpha - DELTA_MARGIN {
            return alpha; 
        }

        if alpha < stand_pat { alpha = stand_pat; }

        let mut moves = board.genera_mosse();
        self.sort_moves_mvv_lva(board, &mut moves, None);

        for mv in moves {
            let is_capture = board.get_piece_at(mv.a()).is_some() || mv.move_flag() == MoveFlag::EnPassant || mv.move_flag() == MoveFlag::Promotion;
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

    fn sort_moves_mvv_lva(&self, board: &Scacchiera, moves: &mut Vec<Mossa>, tt_move: Option<Mossa>) {
        let mut scored_moves: Vec<(Mossa, i32)> = Vec::with_capacity(moves.len());

        for mv in moves.iter() {
            let mut score = 0;
            if let Some(tm) = tt_move {
                if *mv == tm { score = 200000; }
            }
            if score == 0 {
                let move_flag = mv.move_flag();
                if let Some(victim) = board.get_piece_at(mv.a()) {
                    let attacker = board.get_piece_at(mv.da()).unwrap_or(Pezzo::Pedone);
                    score = 10 * PIECE_VALUES[victim.indice()] - PIECE_VALUES[attacker.indice()] + 10000;
                } else if move_flag == MoveFlag::EnPassant {
                    score = 10 * 100 - 100 + 10000;
                }
                if move_flag == MoveFlag::Promotion {
                    score += 15000;
                }
            }
            scored_moves.push((*mv, score));
        }
        scored_moves.sort_by(|a, b| b.1.cmp(&a.1));
        *moves = scored_moves.into_iter().map(|(m, _)| m).collect();
    }
}