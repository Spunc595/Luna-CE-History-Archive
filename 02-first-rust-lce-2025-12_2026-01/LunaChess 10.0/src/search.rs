use crate::board::{Scacchiera, Mossa, Colore};
use crate::movegen;
use crate::evaluation::valuta_posizione;
use crate::transposition::{TranspositionTable, Bound};
use std::time::Instant;
use std::sync::{Arc, RwLock};

const INFINITO: i16 = 30000;
const MATE_VALUE: i16 = 29000;

pub struct Motore {
    pub nodi_visitati: u64,
    pub start_time: Instant,
    pub tempo_limite_ms: u64,
    pub tt: Arc<RwLock<TranspositionTable>>,
    pub killer_moves: [[Option<Mossa>; 2]; 64],
}

impl Motore {
    pub fn nuovo(tt: Arc<RwLock<TranspositionTable>>) -> Self {
        Motore { nodi_visitati: 0, start_time: Instant::now(), tempo_limite_ms: 0, tt, killer_moves: [[None; 2]; 64] }
    }

    pub fn trova_mossa_migliore(&mut self, s: &mut Scacchiera, tempo_ms: u64, _is_main: bool) -> Option<Mossa> {
        self.nodi_visitati = 0; self.start_time = Instant::now();
        self.killer_moves = [[None; 2]; 64];
        self.tempo_limite_ms = if tempo_ms > 0 { tempo_ms / 20 } else { 3000 };

        let mut mossa_migliore = None;
        let mosse_legali = movegen::genera_mosse_legali(s);
        if mosse_legali.is_empty() { return None; }
        mossa_migliore = Some(mosse_legali[0]);

        for depth in 1..=64 {
            let (score, m) = self.negamax(s, -INFINITO, INFINITO, depth, 0);
            if self.fuori_tempo() { break; }
            if let Some(mossa) = m {
                if let Some((_, col)) = s.pezzo_su_casella(mossa.da()) {
                    if col == s.turno() {
                        mossa_migliore = Some(mossa);
                        println!("info depth {} score cp {} nodes {} time {} pv {}", depth, score, self.nodi_visitati, self.start_time.elapsed().as_millis(), mossa);
                    }
                }
            }
            if score.abs() > MATE_VALUE - 500 { break; }
        }
        mossa_migliore
    }

    fn negamax(&mut self, s: &mut Scacchiera, mut alpha: i16, beta: i16, depth: u8, ply: usize) -> (i16, Option<Mossa>) {
        if self.nodi_visitati & 1023 == 0 && self.fuori_tempo() { return (0, None); }
        self.nodi_visitati += 1;

        let hash = s.hash();
        let mut mossa_tt = None;

        if let Ok(tt) = self.tt.read() {
            if let Some(e) = tt.probe(hash) {
                mossa_tt = Mossa::from_u16(e.mossa);
                if e.depth >= depth && ply > 0 {
                    let sc = tt.score_from_tt(e.score, ply);
                    match e.bound {
                        Bound::Exact => return (sc, mossa_tt),
                        Bound::Lower => if sc >= beta { return (sc, mossa_tt); },
                        Bound::Upper => if sc <= alpha { return (sc, mossa_tt); },
                    }
                }
            }
        }

        if depth == 0 { return (self.quiescence(s, alpha, beta), None); }
        let in_check = s.re_in_scacco(s.turno());

        if depth >= 3 && !in_check && ply > 0 && s.ha_pezzi_maggiori(s.turno()) {
            s.esegui_mossa_nulla();
            let (sc, _) = self.negamax(s, -beta, -beta + 1, depth - 3, ply + 1);
            s.annulla_mossa_nulla();
            if -sc >= beta { return (beta, None); }
        }

        let mut mosse = movegen::genera_mosse_legali(s);
        if mosse.is_empty() { return (if in_check { -MATE_VALUE + ply as i16 } else { 0 }, None); }
        self.ordina_mosse(s, &mut mosse, mossa_tt, ply);

        let mut best_m: Option<Mossa> = None; 
        let mut best_s = -INFINITO; 
        let mut bnd = Bound::Upper;
        let mut moves_tried = 0;

        for mossa in mosse {
            s.esegui_mossa(&mossa);
            moves_tried += 1;
            
            let mut sc;
            
            // --- LOGICA LMR (Late Move Reductions) ---
            if depth >= 3 && moves_tried > 4 && !in_check && !mossa.flag().is_capture() && mossa.promozione().is_none() {
                // Ricerca a profondità ridotta
                let reduction = 1; 
                let (res, _) = self.negamax(s, -alpha - 1, -alpha, depth - 1 - reduction, ply + 1);
                sc = -res;
                
                // Se la mossa sembra promettente nonostante la riduzione, rifacciamo la ricerca piena
                if sc > alpha {
                    let (res, _) = self.negamax(s, -beta, -alpha, depth - 1, ply + 1);
                    sc = -res;
                }
            } else {
                // Ricerca normale PVS (Principal Variation Search) semplificata
                let (res, _) = self.negamax(s, -beta, -alpha, depth - 1, ply + 1);
                sc = -res;
            }

            s.annulla_mossa();

            if sc > best_s {
                best_s = sc; best_m = Some(mossa);
                if sc > alpha {
                    alpha = sc; bnd = Bound::Exact;
                    if alpha >= beta {
                        if !mossa.flag().is_capture() && ply < 64 {
                            self.killer_moves[ply][1] = self.killer_moves[ply][0];
                            self.killer_moves[ply][0] = Some(mossa);
                        }
                        bnd = Bound::Lower; break;
                    }
                }
            }
        }

        if !self.fuori_tempo() {
            if let Ok(mut tt) = self.tt.write() {
                tt.store(hash, best_s, best_m.map(|m| m.to_u16()).unwrap_or(0), depth, bnd, ply);
            }
        }
        (best_s, best_m)
    }

    fn quiescence(&mut self, s: &mut Scacchiera, mut alpha: i16, beta: i16) -> i16 {
        let stand_pat = valuta_posizione(s) as i16;
        if stand_pat >= beta { return beta; }
        if alpha < stand_pat { alpha = stand_pat; }
        let mut mosse = movegen::genera_mosse_legali(s);
        mosse.retain(|m| m.flag().is_capture());
        self.ordina_mosse(s, &mut mosse, None, 0);
        for mossa in mosse {
            s.esegui_mossa(&mossa); let sc = -self.quiescence(s, -beta, -alpha); s.annulla_mossa();
            if sc >= beta { return beta; } if sc > alpha { alpha = sc; }
        }
        alpha
    }

    fn ordina_mosse(&self, s: &Scacchiera, mosse: &mut Vec<Mossa>, m_tt: Option<Mossa>, ply: usize) {
        mosse.sort_by_cached_key(|m| {
            if Some(*m) == m_tt { return -100000; }
            if m.flag().is_capture() {
                let v = s.pezzo_su_casella(m.a()).map_or(0, |(p, _)| p.valore());
                let a = s.pezzo_su_casella(m.da()).map_or(0, |(p, _)| p.valore());
                return -(20000 + v - (a/10));
            }
            if ply < 64 {
                if Some(*m) == self.killer_moves[ply][0] { return -6000; }
                if Some(*m) == self.killer_moves[ply][1] { return -5000; }
            }
            if m.promozione().is_some() { return -8000; }
            0
        });
    }

    fn fuori_tempo(&self) -> bool { self.start_time.elapsed().as_millis() as u64 > self.tempo_limite_ms }
}