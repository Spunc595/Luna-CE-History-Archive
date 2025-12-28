use crate::board::{Scacchiera, Mossa, MoveFlag};
use crate::movegen::{self, MoveOrderer};
use crate::evaluation::valuta_posizione;
use crate::transposition::{TranspositionTable, Bound};
use std::time::{Instant, Duration};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

const INFINITO: i16 = 30_000;
const MATE_VALUE: i16 = 29_000;
const MATE_BOUND: i16 = MATE_VALUE - 1000;
const MAX_PLY: usize = 96;

pub struct Motore {
    pub transposition_table: Arc<TranspositionTable>,
    pub mossa_migliore: Option<Mossa>,
    pub nodi_visitati: u64,
    pub seldepth: usize,
    pub killer_moves: [[Option<Mossa>; 2]; MAX_PLY],
    pub history: [[i32; 64]; 64],
    pub pv_table: Vec<Vec<Mossa>>,
    start_time: Instant,
    allocated_time: u64,
    stop_flag: Arc<AtomicBool>,
}

impl Motore {
    pub fn nuovo(tt: Arc<TranspositionTable>) -> Self {
        let mut pv_table = Vec::with_capacity(MAX_PLY);
        for _ in 0..MAX_PLY { pv_table.push(Vec::with_capacity(MAX_PLY)); }
        Motore {
            transposition_table: tt,
            mossa_migliore: None, nodi_visitati: 0, seldepth: 0,
            killer_moves: [[None; 2]; MAX_PLY], history: [[0; 64]; 64],
            pv_table, start_time: Instant::now(), allocated_time: 0,
            stop_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn set_stop_flag(&mut self, flag: Arc<AtomicBool>) { self.stop_flag = flag; }

    pub fn trova_mossa_migliore(&mut self, scacchiera: &mut Scacchiera, tempo_ms: u64, is_main_thread: bool) -> Option<Mossa> {
        self.start_time = Instant::now();
        self.allocated_time = tempo_ms;
        self.nodi_visitati = 0; self.seldepth = 0;
        self.stop_flag.store(false, Ordering::Relaxed);
        let mut best_move = None;
        let mut alpha = -INFINITO; let mut beta = INFINITO;
        for depth in 1..=64 {
            let score = self.negamax(scacchiera, alpha, beta, depth, 0, true, true);
            if self.should_stop() { break; }
            if is_main_thread {
                if !self.pv_table[0].is_empty() { 
                    best_move = Some(self.pv_table[0][0]); 
                    self.mossa_migliore = best_move;
                }
                self.print_info(depth, score, self.start_time.elapsed());
                if score <= alpha || score >= beta { alpha = -INFINITO; beta = INFINITO; }
                else { alpha = score - 30; beta = score + 30; }
            }
            if score.abs() > MATE_BOUND { break; }
        }
        best_move
    }

    fn negamax(&mut self, s: &mut Scacchiera, mut alpha: i16, mut beta: i16, mut depth: u8, ply: usize, is_pv: bool, can_null: bool) -> i16 {
        if self.nodi_visitati & 4095 == 0 && self.should_stop() { return 0; }
        if ply >= MAX_PLY { return valuta_posizione(s) as i16; }
        if depth == 0 { return self.quiescence(s, alpha, beta, ply); }
        self.nodi_visitati += 1;
        let hash = s.hash();
        let mut tt_move = None;
        if let Some(e) = self.transposition_table.probe(hash) {
            tt_move = e.mossa();
            if !is_pv && e.depth() >= depth {
                let sc = self.transposition_table.score_from_tt(e.score(), ply);
                match e.bound() {
                    Bound::Exact => return sc,
                    Bound::Lower => if sc >= beta { return sc; },
                    Bound::Upper => if sc <= alpha { return sc; },
                }
            }
        }
        let in_check = s.re_in_scacco(s.turno());
        let static_eval = if in_check { -INFINITO } else { valuta_posizione(s) as i16 };
        if !is_pv && !in_check {
            if depth <= 7 && static_eval - 75 * (depth as i16) >= beta { return static_eval; }
            if can_null && depth >= 3 && static_eval >= beta && s.ha_pezzi_maggiori(s.turno()) {
                let r = 2 + depth / 6; s.esegui_mossa_nulla();
                let score = -self.negamax(s, -beta, -beta + 1, depth.saturating_sub(1 + r), ply + 1, false, false);
                s.annulla_mossa_nulla(); if score >= beta { return beta; }
            }
        }
        let mut extension = 0;
        if in_check { extension = 1; }
        depth += extension;
        let orderer = MoveOrderer::new(MAX_PLY);
        let mosse = movegen::genera_e_filtra_mosse_ordinate(s, &orderer, tt_move.as_ref(), depth as i32, ply);
        if mosse.is_empty() { return if in_check { -MATE_VALUE + ply as i16 } else { 0 }; }
        let mut best_score = -INFINITO;
        let mut best_m = None;
        let mut moves_searched = 0;
        for m in mosse {
            if !s.esegui_mossa(&m) { continue; }
            moves_searched += 1;
            let mut score;
            if moves_searched == 1 { score = -self.negamax(s, -beta, -alpha, depth - 1, ply + 1, true, true); }
            else {
                let reduction = if depth >= 3 && moves_searched > 4 && !in_check && !m.flag().is_capture() { 1 } else { 0 };
                score = -self.negamax(s, -alpha - 1, -alpha, depth - 1 - reduction, ply + 1, false, true);
                if score > alpha && score < beta { score = -self.negamax(s, -beta, -alpha, depth - 1, ply + 1, true, true); }
            }
            s.annulla_mossa();
            if self.should_stop() { return 0; }
            if score > best_score {
                best_score = score; best_m = Some(m);
                if score > alpha {
                    alpha = score; self.update_pv(ply, m);
                    if alpha >= beta {
                        if !m.flag().is_capture() {
                            self.history[m.da().indice()][m.a().indice()] += (depth * depth) as i32;
                            if self.killer_moves[ply][0] != Some(m) { self.killer_moves[ply][1] = self.killer_moves[ply][0]; self.killer_moves[ply][0] = Some(m); }
                        }
                        break;
                    }
                }
            }
        }
        let bound = if best_score >= beta { Bound::Lower } else if is_pv && best_m.is_some() { Bound::Exact } else { Bound::Upper };
        self.transposition_table.store(hash, self.transposition_table.score_to_tt(best_score, ply), best_m.map(|m| (m.data & 0xFFFF) as u16).unwrap_or(0), depth, bound);
        best_score
    }

    fn quiescence(&mut self, s: &mut Scacchiera, mut alpha: i16, beta: i16, _ply: usize) -> i16 {
        let stand_pat = valuta_posizione(s) as i16;
        if stand_pat >= beta { return beta; }
        if stand_pat > alpha { alpha = stand_pat; }
        let mut mosse = movegen::genera_mosse_legali(s);
        mosse.retain(|m| m.flag().is_capture());
        for m in mosse {
            if !s.esegui_mossa(&m) { continue; }
            let score = -self.quiescence(s, -beta, -alpha, _ply + 1);
            s.annulla_mossa();
            if score >= beta { return beta; }
            if score > alpha { alpha = score; }
        }
        alpha
    }

    fn should_stop(&self) -> bool {
        if self.stop_flag.load(Ordering::Relaxed) { return true; }
        if self.allocated_time > 0 && self.start_time.elapsed().as_millis() as u64 > self.allocated_time {
            self.stop_flag.store(true, Ordering::Relaxed); return true;
        }
        false
    }

    fn update_pv(&mut self, ply: usize, m: Mossa) {
        self.pv_table[ply].clear(); self.pv_table[ply].push(m);
        if ply + 1 < MAX_PLY {
            let next = self.pv_table[ply + 1].clone();
            self.pv_table[ply].extend(next);
        }
    }

    fn print_info(&self, d: u8, s: i16, e: Duration) {
        let nps = (self.nodi_visitati as f64 / e.as_secs_f64().max(0.001)) as u64;
        println!("info depth {} score cp {} nodes {} nps {} time {} pv {}", d, s, self.nodi_visitati, nps, e.as_millis(), 
                 self.pv_table[0].iter().map(|m| m.to_string()).collect::<Vec<_>>().join(" "));
    }
}