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
    pub seldepth: u8,
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
        
        let mut best_move_iter = None;
        let mut alpha = -INFINITO;
        let mut beta = INFINITO;
        let mut last_score = 0;

        for depth in 1..=64 {
            // Aspiration Windows: stringe la ricerca intorno all'ultimo score noto
            let mut delta = 20;
            if depth >= 5 {
                alpha = (last_score - delta).max(-INFINITO);
                beta = (last_score + delta).min(INFINITO);
            }

            loop {
                let score = self.negamax(scacchiera, alpha, beta, depth, 0, true, true);
                
                if self.should_stop() { break; }

                // Se lo score esce dalla finestra, allarghiamo e riproviamo
                if score <= alpha {
                    alpha = (alpha - delta).max(-INFINITO);
                    beta = (beta + delta).min(INFINITO);
                    delta *= 2;
                } else if score >= beta {
                    beta = (beta + delta).min(INFINITO);
                    delta *= 2;
                } else {
                    last_score = score;
                    break;
                }
            }

            if self.should_stop() { break; }

            if is_main_thread && !self.pv_table[0].is_empty() {
                best_move_iter = Some(self.pv_table[0][0]);
                self.mossa_migliore = best_move_iter;
                self.print_info(depth, last_score, self.start_time.elapsed());
            }

            if last_score.abs() > MATE_BOUND { break; }
        }
        best_move_iter
    }

    fn negamax(&mut self, s: &mut Scacchiera, mut alpha: i16, mut beta: i16, mut depth: u8, ply: usize, is_pv: bool, can_null: bool) -> i16 {
        // Controllo tempo ogni 2048 nodi per efficienza
        if self.nodi_visitati & 2047 == 0 && self.should_stop() { return 0; }
        
        if ply >= MAX_PLY { return valuta_posizione(s) as i16; }
        self.seldepth = self.seldepth.max(ply as u8);

        if depth == 0 { return self.quiescence(s, alpha, beta, ply); }
        self.nodi_visitati += 1;

        // Verifica Ripetizione o Patta Materiale
        if ply > 0 && s.è_patta() { return 0; }

        let hash = s.hash();
        let mut tt_move = None;
        
        // Transposition Table Probe
        if let Some(e) = self.transposition_table.probe(hash) {
            tt_move = e.mossa();
            if !is_pv && e.depth >= depth {
                let sc = self.transposition_table.score_from_tt(e.score, ply);
                match e.bound {
                    Bound::Exact => return sc,
                    Bound::Lower => if sc >= beta { return sc; },
                    Bound::Upper => if sc <= alpha { return sc; },
                }
            }
        }

        let in_check = s.re_in_scacco(s.turno());
        if in_check { depth += 1; } // Check Extension semplice

        let static_eval = valuta_posizione(s) as i16;

        // Pruning per mosse non-PV
        if !is_pv && !in_check {
            // Razoring / Reverse Futility Pruning
            if depth <= 3 && static_eval - 120 * (depth as i16) >= beta {
                return static_eval;
            }

            // Null Move Pruning
            if can_null && depth >= 3 && static_eval >= beta && s.ha_pezzi_maggiori(s.turno()) {
                let r = 2 + depth / 6;
                s.esegui_mossa_nulla();
                let score = -self.negamax(s, -beta, -beta + 1, depth.saturating_sub(1 + r), ply + 1, false, false);
                s.annulla_mossa_nulla();
                if score >= beta { return beta; }
            }
        }

        let mut orderer = MoveOrderer::new(MAX_PLY);
        // Passiamo i dati delle euristiche per l'ordinamento
        let mosse = movegen::genera_e_filtra_mosse_ordinate(s, &orderer, tt_move.as_ref(), ply);
        
        if mosse.is_empty() {
            return if in_check { -MATE_VALUE + ply as i16 } else { 0 };
        }

        let mut best_score = -INFINITO;
        let mut best_m = None;
        let mut moves_searched = 0;

        for m in mosse {
            if !s.esegui_mossa(&m) { continue; }
            moves_searched += 1;
            
            let mut score;
            
            // PVS: Principal Variation Search
            if moves_searched == 1 {
                score = -self.negamax(s, -beta, -alpha, depth - 1, ply + 1, is_pv, true);
            } else {
                // LMR: Late Move Reductions
                let mut reduction = 0;
                if depth >= 3 && moves_searched > 4 && !in_check && !m.flag().is_capture() {
                    reduction = 1;
                    if moves_searched > 12 { reduction += 1; }
                }

                score = -self.negamax(s, -alpha - 1, -alpha, depth.saturating_sub(1 + reduction), ply + 1, false, true);
                
                // Ricerca completa se la riduzione fallisce
                if score > alpha && reduction > 0 {
                    score = -self.negamax(s, -alpha - 1, -alpha, depth - 1, ply + 1, false, true);
                }
                
                // Se la mossa è promettente, ricerca con finestra piena
                if score > alpha && score < beta {
                    score = -self.negamax(s, -beta, -alpha, depth - 1, ply + 1, true, true);
                }
            }

            s.annulla_mossa();
            
            if self.should_stop() { return 0; }

            if score > best_score {
                best_score = score;
                best_m = Some(m);
                if score > alpha {
                    alpha = score;
                    self.update_pv(ply, m);
                    if alpha >= beta {
                        // Aggiorna Killer e History solo su mosse "tranquille" (Beta Cut-off)
                        if !m.flag().is_capture() {
                            self.history[m.da().indice()][m.a().indice()] += (depth as i32) * (depth as i32);
                            if self.killer_moves[ply][0] != Some(m) {
                                self.killer_moves[ply][1] = self.killer_moves[ply][0];
                                self.killer_moves[ply][0] = Some(m);
                            }
                        }
                        break; 
                    }
                }
            }
        }

        let bound = if best_score >= beta { Bound::Lower } else if is_pv && best_m.is_some() { Bound::Exact } else { Bound::Upper };
        self.transposition_table.store(hash, self.transposition_table.score_to_tt(best_score, ply), best_m.map(|m| m.to_u16()).unwrap_or(0), depth, bound);
        
        best_score
    }

    fn quiescence(&mut self, s: &mut Scacchiera, mut alpha: i16, beta: i16, ply: usize) -> i16 {
        if self.nodi_visitati & 2047 == 0 && self.should_stop() { return 0; }
        
        let stand_pat = valuta_posizione(s) as i16;
        if stand_pat >= beta { return beta; }
        if stand_pat > alpha { alpha = stand_pat; }

        if ply >= MAX_PLY { return stand_pat; }

        // Genera solo catture per stabilizzare il punteggio
        let mut mosse = movegen::genera_mosse_legali(s);
        mosse.retain(|m| m.flag().is_capture());
        
        // Semplice ordinamento catture MVV-LVA per QSearch
        mosse.sort_by_key(|m| {
            let victim = s.pezzo_su_casella(m.a()).map_or(0, |(p,_)| p.valore());
            let attacker = s.pezzo_su_casella(m.da()).map_or(0, |(p,_)| p.valore());
            -(victim * 10 - attacker)
        });

        for m in mosse {
            if !s.esegui_mossa(&m) { continue; }
            let score = -self.quiescence(s, -beta, -alpha, ply + 1);
            s.annulla_mossa();
            
            if score >= beta { return beta; }
            if score > alpha { alpha = score; }
        }
        alpha
    }

    fn should_stop(&self) -> bool {
        if self.stop_flag.load(Ordering::Relaxed) { return true; }
        if self.allocated_time > 0 && self.start_time.elapsed().as_millis() as u64 > self.allocated_time {
            self.stop_flag.store(true, Ordering::Relaxed);
            return true;
        }
        false
    }

    fn update_pv(&mut self, ply: usize, m: Mossa) {
        self.pv_table[ply].clear();
        self.pv_table[ply].push(m);
        if ply + 1 < MAX_PLY {
            let next_pv = self.pv_table[ply + 1].clone();
            self.pv_table[ply].extend(next_pv);
        }
    }

    fn print_info(&self, d: u8, s: i16, e: Duration) {
        let nps = (self.nodi_visitati as f64 / e.as_secs_f64().max(0.001)) as u64;
        let score_str = if s.abs() > MATE_BOUND {
            let mate_in = (MATE_VALUE - s.abs() + 1) / 2;
            format!("mate {}", if s > 0 { mate_in } else { -mate_in as i16 })
        } else {
            format!("cp {}", s)
        };
        println!("info depth {} seldepth {} score {} nodes {} nps {} time {} pv {}", 
                 d, self.seldepth, score_str, self.nodi_visitati, nps, e.as_millis(), 
                 self.pv_table[0].iter().map(|m| m.to_string()).collect::<Vec<_>>().join(" "));
    }
}