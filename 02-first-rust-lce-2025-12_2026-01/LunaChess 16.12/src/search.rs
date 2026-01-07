use crate::board::{Scacchiera, Mossa, Pezzo, MoveFlag};
use crate::evaluation;
use std::time::Instant;

pub const INFINITY: i32 = 50000;
pub const MATE_VALUE: i32 = 49000;
pub const MAX_PLY: usize = 64;

// Strutture per l'ordinamento avanzato
pub struct SearchData {
    pub killers: [[Option<Mossa>; 2]; MAX_PLY],
    pub history: [[i32; 64]; 64], // [from][to]
}

impl SearchData {
    pub fn new() -> Self {
        Self {
            killers: [[None; 2]; MAX_PLY],
            history: [[0; 64]; 64],
        }
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
    pub fn new() -> Self {
        Self { moves: [Mossa { data: 0 }; MAX_PLY], len: 0 }
    }
}

pub fn search_root(board: &mut Scacchiera, depth: i32, info: &mut SearchInfo) -> (i32, Mossa) {
    let mut root_pv = PvLine::new();
    // Creiamo i dati per Killers e History che durano per tutta la ricerca
    let mut search_data = SearchData::new();
    
    let score = search(board, depth, -INFINITY, INFINITY, 0, info, &mut root_pv, &mut search_data);
    
    let best_move = if root_pv.len > 0 {
        root_pv.moves[0]
    } else {
        board.genera_mosse().get(0).cloned().unwrap_or(Mossa { data: 0 })
    };

    (score, best_move)
}

pub fn search(board: &mut Scacchiera, mut depth: i32, mut alpha: i32, beta: i32, ply: usize, info: &mut SearchInfo, pv: &mut PvLine, data: &mut SearchData) -> i32 {
    if info.nodes & 2047 == 0 {
        if info.start_time.elapsed().as_millis() > info.time_limit_ms { info.stop = true; }
    }
    if info.stop { return 0; }
    info.nodes += 1;

    pv.len = 0;
    if ply >= MAX_PLY { return evaluation::evaluate_classical(board); }

    let in_check = board.re_in_scacco(board.turno);
    if in_check { depth += 1; }

    if depth <= 0 {
        return quiescence(board, alpha, beta, info);
    }

    // NULL MOVE PRUNING
    if depth >= 3 && !in_check && beta != INFINITY {
        let static_eval = evaluation::evaluate_classical(board);
        if static_eval >= beta {
            board.make_null_move();
            let mut null_pv = PvLine::new();
            // Passiamo data anche qui
            let score = -search(board, depth - 3, -beta, -beta + 1, ply + 1, info, &mut null_pv, data);
            board.unmake_null_move();
            if info.stop { return 0; }
            if score >= beta { return beta; }
        }
    }

    let mut moves = board.genera_mosse();
    
    // --- ORDINAMENTO MOSSE AVANZATO ---
    // Passiamo 'ply' per accedere ai killer moves di questo livello
    moves.sort_by_cached_key(|m| -score_move(board, m, ply, data));

    let mut legal_moves_count = 0;
    let mut best_score = -INFINITY;
    let mut child_pv = PvLine::new();
    let mut best_move_this_node: Option<Mossa> = None;

    for (i, mv) in moves.iter().enumerate() {
        if board.esegui_mossa(mv, None) {
            legal_moves_count += 1;
            let mut score;

            if i == 0 {
                score = -search(board, depth - 1, -beta, -alpha, ply + 1, info, &mut child_pv, data);
            } else {
                let reduction = if depth >= 3 && !in_check && !mv.move_flag().is_capture() && i > 3 { 1 } else { 0 };
                score = -search(board, depth - 1 - reduction, -alpha - 1, -alpha, ply + 1, info, &mut child_pv, data);
                if score > alpha && score < beta {
                    score = -search(board, depth - 1, -beta, -alpha, ply + 1, info, &mut child_pv, data);
                }
            }
            
            board.annulla_mossa(mv, None, None);

            if info.stop { return 0; }

            if score > best_score {
                best_score = score;
            }

            if score > alpha {
                alpha = score;
                best_move_this_node = Some(*mv);
                
                pv.moves[0] = *mv;
                for j in 0..child_pv.len { pv.moves[j + 1] = child_pv.moves[j]; }
                pv.len = child_pv.len + 1;

                if score >= beta {
                    // --- AGGIORNAMENTO KILLER E HISTORY (BETA CUTOFF) ---
                    if !mv.move_flag().is_capture() {
                        // 1. Aggiungi ai Killer Moves
                        data.killers[ply][1] = data.killers[ply][0];
                        data.killers[ply][0] = Some(*mv);

                        // 2. Aggiungi alla History Table (Bonus = Depth^2)
                        let bonus = depth * depth;
                        data.history[mv.da()][mv.a()] += bonus;
                        // Limitiamo il valore per evitare overflow
                        if data.history[mv.da()][mv.a()] > 20000 {
                            // Scaliamo tutti i valori se diventano troppo alti
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

    // Qui usiamo un sorting semplificato senza history/killers
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

// Funzione di score per la ricerca principale (con History e Killers)
pub fn score_move(board: &Scacchiera, mv: &Mossa, ply: usize, data: &SearchData) -> i32 {
    let to = mv.a();
    let from = mv.da();
    let move_flag = mv.move_flag();
    
    // 1. CATTURE (MVV-LVA) - Priorità Massima (10000+)
    let attacker = board.get_piece_at(from).unwrap_or(Pezzo::Pedone);
    let victim = if move_flag == MoveFlag::EnPassant { Some(Pezzo::Pedone) } else { board.get_piece_at(to) };
    if let Some(v) = victim {
        return 20000 + (piece_value(v) * 10) - piece_value(attacker);
    }
    if mv.is_promotion() { return 19000; }

    // 2. KILLER MOVES - Priorità Media (9000)
    // Se questa mossa è stata una "Killer" in questo ply, provala subito dopo le catture
    if let Some(k1) = data.killers[ply][0] {
        if k1.data == mv.data { return 9000; }
    }
    if let Some(k2) = data.killers[ply][1] {
        if k2.data == mv.data { return 8000; }
    }

    // 3. HISTORY HEURISTIC - Priorità Dinamica (0 - 5000)
    // Se è una mossa tranquilla, usa il punteggio storico
    return data.history[from][to];
}

// Funzione semplificata per Quiescence (solo MVV-LVA)
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

// Funzione legacy per compatibilità se usata altrove
pub fn punteggio_mossa(board: &Scacchiera, mv: &Mossa) -> i32 { score_capture(board, mv) }