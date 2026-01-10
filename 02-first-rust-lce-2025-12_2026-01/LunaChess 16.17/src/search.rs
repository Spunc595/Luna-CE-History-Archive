use crate::board::{Scacchiera, Mossa, Pezzo, MoveFlag};
use crate::evaluation;
use crate::tt::{self, TranspositionTable, Bound};
use crate::zobrist::{self, ZobristKeys};
use std::time::Instant;

pub const INFINITY: i32 = 50000;
pub const MATE_VALUE: i32 = 49000;
pub const MAX_PLY: usize = 64;

pub struct SearchData {
    pub killers: [[Option<Mossa>; 2]; MAX_PLY],
    pub history: [[i32; 64]; 64],
}

impl SearchData {
    pub fn new() -> Self {
        Self { killers: [[None; 2]; MAX_PLY], history: [[0; 64]; 64] }
    }
}

pub struct SearchInfo {
    pub start_time: Instant,
    pub time_limit_ms: u128,
    pub nodes: u64,
    pub stop: bool,
}

#[derive(Clone, Copy)]
pub struct PvLine {
    pub moves: [Mossa; MAX_PLY],
    pub len: usize,
}
impl PvLine {
    pub fn new() -> Self { Self { moves: [Mossa { data: 0 }; MAX_PLY], len: 0 } }
}

// --- Entry Point ---
pub fn search_root(
    board: &mut Scacchiera, 
    depth: i32, 
    info: &mut SearchInfo, 
    tt: &mut TranspositionTable,
    zobrist: &ZobristKeys
) -> (i32, Mossa, PvLine) {
    let mut root_pv = PvLine::new();
    let mut search_data = SearchData::new();
    
    // NOTA: new_search() incrementa l'età della TT. 
    // Se usi Iterative Deepening in main/uci, chiamalo solo all'inizio della mossa, non ad ogni depth.
    // Qui lo lasciamo per sicurezza se chiami search_root una volta sola.
    // tt.new_search(); 

    let score = search(board, depth, -INFINITY, INFINITY, 0, info, &mut root_pv, &mut search_data, tt, zobrist);
    
    // --- FIX CRITICO PER ILLEGAL MOVE ---
    // Se root_pv è vuota (accade se falliamo low, o time stop immediato), dobbiamo trovare UNA mossa legale.
    // Altrimenti il motore restituisce Mossa{data:0} -> a1b1 -> ILLEGAL -> Crash.
    let best_move = if root_pv.len > 0 {
        root_pv.moves[0]
    } else {
        // Fallback: Genera tutte le mosse e prendi la prima che è legale
        let moves = board.genera_mosse();
        let mut fallback_move = Mossa { data: 0 };
        let mut found_legal = false;
        
        // Ordiniamo per score per prendere una mossa decente anche in fallback
        let mut moves_scored: Vec<(Mossa, i32)> = moves.iter().map(|m| (*m, score_move(board, m, 0, &search_data, None))).collect();
        moves_scored.sort_by_key(|k| -k.1);

        for (m, _) in moves_scored {
            let mut tmp_board = board.clone();
            if tmp_board.esegui_mossa(&m, None) {
                fallback_move = m;
                found_legal = true;
                break;
            }
        }
        
        // Se non troviamo mosse legali, siamo in Scacco Matto o Stallo.
        // Restituiamo una mossa nulla, ma il punteggio 'score' dovrebbe già rifletterlo.
        if !found_legal {
             Mossa { data: 0 } 
        } else {
             fallback_move
        }
    };

    (score, best_move, root_pv)
}

pub fn search(
    board: &mut Scacchiera, 
    mut depth: i32, 
    mut alpha: i32, 
    mut beta: i32, 
    ply: usize, 
    info: &mut SearchInfo, 
    pv: &mut PvLine, 
    data: &mut SearchData,
    tt: &mut TranspositionTable,
    zobrist: &ZobristKeys
) -> i32 {

    // Controllo tempo ogni 2048 nodi
    if info.nodes & 2047 == 0 {
        if info.start_time.elapsed().as_millis() > info.time_limit_ms { info.stop = true; }
    }
    if info.stop { return 0; } // Ritorno neutro in caso di stop
    info.nodes += 1;

    pv.len = 0; 
    if ply >= MAX_PLY { return evaluation::evaluate_classical(board); }

    let hash_key = zobrist::compute_hash(board, zobrist);
    let mut tt_move: Option<Mossa> = None;

    // --- PROBE TRANSPOSITION TABLE ---
    if let Some(entry) = tt.probe(hash_key) {
        if entry.move_data != 0 {
            tt_move = Some(Mossa { data: entry.move_data });
        }
        if entry.depth as i32 >= depth {
            let mut score_v = entry.score as i32;
            // Aggiustamento mate score dalla TT
            if score_v > 40000 { score_v -= ply as i32; }
            else if score_v < -40000 { score_v += ply as i32; }

            match entry.bound {
                1 => return score_v, // Exact
                2 => { if score_v >= beta { return beta; } }, // Lower Bound
                3 => { if score_v <= alpha { return alpha; } }, // Upper Bound
                _ => {}
            }
        }
    }

    let in_check = board.re_in_scacco(board.turno);
    // Check Extension: se siamo sotto scacco, estendiamo la ricerca per non perdere tatticismi
    if in_check { depth += 1; }

    if depth <= 0 {
        return quiescence(board, alpha, beta, info);
    }

    // --- NULL MOVE PRUNING ---
    // Non facciamo Null Move se siamo sotto scacco o in endgame (rischio zugzwang)
    if depth >= 3 && !in_check && beta != INFINITY {
        let static_eval = evaluation::evaluate_classical(board);
        if static_eval >= beta {
            let mut nm_board = board.clone();
            nm_board.make_null_move(); 
            let mut null_pv = PvLine::new();
            
            // R = 3 (riduzione standard)
            let score = -search(&mut nm_board, depth - 3, -beta, -beta + 1, ply + 1, info, &mut null_pv, data, tt, zobrist);
            
            if info.stop { return 0; }
            if score >= beta { return beta; }
        }
    }

    let mut moves = board.genera_mosse();
    moves.sort_by_cached_key(|m| -score_move(board, m, ply, data, tt_move));

    let mut legal_moves_count = 0;
    let mut best_score = -INFINITY;
    let mut best_move_this_node: Option<Mossa> = None;
    let mut bound_type = Bound::Upper; 

    let mut child_pv = PvLine::new();

    for (i, mv) in moves.iter().enumerate() {
        let mut board_clone = board.clone();
        
        if board_clone.esegui_mossa(mv, None) {
            legal_moves_count += 1;
            let mut score;

            // Principal Variation Search (PVS)
            if i == 0 {
                score = -search(&mut board_clone, depth - 1, -beta, -alpha, ply + 1, info, &mut child_pv, data, tt, zobrist);
            } else {
                // Late Move Reduction (LMR)
                let reduction = if depth >= 3 && !in_check && !mv.move_flag().is_capture() && i > 3 { 1 } else { 0 };
                
                score = -search(&mut board_clone, depth - 1 - reduction, -alpha - 1, -alpha, ply + 1, info, &mut child_pv, data, tt, zobrist);
                
                // Re-search se la riduzione ha fallito o se la finestra null (PVS) ha fallito
                if score > alpha && score < beta {
                    score = -search(&mut board_clone, depth - 1, -beta, -alpha, ply + 1, info, &mut child_pv, data, tt, zobrist);
                }
            }
            
            if info.stop { return 0; }

            if score > best_score {
                best_score = score;
                best_move_this_node = Some(*mv);
            }

            if score > alpha {
                alpha = score;
                bound_type = Bound::Exact;
                
                // Aggiorna PV
                pv.moves[0] = *mv;
                for j in 0..child_pv.len {
                    pv.moves[j + 1] = child_pv.moves[j];
                }
                pv.len = child_pv.len + 1;

                if score >= beta {
                    // Store Cutoff (Lower Bound)
                    // Importante: Non salvare in TT se il tempo è scaduto!
                    if !info.stop {
                        tt.store(hash_key, beta, Some(*mv), depth, Bound::Lower, ply);
                    }
                    
                    // History Heuristic & Killer Moves
                    if !mv.move_flag().is_capture() {
                        data.killers[ply][1] = data.killers[ply][0];
                        data.killers[ply][0] = Some(*mv);
                        data.history[mv.da()][mv.a()] += depth * depth;
                    }
                    return beta; 
                }
            }
        }
    }

    // Checkmate / Stalemate detection
    if legal_moves_count == 0 {
        if in_check { 
            return -MATE_VALUE + ply as i32; 
        } else { 
            return 0; // Stallo
        }
    }

    // Store Exact or Upper Bound
    if !info.stop {
        tt.store(hash_key, best_score, best_move_this_node, depth, bound_type, ply);
    }
    
    alpha
}

pub fn quiescence(board: &mut Scacchiera, mut alpha: i32, beta: i32, info: &mut SearchInfo) -> i32 {
    if info.nodes & 2047 == 0 {
        if info.start_time.elapsed().as_millis() > info.time_limit_ms { info.stop = true; }
    }
    if info.stop { return 0; }
    info.nodes += 1;

    let stand_pat = evaluation::evaluate_classical(board);
    if stand_pat >= beta { return beta; }
    if stand_pat > alpha { alpha = stand_pat; }

    let mut moves = board.genera_mosse();
    let mut captures = Vec::new();
    let enemy_bb = board.colori[board.turno.opposto().indice()];

    // Genera solo catture e promozioni
    for m in moves {
        let to = m.a();
        let flag = m.move_flag();
        // Check cattura (tramite bitboard nemico o flag) o promozione
        if (enemy_bb & (1u64 << to)) != 0 || flag == MoveFlag::Capture || flag == MoveFlag::EnPassant || m.is_promotion() { 
             captures.push(m);
        }
    }

    captures.sort_by_cached_key(|m| -score_capture(board, m));

    for mv in captures {
        let mut board_clone = board.clone(); 
        if board_clone.esegui_mossa(&mv, None) {
            let score = -quiescence(&mut board_clone, -beta, -alpha, info);
            
            if info.stop { return 0; }
            
            if score >= beta { return beta; }
            if score > alpha { alpha = score; }
        }
    }
    alpha
}

pub fn score_move(board: &Scacchiera, mv: &Mossa, ply: usize, data: &SearchData, tt_move: Option<Mossa>) -> i32 {
    if let Some(tm) = tt_move {
        if tm.data == mv.data { return 30000; }
    }

    let to = mv.a();
    let from = mv.da();
    let move_flag = mv.move_flag();
    
    let attacker = board.get_piece_at(from).unwrap_or(Pezzo::Pedone);
    // Stima vittima (approssimata se en passant)
    let victim = if move_flag == MoveFlag::EnPassant { 
        Some(Pezzo::Pedone) 
    } else { 
        board.get_piece_at(to) 
    };
    
    // MVV-LVA (Most Valuable Victim - Least Valuable Attacker)
    if let Some(v) = victim {
        return 20000 + (piece_value(v) * 10) - piece_value(attacker);
    }
    
    if mv.is_promotion() { return 19000; }

    // Killer Moves
    if let Some(k1) = data.killers[ply][0] { if k1.data == mv.data { return 9000; } }
    if let Some(k2) = data.killers[ply][1] { if k2.data == mv.data { return 8000; } }

    // History Heuristic
    return data.history[from][to];
}

pub fn score_capture(board: &Scacchiera, mv: &Mossa) -> i32 {
    let to = mv.a();
    let from = mv.da();
    let move_flag = mv.move_flag();
    let attacker = board.get_piece_at(from).unwrap_or(Pezzo::Pedone);
    let victim = if move_flag == MoveFlag::EnPassant { Some(Pezzo::Pedone) } else { board.get_piece_at(to) };
    
    if let Some(v) = victim {
        return 10000 + (piece_value(v) * 10) - piece_value(attacker);
    }
    if mv.is_promotion() { return 8000; }
    0
}

fn piece_value(p: Pezzo) -> i32 {
    match p { Pezzo::Pedone=>1, Pezzo::Cavallo=>3, Pezzo::Alfiere=>3, Pezzo::Torre=>5, Pezzo::Regina=>9, Pezzo::Re=>100 }
}