use crate::board::{Scacchiera, Mossa, INFINITO};
use crate::movegen;
use crate::nnue::Network;
use crate::transposition::TranspositionTable;
use std::time::{Instant, Duration};
use std::sync::{Arc, RwLock};

pub struct Motore {
    pub nodi: u64,
    pub start_time: Instant,
    pub limit_ms: u64,
    pub stopped: bool,
    pub tt: Arc<RwLock<TranspositionTable>>,
    pub nnue: Arc<Network>,
}

impl Motore {
    pub fn trova_mossa_migliore(&mut self, s: &mut Scacchiera, time_ms: u64) -> Option<Mossa> {
        self.nodi = 0;
        self.start_time = Instant::now();
        self.limit_ms = time_ms;
        self.stopped = false;

        let mut best_m = None;

        for depth in 1..=12 {
            let (_score, mossa) = self.negamax(s, -INFINITO, INFINITO, depth, 0);
            if self.stopped { break; }
            if let Some(m) = mossa {
                best_m = Some(m);
            }
        }
        best_m
    }

    fn negamax(&mut self, s: &mut Scacchiera, mut alpha: i16, beta: i16, depth: u8, ply: usize) -> (i16, Option<Mossa>) {
        if self.nodi & 2047 == 0 && self.start_time.elapsed() >= Duration::from_millis(self.limit_ms) {
            self.stopped = true;
        }
        if self.stopped { return (0, None); }

        // 1. TT LOOKUP
        if let Some(entry) = self.tt.read().unwrap().get(s.hash) {
            if entry.depth >= depth {
                return (entry.score, entry.best_move);
            }
        }

        if depth == 0 {
            return (self.nnue.valuta(s), None);
        }

        self.nodi += 1;
        let mut moves = Vec::new();
        movegen::genera_tutte_mosse_pseudo_legali(s, &mut moves);

        let mut best_score = -INFINITO;
        let mut best_m = None;
        let mut legali = 0;

        for m in moves {
            let cap = s.get_piece_at(m.a());
            s.esegui_mossa(&m, &self.nnue);
            
            if s.re_in_scacco(s.turno.opposto()) {
                s.annulla_mossa(&m, cap);
                continue;
            }

            legali += 1;
            let (score_raw, _) = self.negamax(s, -beta, -alpha, depth - 1, ply + 1);
            let score = -score_raw;
            s.annulla_mossa(&m, cap);

            if score > best_score {
                best_score = score;
                best_m = Some(m);
                if score > alpha {
                    alpha = score;
                    if alpha >= beta { break; }
                }
            }
        }

        if legali == 0 {
            return (if s.re_in_scacco(s.turno) { -INFINITO + ply as i16 } else { 0 }, None);
        }

        // 2. TT STORE
        self.tt.write().unwrap().insert(s.hash, depth, best_score, best_m);

        (best_score, best_m)
    }
}