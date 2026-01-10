use std::time::Instant;
use crate::board::{Scacchiera, Mossa, Colore, Pezzo};
use crate::tt::{TranspositionTable, Bound};
use crate::zobrist::ZobristKeys;
use crate::evaluation::SEE_VAL;

pub const MATE_VALUE: i32 = 49000;
const ASPIRATION_WINDOW: i32 = 40;

pub struct SearchInfo {
    pub start_time: Instant,
    pub time_limit_ms: u128,
    pub nodes: u64,
    pub stop: bool,
}

#[derive(Clone, Copy)]
pub struct PVLine { pub moves: [Mossa; 64], pub len: usize }

pub struct KillerMoves { pub primary: [Mossa; 64], pub secondary: [Mossa; 64] }
impl KillerMoves {
    pub fn new() -> Self { Self { primary: [Mossa { data: 0 }; 64], secondary: [Mossa { data: 0 }; 64] } }
    pub fn update(&mut self, m: Mossa, ply: usize) {
        if ply < 64 && m != self.primary[ply] { self.secondary[ply] = self.primary[ply]; self.primary[ply] = m; }
    }
}

pub struct HistoryTable { pub scores: [[[i32; 64]; 64]; 2] }
impl HistoryTable {
    pub fn new() -> Self { Self { scores: [[[0; 64]; 64]; 2] } }
    pub fn update(&mut self, m: Mossa, c: Colore, d: i32) { self.scores[c.indice()][m.da()][m.a()] += d * d; }
}

pub fn iterative_deepening(board: &mut Scacchiera, info: &mut SearchInfo, tt: &mut TranspositionTable, z: &ZobristKeys) -> Mossa {
    let mut best_m = Mossa { data: 0 };
    let (mut killers, mut history) = (KillerMoves::new(), HistoryTable::new());
    let mut last_s = 0;

    for depth in 1..65 {
        if info.stop { break; }
        let (mut alpha, mut beta) = (-100000, 100000);
        if depth > 5 { alpha = last_s - ASPIRATION_WINDOW; beta = last_s + ASPIRATION_WINDOW; }

        loop {
            let (score, m, pv) = search_root(board, depth, alpha, beta, info, tt, z, &mut killers, &mut history);
            if info.stop { break; }
            if score <= alpha { alpha = -100000; }
            else if score >= beta { beta = 100000; }
            else {
                last_s = score; best_m = m;
                let nps = (info.nodes as u128 * 1000) / info.start_time.elapsed().as_millis().max(1);
                print!("info depth {} score cp {} nodes {} nps {} pv", depth, score, info.nodes, nps);
                for i in 0..pv.len { if pv.moves[i].data != 0 { print!(" {}", pv.moves[i]); } }
                println!();
                break;
            }
        }
    }
    best_m
}

fn search_root(board: &mut Scacchiera, depth: i32, mut alpha: i32, beta: i32, info: &mut SearchInfo, tt: &mut TranspositionTable, z: &ZobristKeys, km: &mut KillerMoves, ht: &mut HistoryTable) -> (i32, Mossa, PVLine) {
    let mut pv = PVLine { moves: [Mossa { data: 0 }; 64], len: 0 };
    let mut moves = board.genera_mosse();
    moves.sort_by_key(|m| -punteggio_mossa(m, board, 0, km, ht, None));
    
    let mut best_m = moves[0];
    for (i, m) in moves.iter().enumerate() {
        if info.nodes % 1024 == 0 { println!("info currmove {} currmovenumber {}", m, i + 1); }
        let mut tmp = board.clone();
        if !tmp.esegui_mossa(m, None) { continue; }
        let score = -search(&mut tmp, depth - 1, -beta, -alpha, 1, info, tt, z, km, ht);
        if info.stop { break; }
        if score > alpha { alpha = score; best_m = *m; pv.moves[0] = *m; pv.len = 1; }
    }
    (alpha, best_m, pv)
}

fn search(board: &mut Scacchiera, mut depth: i32, mut alpha: i32, mut beta: i32, ply: usize, info: &mut SearchInfo, tt: &mut TranspositionTable, z: &ZobristKeys, km: &mut KillerMoves, ht: &mut HistoryTable) -> i32 {
    info.nodes += 1;
    if info.nodes % 2048 == 0 && info.start_time.elapsed().as_millis() >= info.time_limit_ms { info.stop = true; }
    if info.stop { return 0; }
    
    let in_s = board.re_in_scacco(board.turno);
    if in_s { depth += 1; }
    if depth <= 0 { return quiescence(board, alpha, beta, info); }

    let mut moves = board.genera_mosse();
    moves.sort_by_key(|m| -punteggio_mossa(m, board, ply, km, ht, None));
    
    let mut legal = 0;
    for m in moves {
        let mut tmp = board.clone();
        if !tmp.esegui_mossa(&m, None) { continue; }
        legal += 1;
        if depth <= 3 && legal > 10 && !m.move_flag().is_capture() { continue; }
        let score = -search(&mut tmp, depth - 1, -beta, -alpha, ply + 1, info, tt, z, km, ht);
        if score >= beta { return beta; }
        if score > alpha { alpha = score; }
    }
    if legal == 0 { return if in_s { -MATE_VALUE + ply as i32 } else { 0 }; }
    alpha
}

fn quiescence(board: &mut Scacchiera, mut alpha: i32, beta: i32, info: &mut SearchInfo) -> i32 {
    let stand_pat = crate::evaluation::evaluate(board);
    if stand_pat >= beta { return beta; }
    if stand_pat > alpha { alpha = stand_pat; }
    let mut moves = board.genera_mosse();
    moves.retain(|m: &Mossa| m.move_flag().is_capture());
    for m in moves {
        let mut tmp = board.clone();
        if !tmp.esegui_mossa(&m, None) { continue; }
        info.nodes += 1;
        let score = -quiescence(&mut tmp, -beta, -alpha, info);
        if score >= beta { return beta; }
        if score > alpha { alpha = score; }
    }
    alpha
}

fn punteggio_mossa(m: &Mossa, b: &Scacchiera, _p: usize, _km: &KillerMoves, _ht: &HistoryTable, _tt: Option<Mossa>) -> i32 {
    if m.move_flag().is_capture() { 10000 } else { 0 }
}