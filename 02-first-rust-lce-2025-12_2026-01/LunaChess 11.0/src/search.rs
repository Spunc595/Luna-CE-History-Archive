use crate::board::{Scacchiera, Mossa, Colore};
use crate::movegen;
use crate::evaluation::valuta_posizione;
use crate::transposition::{TranspositionTable, Bound};
use std::time::Instant;
use std::sync::{Arc, RwLock};

const INFINITO: i16 = 30000;
const MATE_SCORE: i16 = 29000;

pub struct Motore {
    pub nodi: u64,
    pub start_time: Instant,
    pub limit_ms: u64,
    pub tt: Arc<RwLock<TranspositionTable>>,
    pub killers: [[Option<Mossa>; 2]; 128],
    pub history: [[i32; 64]; 6],
}

impl Motore {
    pub fn nuovo(tt: Arc<RwLock<TranspositionTable>>) -> Self {
        Self { nodi: 0, start_time: Instant::now(), limit_ms: 0, tt, killers: [[None; 2]; 128], history: [[0; 64]; 6] }
    }

    pub fn trova_mossa_migliore(&mut self, s: &mut Scacchiera, ms: u64) -> Option<Mossa> {
        self.nodi = 0;
        self.start_time = Instant::now();
        self.limit_ms = ms / 25; // Allocazione tempo cauta
        let mut best_so_far = None;

        for depth in 1..=64 {
            let (score, mossa) = self.negamax(s, -INFINITO, INFINITO, depth, 0);
            if self.is_stop() && depth > 1 { break; }
            
            if let Some(m) = mossa {
                best_so_far = Some(m);
                println!("info depth {} score cp {} nodes {} time {} pv {}", 
                    depth, score, self.nodi, self.start_time.elapsed().as_millis(), m);
            }
            if score.abs() > MATE_SCORE - 100 { break; }
        }
        best_so_far
    }

    fn negamax(&mut self, s: &mut Scacchiera, mut alpha: i16, beta: i16, depth: u8, ply: usize) -> (i16, Option<Mossa>) {
        if self.nodi & 2047 == 0 && self.is_stop() { return (0, None); }
        if depth == 0 { return (self.quiescence(s, alpha, beta), None); }
        self.nodi += 1;

        let hash = s.hash();
        let mut tt_move = None;

        if let Ok(tt) = self.tt.read() {
            if let Some(e) = tt.probe(hash) {
                tt_move = Mossa::from_u16(e.mossa);
                if e.depth >= depth && ply > 0 {
                    let sc = tt.score_from_tt(e.score, ply);
                    match e.bound {
                        Bound::Exact => return (sc, tt_move),
                        Bound::Lower => if sc >= beta { return (sc, tt_move); },
                        Bound::Upper => if sc <= alpha { return (sc, tt_move); },
                    }
                }
            }
        }

        let mut raw_moves = Vec::with_capacity(64);
        movegen::genera_tutte_mosse_pseudo_legali(s, &mut raw_moves);
        let mut moves: Vec<_> = raw_moves.into_iter().map(|m| movegen::MossaConInfo { mossa: m, score: 0 }).collect();
        self.score_moves(s, &mut moves, tt_move, ply);

        let mut best_m = None;
        let mut best_score = -INFINITO;
        let mut legal_count = 0;

        for info in moves {
            if !s.esegui_mossa(&info.mossa) { continue; }
            if s.re_in_scacco(s.turno().opposto()) {
                s.annulla_mossa();
                continue;
            }
            legal_count += 1;
            let (score, _) = self.negamax(s, -beta, -alpha, depth - 1, ply + 1);
            let score = -score;
            s.annulla_mossa();

            if score > best_score {
                best_score = score;
                best_m = Some(info.mossa);
                if score > alpha {
                    alpha = score;
                    if alpha >= beta {
                        if !info.mossa.flag().is_capture() && ply < 128 {
                            self.killers[ply][1] = self.killers[ply][0];
                            self.killers[ply][0] = Some(info.mossa);
                        }
                        break;
                    }
                }
            }
        }

        if legal_count == 0 {
            return (if s.re_in_scacco(s.turno()) { -MATE_SCORE + ply as i16 } else { 0 }, None);
        }

        let bound = if best_score >= beta { Bound::Lower } else if best_score > alpha { Bound::Exact } else { Bound::Upper };
        if let Ok(mut tt) = self.tt.write() {
            tt.store(hash, best_score, best_m.map(|m| m.to_u16()).unwrap_or(0), depth, bound, ply);
        }

        (best_score, best_m)
    }

    fn quiescence(&mut self, s: &mut Scacchiera, mut alpha: i16, beta: i16) -> i16 {
        let stand_pat = valuta_posizione(s) as i16;
        if stand_pat >= beta { return beta; }
        if alpha < stand_pat { alpha = stand_pat; }

        let mut raw_moves = Vec::with_capacity(32);
        movegen::genera_tutte_mosse_pseudo_legali(s, &mut raw_moves);
        let mut captures: Vec<_> = raw_moves.into_iter()
            .filter(|m| m.flag().is_capture())
            .map(|m| movegen::MossaConInfo { mossa: m, score: 0 }).collect();
        
        self.score_moves(s, &mut captures, None, 0);

        for info in captures {
            if movegen::see(s, info.mossa) < 0 { continue; }
            if !s.esegui_mossa(&info.mossa) { continue; }
            if s.re_in_scacco(s.turno().opposto()) { s.annulla_mossa(); continue; }
            let score = -self.quiescence(s, -beta, -alpha);
            s.annulla_mossa();
            if score >= beta { return beta; }
            if score > alpha { alpha = score; }
        }
        alpha
    }

    fn score_moves(&self, s: &Scacchiera, moves: &mut [movegen::MossaConInfo], tt_m: Option<Mossa>, ply: usize) {
        for m in moves.iter_mut() {
            if Some(m.mossa) == tt_m { m.score = 1000000; }
            else if m.mossa.flag().is_capture() {
                let victim = s.get_piece_at(m.mossa.a().indice()).map_or(0, |p| p.valore());
                let attacker = s.get_piece_at(m.mossa.da().indice()).map_or(0, |p| p.valore());
                m.score = 500000 + (victim * 10 - attacker);
            } else if ply < 128 && (Some(m.mossa) == self.killers[ply][0] || Some(m.mossa) == self.killers[ply][1]) {
                m.score = 100000;
            } else if let Some(p) = s.get_piece_at(m.mossa.da().indice()) {
                m.score = self.history[p.indice()][m.mossa.a().indice()];
            }
        }
        moves.sort_unstable_by_key(|m| -m.score);
    }

    fn is_stop(&self) -> bool { self.start_time.elapsed().as_millis() as u64 > self.limit_ms }
}