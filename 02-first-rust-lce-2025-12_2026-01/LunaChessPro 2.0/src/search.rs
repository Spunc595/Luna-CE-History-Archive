use crate::board::{Scacchiera, Mossa, Colore};
use crate::movegen;
use crate::nnue::Network;
use crate::transposition::TranspositionTable;
use std::time::Instant;
use std::sync::{Arc, RwLock};

const INFINITO: i16 = 30000;

pub struct Motore { pub nodi: u64, pub start_time: Instant, pub limit_ms: u64, pub stopped: bool, pub tt: Arc<RwLock<TranspositionTable>>, pub nnue: Arc<Network> }

impl Motore {
    pub fn trova_mossa_migliore(&mut self, s: &mut Scacchiera, time_ms: u64) -> Option<Mossa> {
        self.nodi = 0; self.start_time = Instant::now(); self.limit_ms = time_ms / 20; self.stopped = false;
        let mut best_m = None;
        for depth in 1..=12 {
            let (_, mossa) = self.negamax(s, -INFINITO, INFINITO, depth, 0);
            if self.stopped { break; }
            if let Some(m) = mossa { best_m = Some(m); }
        }
        best_m
    }

    fn negamax(&mut self, s: &mut Scacchiera, mut alpha: i16, beta: i16, depth: u8, ply: usize) -> (i16, Option<Mossa>) {
        if self.nodi & 2047 == 0 && self.start_time.elapsed().as_millis() as u64 > self.limit_ms { self.stopped = true; }
        if self.stopped { return (0, None); }
        if depth == 0 { return (self.nnue.valuta(s), None); }
        let mut moves = Vec::with_capacity(64);
        movegen::genera_tutte_mosse_pseudo_legali(s, &mut moves);
        let mut best_score = -INFINITO; let mut best_m = None;
        for m in moves {
            if !s.esegui_mossa(&m, &self.nnue) { continue; }
            if s.re_in_scacco(s.turno().opposto()) { s.annulla_mossa(); continue; }
            self.nodi += 1;
            let score = -self.negamax(s, -beta, -alpha, depth - 1, ply + 1).0;
            s.annulla_mossa();
            if score > best_score { best_score = score; best_m = Some(m); alpha = alpha.max(score); if alpha >= beta { break; } }
        }
        (best_score, best_m)
    }
}