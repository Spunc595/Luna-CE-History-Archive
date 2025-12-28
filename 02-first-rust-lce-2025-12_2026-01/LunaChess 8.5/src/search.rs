use crate::board::{Scacchiera, Mossa, MoveFlag};
use crate::movegen::{MovePicker};
use crate::evaluation::valuta_posizione;
use crate::transposition::{TranspositionTable, Bound};
use std::time::{Instant, Duration};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

const INFINITO: i16 = 30_000;
const MATE_VALUE: i16 = 29_000;
const MAX_PLY: usize = 96;

pub struct Motore {
    pub tt: Arc<TranspositionTable>,
    pub mossa_migliore: Option<Mossa>,
    pub nodi_visitati: u64,
    pub seldepth: usize,
    pub killer_moves: [[Option<Mossa>; 2]; MAX_PLY],
    pub pv_table: Vec<Vec<Mossa>>,
    start_time: Instant,
    allocated_time: u64,
    stop_flag: Arc<AtomicBool>,
}

impl Motore {
    pub fn nuovo(tt: Arc<TranspositionTable>) -> Self {
        let mut pv = Vec::with_capacity(MAX_PLY);
        for _ in 0..MAX_PLY { pv.push(Vec::with_capacity(MAX_PLY)); }
        Motore { tt, mossa_migliore: None, nodi_visitati: 0, seldepth: 0, killer_moves: [[None; 2]; MAX_PLY], pv_table: pv, start_time: Instant::now(), allocated_time: 0, stop_flag: Arc::new(AtomicBool::new(false)) }
    }
    pub fn set_stop_flag(&mut self, flag: Arc<AtomicBool>) { self.stop_flag = flag; }

    pub fn trova_mossa_migliore(&mut self, s: &mut Scacchiera, tempo: u64, is_main: bool) -> Option<Mossa> {
        self.start_time = Instant::now(); self.allocated_time = tempo; self.nodi_visitati = 0; self.stop_flag.store(false, Ordering::SeqCst);
        let (mut alpha, mut beta) = (-INFINITO, INFINITO);
        for depth in 1..=64 {
            let score = self.negamax(s, alpha, beta, depth, 0, true, true);
            if self.should_stop() { break; }
            if is_main {
                if !self.pv_table[0].is_empty() { self.mossa_migliore = Some(self.pv_table[0][0]); }
                self.print_info(depth, score, self.start_time.elapsed());
                if score <= alpha || score >= beta { alpha = -INFINITO; beta = INFINITO; }
                else { alpha = score - 30; beta = score + 30; }
            }
            if score.abs() > 28000 { break; }
        }
        self.mossa_migliore
    }

    fn negamax(&mut self, s: &mut Scacchiera, mut alpha: i16, mut beta: i16, mut depth: u8, ply: usize, is_pv: bool, can_null: bool) -> i16 {
        if self.should_stop() { return 0; }
        if ply >= MAX_PLY { return valuta_posizione(s) as i16; }
        if depth == 0 { return self.quiescence(s, alpha, beta, ply); }
        self.nodi_visitati += 1; self.seldepth = self.seldepth.max(ply);
        let hash = s.hash(); let mut tt_move = None;
        if let Some(e) = self.tt.probe(hash) {
            tt_move = e.mossa();
            if !is_pv && e.depth() >= depth {
                let sc = self.tt.score_from_tt(e.score(), ply);
                match e.bound() { Bound::Exact => return sc, Bound::Lower => if sc >= beta { return sc; }, Bound::Upper => if sc <= alpha { return sc; } }
            }
        }
        let in_check = s.re_in_scacco(s.turno());
        if !is_pv && !in_check {
            let eval = valuta_posizione(s) as i16;
            if depth <= 7 && eval - 75 * (depth as i16) >= beta { return eval; }
            if can_null && depth >= 3 && eval >= beta && s.ha_pezzi_maggiori(s.turno()) {
                s.esegui_mossa_nulla(); let sc = -self.negamax(s, -beta, -beta + 1, depth.saturating_sub(4), ply + 1, false, false); s.annulla_mossa_nulla();
                if sc >= beta { return beta; }
            }
        }
        if in_check { depth += 1; }
        let mut best_score = -INFINITO; let mut best_m = None; let mut count = 0;
        let mut picker = MovePicker::new(tt_move, self.killer_moves[ply]);
        while let Some(m) = picker.next(s) {
            if !s.esegui_mossa(&m) { continue; }
            count += 1;
            let score = if count == 1 { -self.negamax(s, -beta, -alpha, depth - 1, ply + 1, is_pv, true) }
            else {
                let r = if depth >= 3 && count > 4 && !in_check && !m.flag().is_capture() { 1 } else { 0 };
                let mut sc = -self.negamax(s, -alpha - 1, -alpha, depth - 1 - r, ply + 1, false, true);
                if sc > alpha { sc = -self.negamax(s, -beta, -alpha, depth - 1, ply + 1, true, true); } sc
            };
            s.annulla_mossa(); if self.should_stop() { return 0; }
            if score > best_score {
                best_score = score; best_m = Some(m);
                if score > alpha { alpha = score; self.update_pv(ply, m); if alpha >= beta { if !m.flag().is_capture() { self.killer_moves[ply][1] = self.killer_moves[ply][0]; self.killer_moves[ply][0] = Some(m); } break; } }
            }
        }
        if count == 0 { return if in_check { -29000 + ply as i16 } else { 0 }; }
        let bound = if best_score >= beta { Bound::Lower } else if is_pv && best_m.is_some() { Bound::Exact } else { Bound::Upper };
        self.tt.store(hash, self.tt.score_to_tt(best_score, ply), best_m.map(|m| (m.data & 0xFFFF) as u16).unwrap_or(0), depth, bound);
        best_score
    }

    fn quiescence(&mut self, s: &mut Scacchiera, mut alpha: i16, beta: i16, _ply: usize) -> i16 {
        let eval = valuta_posizione(s) as i16; if eval >= beta { return beta; } if eval > alpha { alpha = eval; }
        let mut picker = MovePicker::new(None, [None, None]);
        while let Some(m) = picker.next(s) {
            if !m.flag().is_capture() { continue; } if !s.esegui_mossa(&m) { continue; }
            let sc = -self.quiescence(s, -beta, -alpha, _ply + 1); s.annulla_mossa();
            if sc >= beta { return beta; } if sc > alpha { alpha = sc; }
        }
        alpha
    }

    fn should_stop(&self) -> bool {
        if self.stop_flag.load(Ordering::SeqCst) { return true; }
        if self.nodi_visitati & 2047 == 0 && self.allocated_time > 0 && self.start_time.elapsed().as_millis() as u64 > self.allocated_time { self.stop_flag.store(true, Ordering::SeqCst); return true; }
        false
    }
    fn update_pv(&mut self, ply: usize, m: Mossa) { self.pv_table[ply].clear(); self.pv_table[ply].push(m); if ply + 1 < MAX_PLY { let next = self.pv_table[ply + 1].clone(); self.pv_table[ply].extend(next); } }
    fn print_info(&self, d: u8, s: i16, e: Duration) {
        let nps = (self.nodi_visitati as f64 / e.as_secs_f64().max(0.001)) as u64;
        println!("info depth {} score cp {} nodes {} nps {} time {} pv {}", d, s, self.nodi_visitati, nps, e.as_millis(), self.pv_table[0].iter().map(|m| m.to_string()).collect::<Vec<_>>().join(" "));
    }
}