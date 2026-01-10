use std::time::Instant;
use crate::board::{Scacchiera, Mossa, Colore};
use crate::tt::TranspositionTable;
use crate::zobrist::ZobristKeys;

pub const MATE_VALUE: i32 = 49000;

pub struct SearchInfo {
    pub start_time: Instant,
    pub time_limit_ms: u128,
    pub nodes: u64,
    pub stop: bool,
}

#[derive(Clone, Copy)]
pub struct PVLine {
    pub moves: [Mossa; 64],
    pub len: usize,
}

pub struct KillerMoves {
    pub primary: [Mossa; 64],
    pub secondary: [Mossa; 64],
}

impl KillerMoves {
    pub fn new() -> Self {
        Self {
            primary: [Mossa { data: 0 }; 64],
            secondary: [Mossa { data: 0 }; 64],
        }
    }
    pub fn update(&mut self, m: Mossa, ply: usize) {
        if ply < 64 && m != self.primary[ply] {
            self.secondary[ply] = self.primary[ply];
            self.primary[ply] = m;
        }
    }
}

/// Tabella per History Heuristic: [colore][da][a]
pub struct HistoryTable {
    pub scores: [[[i32; 64]; 64]; 2],
}

impl HistoryTable {
    pub fn new() -> Self {
        Self { scores: [[[0; 64]; 64]; 2] }
    }
    pub fn update(&mut self, m: Mossa, colore: Colore, depth: i32) {
        let d = depth as i32;
        self.scores[colore.indice()][m.da()][m.a()] += d * d;
    }
}

pub fn iterative_deepening(board: &mut Scacchiera, info: &mut SearchInfo, tt: &mut TranspositionTable, zobrist: &ZobristKeys) -> Mossa {
    let mut best_move = Mossa { data: 0 };
    let mut killers = KillerMoves::new();
    let mut history = HistoryTable::new();
    
    for depth in 1..65 {
        let (score, m, pv) = search_root(board, depth, info, tt, zobrist, &mut killers, &mut history);
        
        if info.stop && depth > 1 { break; }

        best_move = m;

        let elapsed = info.start_time.elapsed().as_millis().max(1);
        let nps = (info.nodes as u128 * 1000) / elapsed;
        
        print!("info depth {} score cp {} nodes {} nps {} pv", depth, score, info.nodes, nps);
        for i in 0..pv.len {
            print!(" {}", pv.moves[i]);
        }
        println!();

        if score > MATE_VALUE - 100 || score < -MATE_VALUE + 100 { break; }
    }
    best_move
}

fn punteggio_mossa(m: &Mossa, board: &Scacchiera, ply: usize, killers: &KillerMoves, history: &HistoryTable) -> i32 {
    if m.move_flag().is_capture() {
        let victim = board.get_piece_at(m.a()).map_or(0, |p| p.indice() as i32);
        let attacker = board.get_piece_at(m.da()).map_or(0, |p| p.indice() as i32);
        return 10000 + (10 * victim) - attacker;
    }
    if ply < 64 {
        if *m == killers.primary[ply] { return 9000; }
        if *m == killers.secondary[ply] { return 8000; }
    }
    history.scores[board.turno.indice()][m.da()][m.a()]
}

fn search_root(board: &mut Scacchiera, depth: i32, info: &mut SearchInfo, tt: &mut TranspositionTable, zobrist: &ZobristKeys, killers: &mut KillerMoves, history: &mut HistoryTable) -> (i32, Mossa, PVLine) {
    let mut pv = PVLine { moves: [Mossa { data: 0 }; 64], len: 0 };
    let mut moves: Vec<Mossa> = board.genera_mosse().into_iter().filter(|m| {
        let mut tmp = board.clone();
        tmp.esegui_mossa(m, None)
    }).collect();

    if moves.is_empty() { return (0, Mossa { data: 0 }, pv); }

    moves.sort_by_key(|m| -punteggio_mossa(m, board, 0, killers, history));

    let mut best_move = moves[0];
    let mut alpha = -100000;
    let beta = 100000;

    for m in moves {
        let mut temp_board = board.clone();
        temp_board.esegui_mossa(&m, None);
        
        let score = -search(&mut temp_board, depth - 1, -beta, -alpha, 1, info, tt, zobrist, killers, history);

        if info.stop { break; }

        if score > alpha {
            alpha = score;
            best_move = m;
            pv.moves[0] = m;
            pv.len = 1;
        }
    }
    (alpha, best_move, pv)
}

fn search(board: &mut Scacchiera, depth: i32, mut alpha: i32, beta: i32, ply: usize, info: &mut SearchInfo, tt: &mut TranspositionTable, zobrist: &ZobristKeys, killers: &mut KillerMoves, history: &mut HistoryTable) -> i32 {
    info.nodes += 1;
    if info.nodes % 2048 == 0 && info.start_time.elapsed().as_millis() >= info.time_limit_ms {
        info.stop = true;
    }
    if info.stop { return 0; }
    if depth <= 0 { return quiescence(board, alpha, beta, info); }

    // --- NULL MOVE PRUNING ---
    if depth >= 3 && !board.re_in_scacco(board.turno) && ply > 0 {
        let mut temp_board = board.clone();
        temp_board.make_null_move();
        let score = -search(&mut temp_board, depth - 1 - 2, -beta, -beta + 1, ply + 1, info, tt, zobrist, killers, history);
        if score >= beta { return beta; }
    }

    let mut moves = board.genera_mosse();
    moves.sort_by_key(|m| -punteggio_mossa(m, board, ply, killers, history));

    let mut legal_count = 0;
    let mut best_score = -100000;

    for (i, m) in moves.into_iter().enumerate() {
        let mut temp_board = board.clone();
        if !temp_board.esegui_mossa(&m, None) { continue; }
        legal_count += 1;

        let mut score;
        
        // --- LATE MOVE REDUCTIONS (LMR) ---
        if depth >= 3 && legal_count > 3 && !m.move_flag().is_capture() && !board.re_in_scacco(board.turno) {
            score = -search(&mut temp_board, depth - 2, -alpha - 1, -alpha, ply + 1, info, tt, zobrist, killers, history);
            if score > alpha {
                score = -search(&mut temp_board, depth - 1, -beta, -alpha, ply + 1, info, tt, zobrist, killers, history);
            }
        } else {
            score = -search(&mut temp_board, depth - 1, -beta, -alpha, ply + 1, info, tt, zobrist, killers, history);
        }

        if score > best_score {
            best_score = score;
            if score > alpha {
                alpha = score;
                if alpha >= beta {
                    if !m.move_flag().is_capture() {
                        killers.update(m, ply);
                        history.update(m, board.turno.opposto(), depth); // opposto perché il turno è già cambiato
                    }
                    return beta;
                }
            }
        }
    }

    if legal_count == 0 {
        return if board.re_in_scacco(board.turno) { -MATE_VALUE + ply as i32 } else { 0 };
    }
    alpha
}

fn quiescence(board: &mut Scacchiera, mut alpha: i32, beta: i32, info: &mut SearchInfo) -> i32 {
    let stand_pat = crate::evaluation::evaluate(board);
    if stand_pat >= beta { return beta; }
    if stand_pat > alpha { alpha = stand_pat; }

    let mut moves = board.genera_mosse();
    moves.retain(|m| m.move_flag().is_capture());
    
    moves.sort_by_key(|m| {
        let victim = board.get_piece_at(m.a()).map_or(0, |p| p.indice() as i32);
        let attacker = board.get_piece_at(m.da()).map_or(0, |p| p.indice() as i32);
        !(10 * victim - attacker)
    });

    for m in moves {
        let mut temp_board = board.clone();
        if !temp_board.esegui_mossa(&m, None) { continue; }
        info.nodes += 1;
        let score = -quiescence(&mut temp_board, -beta, -alpha, info);
        if score >= beta { return beta; }
        if score > alpha { alpha = score; }
    }
    alpha
}