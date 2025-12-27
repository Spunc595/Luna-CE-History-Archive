// src/search.rs

use crate::board::{Scacchiera, Mossa, MoveFlag};
use crate::movegen::{self, MoveOrderer};
use crate::evaluation::valuta_posizione;
use crate::transposition::{TranspositionTable, Bound, TTEntry};
use std::time::{Instant, Duration};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

// ===== COSTANTI =====
const INFINITO: i16 = 30_000;
const MATE_VALUE: i16 = 29_000;
const MATE_BOUND: i16 = MATE_VALUE - 1000;
const MAX_PLY: usize = 64;

// ===== STRUTTURA MOTORE =====

// IMPORTANTE: Deve essere "pub" per essere vista da uci.rs
pub struct Motore {
    pub transposition_table: TranspositionTable,
    pub mossa_migliore: Option<Mossa>,
    pub nodi_visitati: u64,
    pub seldepth: usize,
    
    // Euristiche
    pub killer_moves: [[Option<Mossa>; 2]; MAX_PLY],
    pub history: [[i32; 64]; 64],
    pub pv_table: Vec<Vec<Mossa>>,
    
    // Gestione tempo
    start_time: Instant,
    allocated_time: u64,
    stop_flag: Arc<AtomicBool>,
    is_searching: bool,
}

impl Motore {
    pub fn nuovo(tt_size_mb: usize) -> Self {
        // Inizializza tabella PV
        let mut pv_table = Vec::with_capacity(MAX_PLY);
        for _ in 0..MAX_PLY {
            pv_table.push(Vec::with_capacity(MAX_PLY));
        }

        Motore {
            transposition_table: TranspositionTable::new(tt_size_mb),
            mossa_migliore: None,
            nodi_visitati: 0,
            seldepth: 0,
            killer_moves: [[None; 2]; MAX_PLY],
            history: [[0; 64]; 64],
            pv_table,
            start_time: Instant::now(),
            allocated_time: 0,
            stop_flag: Arc::new(AtomicBool::new(false)),
            is_searching: false,
        }
    }
    
    pub fn clear_heuristics(&mut self) {
        self.killer_moves = [[None; 2]; MAX_PLY];
        self.history = [[0; 64]; 64];
        self.transposition_table.clear();
    }

    pub fn set_stop_flag(&mut self, flag: Arc<AtomicBool>) {
        self.stop_flag = flag;
    }

    // ===== RICERCA PRINCIPALE (Iterative Deepening) =====

    pub fn trova_mossa_migliore(&mut self, scacchiera: &mut Scacchiera, tempo_ms: u64) -> Option<Mossa> {
        self.start_time = Instant::now();
        self.allocated_time = tempo_ms;
        self.nodi_visitati = 0;
        self.seldepth = 0;
        self.is_searching = true;
        self.stop_flag.store(false, Ordering::Relaxed);

        let mut best_move = None;
        let mut alpha = -INFINITO;
        let mut beta = INFINITO;
        
        // Genera mosse legali alla radice per check immediato
        let mosse_legali = movegen::genera_mosse_legali(scacchiera);
        if mosse_legali.is_empty() {
            return None; // Matto o stallo
        }
        
        // Se c'è una sola mossa, giocala subito (risparmia tempo)
        if mosse_legali.len() == 1 {
            return Some(mosse_legali[0]);
        }

        let max_depth = 64; 

        for depth in 1..=max_depth {
            // Nota: passiamo &mut scacchiera ma dentro usiamo i cloni
            let score = self.negamax(scacchiera, alpha, beta, depth, 0, true);

            if self.should_stop() {
                break;
            }

            // Recupera la mossa migliore dal PV
            if !self.pv_table[0].is_empty() {
                best_move = Some(self.pv_table[0][0]);
                self.mossa_migliore = best_move;
            } else if best_move.is_none() {
                // Fallback
                best_move = Some(mosse_legali[0]);
            }

            self.print_info(depth, score, self.start_time.elapsed());

            // Aspiration Windows
            if score <= alpha || score >= beta {
                alpha = -INFINITO;
                beta = INFINITO;
            } else {
                alpha = score - 50;
                beta = score + 50;
            }

            if score.abs() > MATE_BOUND {
                break;
            }
        }

        self.is_searching = false;
        best_move
    }

    // ===== NEGAMAX (Alpha-Beta + PVS + CLONE STRATEGY) =====

    fn negamax(
        &mut self,
        scacchiera: &Scacchiera, 
        mut alpha: i16,
        mut beta: i16,
        depth: u8,
        ply: usize,
        is_pv: bool,
    ) -> i16 {
        // 1. Controllo Tempo ogni 2048 nodi
        if self.nodi_visitati & 2047 == 0 {
            if self.should_stop() {
                return 0;
            }
        }

        // 2. Max Ply e Quiescence
        if ply >= MAX_PLY {
            return valuta_posizione(scacchiera) as i16;
        }
        
        if depth == 0 {
            return self.quiescence(scacchiera, alpha, beta, ply);
        }

        self.nodi_visitati += 1;
        self.seldepth = self.seldepth.max(ply);

        // 3. Transposition Table Probe
        let hash = scacchiera.hash();
        let mut tt_move = None;

        if let Some(entry) = self.transposition_table.probe(hash) {
            tt_move = entry.mossa();
            
            if !is_pv && entry.depth() >= depth {
                let score = self.transposition_table.score_from_tt(entry.score(), ply);
                match entry.bound() {
                    Bound::Exact => return score,
                    Bound::Lower => if score >= beta { return score; },
                    Bound::Upper => if score <= alpha { return score; },
                    _ => {}
                }
            }
        }
        
        // 4. Check Extension
        let in_check = scacchiera.re_in_scacco(scacchiera.turno());
        let extension = if in_check { 1 } else { 0 };
        let new_depth = depth + extension;

        // 5. Generazione Mosse
        let orderer = MoveOrderer::new(MAX_PLY); 
        
        // Copia per il generatore (richiede &mut)
        let mut scacchiera_gen = scacchiera.clone();
        let mosse = movegen::genera_e_filtra_mosse_ordinate(
            &mut scacchiera_gen, 
            &orderer, 
            tt_move.as_ref(), 
            depth as i32, 
            ply
        );

        // 6. Check Mate / Stallo
        if mosse.is_empty() {
            if in_check {
                return -MATE_VALUE + ply as i16;
            } else {
                return 0; // Stallo
            }
        }

        // 7. Loop Mosse con CLONE
        let mut best_score = -INFINITO;
        let mut best_move_found = None;
        let mut moves_searched = 0;

        for mossa in mosse {
            // === STRATEGIA SICURA: CLONE ===
            // Creiamo una copia pulita per ogni figlio.
            // Questo previene il bug "Illegal Move" dovuto a stati corrotti.
            let mut child_board = scacchiera.clone();
            
            // Esegui mossa sulla copia
            if !child_board.esegui_mossa(&mossa) {
                continue; 
            }
            
            moves_searched += 1;
            let mut score: i16;

            // Late Move Reduction (LMR)
            let needs_full_search = if moves_searched > 1 && depth > 2 && !in_check && !mossa.flag().is_capture() {
                let r = 1;
                score = -self.negamax(&child_board, -alpha - 1, -alpha, new_depth - 1 - r, ply + 1, false);
                score > alpha 
            } else {
                true
            };

            if needs_full_search {
                // PVS (Principal Variation Search)
                if moves_searched == 1 {
                    score = -self.negamax(&child_board, -beta, -alpha, new_depth - 1, ply + 1, true);
                } else {
                    // Zero Window Search
                    score = -self.negamax(&child_board, -alpha - 1, -alpha, new_depth - 1, ply + 1, false);
                    if score > alpha && score < beta {
                        // Re-search full window
                        score = -self.negamax(&child_board, -beta, -alpha, new_depth - 1, ply + 1, true);
                    }
                }
            } else {
                // Abbiamo già lo score da LMR
                 score = -self.negamax(&child_board, -beta, -alpha, new_depth - 1, ply + 1, true);
            }

            if self.should_stop() { return 0; }

            if score > best_score {
                best_score = score;
                best_move_found = Some(mossa);

                if score > alpha {
                    alpha = score;
                    self.update_pv(ply, mossa);

                    if alpha >= beta {
                        // Cutoff / Beta pruning
                        let tt_score = self.transposition_table.score_to_tt(beta, ply);
                        self.transposition_table.store(
                            hash, tt_score, 0, Some(mossa), depth, Bound::Lower
                        );
                        
                        // Aggiorna euristiche
                        if !mossa.flag().is_capture() {
                            self.history[mossa.da().indice()][mossa.a().indice()] += (depth * depth) as i32;
                            if self.killer_moves[ply][0] != Some(mossa) {
                                self.killer_moves[ply][1] = self.killer_moves[ply][0];
                                self.killer_moves[ply][0] = Some(mossa);
                            }
                        }
                        return beta;
                    }
                }
            }
        }

        // Store TT
        let bound = if best_score >= beta { Bound::Lower }
                    else if best_score > -INFINITO && best_move_found.is_some() { Bound::Exact }
                    else { Bound::Upper };
        
        let tt_score = self.transposition_table.score_to_tt(best_score, ply);
        self.transposition_table.store(
            hash, tt_score, 0, best_move_found, depth, bound
        );

        best_score
    }

    // ===== QUIESCENCE SEARCH =====

    fn quiescence(
        &mut self,
        scacchiera: &Scacchiera,
        mut alpha: i16,
        beta: i16,
        ply: usize,
    ) -> i16 {
        if self.nodi_visitati & 2047 == 0 && self.should_stop() {
            return 0;
        }
        self.nodi_visitati += 1;

        // Valutazione statica
        let stand_pat = valuta_posizione(scacchiera) as i16;
        if stand_pat >= beta {
            return beta;
        }
        if stand_pat > alpha {
            alpha = stand_pat;
        }

        // Genera solo catture
        let mut scacchiera_gen = scacchiera.clone();
        let mut mosse = movegen::genera_mosse_legali(&mut scacchiera_gen);
        
        // Filtra e ordina (MVV-LVA semplice)
        mosse.retain(|m| m.flag().is_capture() || m.is_promotion());
        mosse.sort_unstable_by_key(|m| {
            let victim = scacchiera.pezzo_su_casella(m.a()).map_or(0, |(p,_)| p.valore());
            -victim 
        });

        for mossa in mosse {
            // CLONE anche in quiescence
            let mut child_board = scacchiera.clone();
            if !child_board.esegui_mossa(&mossa) { continue; }

            let score = -self.quiescence(&child_board, -beta, -alpha, ply + 1);

            if score >= beta {
                return beta;
            }
            if score > alpha {
                alpha = score;
            }
        }

        alpha
    }

    // ===== UTILS =====

    fn should_stop(&self) -> bool {
        if self.stop_flag.load(Ordering::Relaxed) {
            return true;
        }
        if self.allocated_time > 0 {
            if self.start_time.elapsed().as_millis() as u64 > self.allocated_time {
                self.stop_flag.store(true, Ordering::Relaxed);
                return true;
            }
        }
        false
    }

    fn update_pv(&mut self, ply: usize, mossa: Mossa) {
        self.pv_table[ply].clear();
        self.pv_table[ply].push(mossa);
        if ply + 1 < MAX_PLY {
            let next_pv = self.pv_table[ply + 1].clone();
            self.pv_table[ply].extend(next_pv);
        }
    }

    fn print_info(&self, depth: u8, score: i16, elapsed: Duration) {
        let nps = (self.nodi_visitati as f64 / elapsed.as_secs_f64().max(0.0001)) as u64;
        let time_ms = elapsed.as_millis();
        
        let score_str = if score.abs() > MATE_BOUND {
            let moves = (MATE_VALUE - score.abs() + 1) / 2;
            if score > 0 { format!("mate {}", moves) } else { format!("mate -{}", moves) }
        } else {
            format!("cp {}", score)
        };

        print!("info depth {} seldepth {} score {} nodes {} nps {} time {} pv", 
               depth, self.seldepth, score_str, self.nodi_visitati, nps, time_ms);
        
        for m in &self.pv_table[0] {
            print!(" {}", m);
        }
        println!();
    }
}