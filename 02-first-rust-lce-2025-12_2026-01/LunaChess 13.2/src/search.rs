use crate::board::{Scacchiera, Mossa, INFINITO};
use crate::movegen;
use crate::move_ordering;
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

        let mut ultima_best_m = None;

        // --- ITERATIVE DEEPENING ---
        // Aumentiamo la profondità gradualmente finché c'è tempo
        for depth in 1..=12 {
            let (score, mossa) = self.negamax(s, -INFINITO, INFINITO, depth, 0);

            // Se il tempo è scaduto durante questa iterazione, non sovrascriviamo la mossa precedente
            if self.stopped && depth > 1 { break; }

            if let Some(m) = mossa {
                ultima_best_m = Some(m);
                let elapsed = self.start_time.elapsed().as_millis();
                println!(
                    "info depth {} score cp {} nodes {} time {} pv {}",
                    depth, score, self.nodi, elapsed, m
                );
            }

            // Se abbiamo già usato più della metà del tempo, non iniziamo un'altra depth
            if self.start_time.elapsed().as_millis() > (time_ms / 2) as u128 {
                break;
            }
        }
        
        ultima_best_m
    }

    fn negamax(&mut self, s: &mut Scacchiera, mut alpha: i16, beta: i16, depth: u8, ply: usize) -> (i16, Option<Mossa>) {
        // Controllo periodico del tempo
        if self.nodi & 2047 == 0 && self.start_time.elapsed() >= Duration::from_millis(self.limit_ms) {
            self.stopped = true;
        }
        if self.stopped { return (0, None); }

        // 1. TT LOOKUP
        let mut tt_move = None;
        if let Some(entry) = self.tt.read().unwrap().get(s.hash) {
            if entry.depth >= depth {
                return (entry.score, entry.best_move);
            }
            tt_move = entry.best_move;
        }

        // 2. BASE CASE (Depth 0)
        if depth == 0 {
            return (self.quiesce(s, alpha, beta), None);
        }

        self.nodi += 1;
        let mut moves = Vec::new();
        movegen::genera_tutte_mosse_pseudo_legali(s, &mut moves);

        // --- MOVE ORDERING ---
        moves.sort_by_cached_key(|m| -move_ordering::punteggio_mossa(s, m, tt_move));

        let mut best_score = -INFINITO;
        let mut best_m = None;
        let mut legali = 0;

        for m in moves {
            let cap = s.get_piece_at(m.a());
            s.esegui_mossa(&m, &self.nnue);
            
            if s.re_in_scacco(s.turno.opposto()) {
                s.annulla_mossa(&m, cap, &self.nnue);
                continue;
            }

            legali += 1;
            let (score_raw, _) = self.negamax(s, -beta, -alpha, depth - 1, ply + 1);
            let score = -score_raw;
            s.annulla_mossa(&m, cap, &self.nnue);

            if score > best_score {
                best_score = score;
                best_m = Some(m);
                if score > alpha {
                    alpha = score;
                    if alpha >= beta { break; } // Potatura Beta
                }
            }
        }

        // Gestione Scacco Matto / Stallo
        if legali == 0 {
            return (if s.re_in_scacco(s.turno) { -INFINITO + ply as i16 } else { 0 }, None);
        }

        // 3. TT STORE
        self.tt.write().unwrap().insert(s.hash, depth, best_score, best_m);

        (best_score, best_m)
    }

    // --- QUIESCENCE SEARCH ---
    // Serve a risolvere le catture pendenti per evitare errori di valutazione tattica
    fn quiesce(&mut self, s: &mut Scacchiera, mut alpha: i16, beta: i16) -> i16 {
        let stand_pat = self.nnue.valuta(s);
        if stand_pat >= beta { return beta; }
        if alpha < stand_pat { alpha = stand_pat; }

        let mut moves = Vec::new();
        movegen::genera_tutte_mosse_pseudo_legali(s, &mut moves);
        
        // In Quiescence consideriamo solo le catture
        moves.retain(|m| s.get_piece_at(m.a()).is_some());
        moves.sort_by_cached_key(|m| -move_ordering::punteggio_mossa(s, m, None));

        for m in moves {
            let cap = s.get_piece_at(m.a());
            s.esegui_mossa(&m, &self.nnue);
            if s.re_in_scacco(s.turno.opposto()) {
                s.annulla_mossa(&m, cap, &self.nnue);
                continue;
            }
            let score = -self.quiesce(s, -beta, -alpha);
            s.annulla_mossa(&m, cap, &self.nnue);

            if score >= beta { return beta; }
            if score > alpha { alpha = score; }
        }
        alpha
    }
}