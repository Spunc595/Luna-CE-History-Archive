use crate::board::{Scacchiera, Mossa, Colore};
use crate::movegen;
use crate::evaluation::valuta_posizione;
use crate::transposition::{TranspositionTable, Bound};
use std::time::Instant;
use std::sync::{Arc, RwLock};

const INFINITO: i16 = 30000;
const MATE_VALUE: i16 = 29000;
const MAX_QUIESCENCE_DEPTH: u8 = 10; // Limite di sicurezza per evitare loop infiniti

pub struct Motore {
    pub nodi_visitati: u64,
    pub start_time: Instant,
    pub tempo_limite_ms: u64,
    pub tt: Arc<RwLock<TranspositionTable>>,
    pub killer_moves: [[Option<Mossa>; 2]; 64],
    pub history_moves: [[i32; 64]; 6], 
}

impl Motore {
    pub fn nuovo(tt: Arc<RwLock<TranspositionTable>>) -> Self {
        Motore {
            nodi_visitati: 0,
            start_time: Instant::now(),
            tempo_limite_ms: 0,
            tt,
            killer_moves: [[None; 2]; 64],
            history_moves: [[0; 64]; 6],
        }
    }

    pub fn trova_mossa_migliore(&mut self, s: &mut Scacchiera, tempo_ms: u64, _is_main: bool) -> Option<Mossa> {
        self.nodi_visitati = 0;
        self.start_time = Instant::now();
        self.killer_moves = [[None; 2]; 64];
        self.history_moves = [[0; 64]; 6]; 
        
        let tempo_per_mossa = if tempo_ms > 0 { tempo_ms / 25 } else { 3000 };
        self.tempo_limite_ms = tempo_per_mossa;

        let mut mossa_migliore = None;
        let mosse_legali = movegen::genera_mosse_legali(s);
        if mosse_legali.is_empty() { return None; }
        mossa_migliore = Some(mosse_legali[0]);

        let mut alpha = -INFINITO;
        let mut beta = INFINITO;
        let mut score = 0;

        for depth in 1..=64 {
            // Aspiration Window
            if depth > 4 { 
                alpha = score - 50; 
                beta = score + 50; 
            } else { 
                alpha = -INFINITO; 
                beta = INFINITO; 
            }

            loop {
                let (val, m) = self.negamax(s, alpha, beta, depth, 0);
                score = val;
                
                if self.fuori_tempo() { break; }
                
                if val <= alpha { 
                    alpha = -INFINITO; 
                    continue; // Fall-low, riallarga finestra
                }
                if val >= beta { 
                    beta = INFINITO; 
                    continue; // Fall-high, riallarga finestra
                }

                if let Some(mossa) = m {
                    mossa_migliore = Some(mossa);
                    println!("info depth {} score cp {} nodes {} time {} pv {}", 
                        depth, score, self.nodi_visitati, self.start_time.elapsed().as_millis(), mossa);
                }
                break; 
            }

            if self.start_time.elapsed().as_millis() as u64 > (self.tempo_limite_ms * 6 / 10) { 
                break; 
            }
            if score.abs() > MATE_VALUE - 1000 { break; }
        }
        mossa_migliore
    }

    fn negamax(&mut self, s: &mut Scacchiera, mut alpha: i16, mut beta: i16, depth: u8, ply: usize) -> (i16, Option<Mossa>) {
        if self.nodi_visitati & 2047 == 0 && self.fuori_tempo() { return (0, None); }
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

        if depth == 0 { 
            return (self.quiescence(s, alpha, beta, 0), None); 
        }

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

        let mut best_m = None;
        let mut best_s = -INFINITO;
        let mut bnd = Bound::Upper;
        let mut moves_tried = 0;

        for mossa in mosse {
            // SEE Pruning: salta catture che perdono materiale (solo se non è mossa Hash)
            if ply > 0 && mossa.flag().is_capture() && Some(mossa) != mossa_tt {
                if movegen::see(s, mossa) < 0 {
                    continue; 
                }
            }

            s.esegui_mossa(&mossa);
            moves_tried += 1;
            
            let mut sc;
            
            if depth >= 3 && moves_tried > 4 && !in_check && !mossa.flag().is_capture() && mossa.promozione().is_none() {
                let reduction = 1 + (depth / 6);
                let (res, _) = self.negamax(s, -alpha - 1, -alpha, depth.saturating_sub(1 + reduction), ply + 1);
                sc = -res;
                if sc > alpha {
                     let (res, _) = self.negamax(s, -beta, -alpha, depth - 1, ply + 1);
                     sc = -res;
                }
            } else {
                let (res, _) = self.negamax(s, -beta, -alpha, depth - 1, ply + 1);
                sc = -res;
            }

            s.annulla_mossa();

            if sc > best_s {
                best_s = sc;
                best_m = Some(mossa);
                if sc > alpha {
                    alpha = sc;
                    bnd = Bound::Exact;
                    if alpha >= beta {
                        if !mossa.flag().is_capture() {
                            if ply < 64 {
                                self.killer_moves[ply][1] = self.killer_moves[ply][0];
                                self.killer_moves[ply][0] = Some(mossa);
                            }
                            if let Some((p, _)) = s.pezzo_su_casella(mossa.da()) {
                                let bonus = (depth as i32) * (depth as i32);
                                self.history_moves[p.indice()][mossa.a().indice()] += bonus;
                                if self.history_moves[p.indice()][mossa.a().indice()] > 2000 {
                                     self.history_moves[p.indice()][mossa.a().indice()] = 2000;
                                }
                            }
                        }
                        bnd = Bound::Lower;
                        break; 
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

    fn quiescence(&mut self, s: &mut Scacchiera, mut alpha: i16, beta: i16, q_ply: u8) -> i16 {
        // SAFETY: Limita la profondità della quiescence per evitare esplosioni di memoria
        if q_ply > MAX_QUIESCENCE_DEPTH {
            return valuta_posizione(s) as i16;
        }

        let stand_pat = valuta_posizione(s) as i16;
        if stand_pat >= beta { return beta; }
        if alpha < stand_pat { alpha = stand_pat; }
        
        let mut mosse = movegen::genera_mosse_legali(s);
        mosse.retain(|m| m.flag().is_capture());
        self.ordina_mosse(s, &mut mosse, None, 0);
        
        for mossa in mosse {
            if movegen::see(s, mossa) < 0 { continue; }

            s.esegui_mossa(&mossa); 
            let sc = -self.quiescence(s, -beta, -alpha, q_ply + 1); 
            s.annulla_mossa();
            if sc >= beta { return beta; } 
            if sc > alpha { alpha = sc; }
        }
        alpha
    }

    fn ordina_mosse(&self, s: &Scacchiera, mosse: &mut Vec<Mossa>, m_tt: Option<Mossa>, ply: usize) {
        mosse.sort_by_cached_key(|m| {
            if Some(*m) == m_tt { return -200000; }

            if m.flag().is_capture() {
                let v = s.pezzo_su_casella(m.a()).map_or(0, |(p, _)| p.valore());
                let a = s.pezzo_su_casella(m.da()).map_or(0, |(p, _)| p.valore());
                return -(100000 + v - (a/10));
            }

            if ply < 64 {
                if Some(*m) == self.killer_moves[ply][0] { return -90000; }
                if Some(*m) == self.killer_moves[ply][1] { return -80000; }
            }

            if let Some((p, _)) = s.pezzo_su_casella(m.da()) {
                return -self.history_moves[p.indice()][m.a().indice()];
            }

            0
        });
    }

    fn fuori_tempo(&self) -> bool {
        self.start_time.elapsed().as_millis() as u64 > self.tempo_limite_ms
    }
}