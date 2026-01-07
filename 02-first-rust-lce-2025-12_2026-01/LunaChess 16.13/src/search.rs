use crate::board::{Scacchiera, Mossa, Pezzo, MoveFlag};
use crate::evaluation;
use crate::tt::{self, TranspositionTable, Bound}; // IMPORTA TT
use crate::zobrist::{self, ZobristKeys}; // IMPORTA Zobrist
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

// Entry Point
pub fn search_root(
    board: &mut Scacchiera, 
    depth: i32, 
    info: &mut SearchInfo, 
    tt: &mut TranspositionTable, // NUOVO PARAMS
    zobrist: &ZobristKeys        // NUOVO PARAMS
) -> (i32, Mossa) {
    let mut root_pv = PvLine::new();
    let mut search_data = SearchData::new();
    
    // Incrementa età della TT per nuova ricerca
    tt.new_search();

    let score = search(board, depth, -INFINITY, INFINITY, 0, info, &mut root_pv, &mut search_data, tt, zobrist);
    
    let best_move = if root_pv.len > 0 {
        root_pv.moves[0]
    } else {
        board.genera_mosse().get(0).cloned().unwrap_or(Mossa { data: 0 })
    };

    (score, best_move)
}

// SEARCH con TT
pub fn search(
    board: &mut Scacchiera, 
    mut depth: i32, 
    mut alpha: i32, 
    mut beta: i32, 
    ply: usize, 
    info: &mut SearchInfo, 
    pv: &mut PvLine, 
    data: &mut SearchData,
    tt: &mut TranspositionTable, // Passiamo TT
    zobrist: &ZobristKeys        // Passiamo Keys
) -> i32 {

    if info.nodes & 2047 == 0 {
        if info.start_time.elapsed().as_millis() > info.time_limit_ms { info.stop = true; }
    }
    if info.stop { return 0; }
    info.nodes += 1;

    pv.len = 0;
    if ply >= MAX_PLY { return evaluation::evaluate_classical(board); }

    // --- 1. CALCOLO HASH & TT PROBE ---
    // In un motore avanzato, l'hash si aggiorna in make_move. Qui lo calcoliamo per semplicità.
    let hash_key = zobrist::compute_hash(board, zobrist);
    
    // Mossa suggerita dalla TT (Hash Move) utile per l'ordinamento
    let mut tt_move: Option<Mossa> = None;

    if let Some(entry) = tt.probe(hash_key) {
        // Estrai mossa
        if entry.move_data != 0 {
            tt_move = Some(Mossa { data: entry.move_data });
        }

        // Possiamo usare il punteggio della TT per tagliare?
        // Solo se la profondità salvata è >= alla profondità che ci serve ora
        if entry.depth as i32 >= depth {
            
            // Recupera score corretto per il ply
            let mut score_v = entry.score as i32;
            if score_v > 40000 { score_v -= ply as i32; }
            else if score_v < -40000 { score_v += ply as i32; }

            match entry.bound {
                1 => return score_v, // Exact
                2 => { // Lower bound (Alpha)
                    if score_v >= beta { return beta; }
                },
                3 => { // Upper bound (Beta)
                    if score_v <= alpha { return alpha; }
                },
                _ => {}
            }
        }
    }
    // ----------------------------------

    let in_check = board.re_in_scacco(board.turno);
    if in_check { depth += 1; }

    if depth <= 0 {
        return quiescence(board, alpha, beta, info);
    }

    // NULL MOVE PRUNING (Invariato)
    if depth >= 3 && !in_check && beta != INFINITY {
        let static_eval = evaluation::evaluate_classical(board);
        if static_eval >= beta {
            board.make_null_move();
            let mut null_pv = PvLine::new();
            let score = -search(board, depth - 3, -beta, -beta + 1, ply + 1, info, &mut null_pv, data, tt, zobrist);
            board.unmake_null_move();
            if info.stop { return 0; }
            if score >= beta { return beta; }
        }
    }

    let mut moves = board.genera_mosse();
    
    // --- ORDINAMENTO CON HASH MOVE ---
    // Passiamo tt_move allo score_move
    moves.sort_by_cached_key(|m| -score_move(board, m, ply, data, tt_move));

    let mut legal_moves_count = 0;
    let mut best_score = -INFINITY;
    let mut best_move_this_node: Option<Mossa> = None;
    
    // Tipo di bound di default: Upper (non abbiamo trovato nulla meglio di alpha)
    let mut bound_type = Bound::Upper; 

    let mut child_pv = PvLine::new();

    for (i, mv) in moves.iter().enumerate() {
        if board.esegui_mossa(mv, None) {
            legal_moves_count += 1;
            let mut score;

            if i == 0 {
                score = -search(board, depth - 1, -beta, -alpha, ply + 1, info, &mut child_pv, data, tt, zobrist);
            } else {
                let reduction = if depth >= 3 && !in_check && !mv.move_flag().is_capture() && i > 3 { 1 } else { 0 };
                score = -search(board, depth - 1 - reduction, -alpha - 1, -alpha, ply + 1, info, &mut child_pv, data, tt, zobrist);
                if score > alpha && score < beta {
                    score = -search(board, depth - 1, -beta, -alpha, ply + 1, info, &mut child_pv, data, tt, zobrist);
                }
            }
            
            board.annulla_mossa(mv, None, None);

            if info.stop { return 0; }

            if score > best_score {
                best_score = score;
                best_move_this_node = Some(*mv);
            }

            if score > alpha {
                alpha = score;
                bound_type = Bound::Exact; // Abbiamo migliorato alpha, quindi è Exact (PV)
                
                pv.moves[0] = *mv;
                for j in 0..child_pv.len { pv.moves[j + 1] = child_pv.moves[j]; }
                pv.len = child_pv.len + 1;

                if score >= beta {
                    // Fail High -> Store Lower Bound (Beta Cutoff)
                    tt.store(hash_key, beta, Some(*mv), depth, Bound::Lower, ply);

                    if !mv.move_flag().is_capture() {
                        data.killers[ply][1] = data.killers[ply][0];
                        data.killers[ply][0] = Some(*mv);
                        let bonus = depth * depth;
                        data.history[mv.da()][mv.a()] += bonus;
                        if data.history[mv.da()][mv.a()] > 20000 {
                            for r in 0..64 { for c in 0..64 { data.history[r][c] /= 2; } }
                        }
                    }
                    return beta; 
                }
            }
        }
    }

    if legal_moves_count == 0 {
        if in_check { return -MATE_VALUE + ply as i32; } else { return 0; }
    }

    // Store TT result alla fine della ricerca del nodo
    tt.store(hash_key, best_score, best_move_this_node, depth, bound_type, ply);

    alpha
}

pub fn quiescence(board: &mut Scacchiera, mut alpha: i32, beta: i32, info: &mut SearchInfo) -> i32 {
    info.nodes += 1;
    if info.nodes & 2047 == 0 {
        if info.start_time.elapsed().as_millis() > info.time_limit_ms { info.stop = true; }
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
        if (enemy_bb & (1u64 << to)) != 0 || flag == MoveFlag::Capture || flag == MoveFlag::EnPassant || m.is_promotion() { 
             captures.push(m);
        }
    }

    captures.sort_by_cached_key(|m| -score_capture(board, m));

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

impl MoveFlag {
    pub fn is_capture(&self) -> bool {
        match self { MoveFlag::Capture | MoveFlag::EnPassant => true, _ => false }
    }
}

// Aggiornato score_move per accettare tt_move (Hash Move)
pub fn score_move(board: &Scacchiera, mv: &Mossa, ply: usize, data: &SearchData, tt_move: Option<Mossa>) -> i32 {
    // 0. HASH MOVE (Massima priorità assoluta)
    if let Some(tm) = tt_move {
        if tm.data == mv.data { return 30000; }
    }

    let to = mv.a();
    let from = mv.da();
    let move_flag = mv.move_flag();
    
    // 1. CATTURE
    let attacker = board.get_piece_at(from).unwrap_or(Pezzo::Pedone);
    let victim = if move_flag == MoveFlag::EnPassant { Some(Pezzo::Pedone) } else { board.get_piece_at(to) };
    if let Some(v) = victim {
        return 20000 + (piece_value(v) * 10) - piece_value(attacker);
    }
    if mv.is_promotion() { return 19000; }

    // 2. KILLERS
    if let Some(k1) = data.killers[ply][0] { if k1.data == mv.data { return 9000; } }
    if let Some(k2) = data.killers[ply][1] { if k2.data == mv.data { return 8000; } }

    // 3. HISTORY
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