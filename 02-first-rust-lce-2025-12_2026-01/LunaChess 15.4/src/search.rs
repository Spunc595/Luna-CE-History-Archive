use crate::board::{Scacchiera, Mossa, MoveFlag, Pezzo};
use crate::nnue::Network; 
use crate::transposition::TranspositionTable;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

const INF: i32 = 50000;
const MATE_SCORE: i32 = 49000;

// Valori dei pezzi per MVV-LVA (Pedone..Re)
const PIECE_VALUES: [i32; 6] = [100, 300, 300, 500, 900, 20000];

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
        // Carica la rete (che ora è una dummy piena di zeri per compatibilità)
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

        // Iterative Deepening
        for depth in 1..=64 {
            let mut search_board = board.clone();
            let (score, mv) = self.root_search(&mut search_board, depth, &mut info);
            
            if info.stop_signal.load(Ordering::Relaxed) { break; }

            if let Some(m) = mv { best_move = Some(m); }

            if is_main_thread {
                let time_elapsed = info.start_time.elapsed().as_millis();
                let nps = if time_elapsed > 0 { (info.nodes as u128 * 1000) / time_elapsed } else { 0 };
                let mv_str = best_move.map_or("none".to_string(), |m| m.to_string());
                
                // Formattazione UCI per info
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
        
        // ORDIAMENTO MOSSE (MVV-LVA)
        self.sort_moves_mvv_lva(board, &mut moves);

        let mut legal_moves_count = 0;

        for mv in moves {
            let capture = board.get_piece_at(mv.da()); 
            if board.esegui_mossa(&mv, self.net.as_ref()) {
                if board.re_in_scacco(board.turno.opposto()) {
                    board.annulla_mossa(&mv, capture, self.net.as_ref());
                    continue;
                }

                legal_moves_count += 1;
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
        // Controllo tempo ogni 2048 nodi
        if info.nodes & 2047 == 0 {
            if info.start_time.elapsed().as_millis() > info.time_limit_ms {
                info.stop_signal.store(true, Ordering::Relaxed);
            }
        }
        info.nodes += 1;

        if info.stop_signal.load(Ordering::Relaxed) { return 0; }

        if depth == 0 {
            return self.quiescence(board, alpha, beta, info);
        }

        // --- NULL MOVE PRUNING ---
        if allow_nmp && depth >= 3 && !board.re_in_scacco(board.turno) {
            // USIAMO LA NUOVA VALUTAZIONE HCE IN NNUE.RS
            let static_eval = if let Some(net) = &self.net {
                crate::nnue::evaluate_nnue(board, net)
            } else {
                0 // Fallback
            };

            if static_eval >= beta {
                let r = 2; // Riduzione cauta
                let mut board_copy = board.clone();
                board_copy.turno = board_copy.turno.opposto(); 
                
                // Passiamo una dummy net o None per annulla_mossa se servisse, qui cloniamo quindi ok
                // Nota: evaluate_nnue gestisce il turno, ma qui stiamo saltando la mossa
                // In una implementazione completa NMP bisogna fare make_null_move.
                // Per ora usiamo una logica semplificata di riduzione.
                
                let score = -self.negamax(&mut board_copy, depth.saturating_sub(1 + r), -beta, -beta + 1, info, false);
                
                if score >= beta {
                    return beta; 
                }
            }
        }

        let mut moves = board.genera_mosse();
        self.sort_moves_mvv_lva(board, &mut moves);
        
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
                if alpha >= beta { break; } // Beta Cutoff
            }
        }

        if legal_moves_count == 0 { return self.evaluate_endgame(board); }
        best_score
    }

    fn quiescence(&mut self, board: &mut Scacchiera, mut alpha: i32, beta: i32, info: &mut SearchInfo) -> i32 {
        info.nodes += 1;
        
        // Stand-pat (Valutazione statica)
        let stand_pat = if let Some(net) = &self.net {
            crate::nnue::evaluate_nnue(board, net)
        } else {
            0
        };

        if stand_pat >= beta { return beta; }
        if alpha < stand_pat { alpha = stand_pat; }

        let mut moves = board.genera_mosse();
        self.sort_moves_mvv_lva(board, &mut moves);

        for mv in moves {
            // Solo catture o promozioni in QSearch
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

    // --- FUNZIONE DI ORDINAMENTO MVV-LVA ---
    fn sort_moves_mvv_lva(&self, board: &Scacchiera, moves: &mut Vec<Mossa>) {
        // Calcola uno score per ogni mossa
        // Usiamo un vettore temporaneo di tuple (mossa, score)
        let mut scored_moves: Vec<(Mossa, i32)> = Vec::with_capacity(moves.len());

        for mv in moves.iter() {
            let mut score = 0;
            let move_flag = mv.move_flag();

            // 1. Catture
            if let Some(victim) = board.get_piece_at(mv.a()) {
                // MVV-LVA: 10 * ValoreVittima - ValoreAggressore
                // In questo modo PxQ vale tantissimo, QxP vale poco.
                let attacker = board.get_piece_at(mv.da()).unwrap_or(Pezzo::Pedone);
                score = 10 * PIECE_VALUES[victim.indice()] - PIECE_VALUES[attacker.indice()] + 10000;
            } 
            // Gestione En Passant (la casella 'a' è vuota, ma è una cattura di pedone)
            else if move_flag == MoveFlag::EnPassant {
                score = 10 * 100 - 100 + 10000; // PxP
            }

            // 2. Promozioni (Alta priorità)
            if move_flag == MoveFlag::Promotion {
                score += 15000; // Più alto di quasi tutte le catture
            }

            scored_moves.push((*mv, score));
        }

        // Ordina decrescente (punteggio più alto prima)
        scored_moves.sort_by(|a, b| b.1.cmp(&a.1));

        // Rimetti le mosse nel vettore originale
        *moves = scored_moves.into_iter().map(|(m, _)| m).collect();
    }
}