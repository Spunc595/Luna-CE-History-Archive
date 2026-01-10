use std::time::Instant;
use crate::board::{Scacchiera, Mossa, Colore, Pezzo};
use crate::tt::{TranspositionTable, Bound};
use crate::zobrist::ZobristKeys;

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
                let elapsed = info.start_time.elapsed().as_millis().max(1);
                let nps = (info.nodes as u128 * 1000) / elapsed;
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

    let hash = board.get_hash(z);

    // NULL MOVE PRUNING
    if depth >= 3 && !in_s && ply > 0 {
        let r = 2;
        let mut null_board = board.clone();
        null_board.make_null_move();
        let score = -search(&mut null_board, depth - 1 - r, -beta, -beta + 1, ply + 1, info, tt, z, km, ht);
        if score >= beta { return beta; }
    }

    let mut tt_move = None;
    if let Some(entry) = tt.probe(hash) {
        tt_move = Some(Mossa { data: entry.move_data });
        if entry.depth as i32 >= depth {
            let s = tt.get_score(&entry, ply);
            match entry.bound {
                1 => return s,
                2 => alpha = alpha.max(s),
                3 => beta = beta.min(s),
                _ => {}
            }
            if alpha >= beta { return s; }
        }
    }

    let mut moves = board.genera_mosse();
    moves.sort_by_key(|m| -punteggio_mossa(m, board, ply, km, ht, tt_move));
    
    let mut legal = 0;
    for m in moves {
        let mut tmp = board.clone();
        if !tmp.esegui_mossa(&m, None) { continue; }
        legal += 1;
        if depth <= 3 && legal > 8 + (depth * depth) as usize && !m.move_flag().is_capture() && !in_s { continue; }
        let score = -search(&mut tmp, depth - 1, -beta, -alpha, ply + 1, info, tt, z, km, ht);
        if score >= beta {
            tt.store(hash, score, m, depth, Bound::Lower, ply);
            if !m.move_flag().is_capture() { km.update(m, ply); ht.update(m, board.turno.opposto(), depth); }
            return beta; 
        }
        if score > alpha { 
            alpha = score;
            tt.store(hash, score, m, depth, Bound::Exact, ply);
        }
    }
    if legal == 0 { return if in_s { -MATE_VALUE + ply as i32 } else { 0 }; }
    
    tt.store(hash, alpha, Mossa { data: 0 }, depth, Bound::Upper, ply);
    alpha
}

fn quiescence(board: &mut Scacchiera, mut alpha: i32, beta: i32, info: &mut SearchInfo) -> i32 {
    let stand_pat = crate::evaluation::evaluate(board);
    if stand_pat >= beta { return beta; }
    if stand_pat > alpha { alpha = stand_pat; }
    let mut moves = board.genera_mosse();
    moves.retain(|m| m.move_flag().is_capture());
    moves.sort_by_key(|m| -punteggio_mossa(m, board, 0, &KillerMoves::new(), &HistoryTable::new(), None));

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

fn punteggio_mossa(m: &Mossa, _b: &Scacchiera, ply: usize, km: &KillerMoves, ht: &HistoryTable, tt: Option<Mossa>) -> i32 {
    if let Some(tm) = tt { if m.data == tm.data { return 30000; } }
    if m.move_flag().is_capture() { return 20000; }
    if ply < 64 && km.primary[ply].data == m.data { return 9000; }
    ht.scores[_b.turno.indice()][m.da()][m.a()]
}