use crate::board::{Scacchiera, Mossa, Pezzo, MoveFlag};
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
    pub stopped: bool,
    pub tt: Arc<RwLock<TranspositionTable>>,
    pub killers: [[Option<Mossa>; 2]; 128],
    pub history: [[i32; 64]; 6],
}

impl Motore {
    pub fn nuovo(tt: Arc<RwLock<TranspositionTable>>) -> Self {
        Self { 
            nodi: 0, 
            start_time: Instant::now(), 
            limit_ms: 0, 
            stopped: false,
            tt, 
            killers: [[None; 2]; 128], 
            history: [[0; 64]; 6] 
        }
    }

    pub fn trova_mossa_migliore(&mut self, s: &mut Scacchiera, time_left_ms: u64, inc_ms: u64) -> Option<Mossa> {
        self.nodi = 0;
        self.start_time = Instant::now();
        self.stopped = false;

        let allocazione = (time_left_ms / 20) + (inc_ms / 2);
        self.limit_ms = if allocazione > 50 { allocazione - 50 } else { 50 };

        let mut best_move_total = None;

        for depth in 1..=64 {
            // allow_nmp è true (parametro non utilizzato per ora, sopprimiamo il warning con _)
            let (score, mossa) = self.negamax(s, -INFINITO, INFINITO, depth, 0, true);

            if self.stopped { break; }

            if let Some(m) = mossa {
                best_move_total = Some(m);
                
                let elapsed = self.start_time.elapsed().as_millis();
                let nps = if elapsed > 0 { (self.nodi as u128 * 1000) / elapsed } else { 0 };
                
                let score_str = if score > MATE_SCORE - 100 {
                    format!("mate {}", (MATE_SCORE - score + 1) / 2)
                } else if score < -MATE_SCORE + 100 {
                    format!("mate {}", -(MATE_SCORE + score + 1) / 2)
                } else {
                    format!("cp {}", score)
                };

                println!("info depth {} score {} nodes {} time {} nps {} pv {}", 
                    depth, score_str, self.nodi, elapsed, nps, m);
            }

            if score.abs() > MATE_SCORE - 100 { break; }
        }

        best_move_total
    }

    fn negamax(&mut self, s: &mut Scacchiera, mut alpha: i16, beta: i16, depth: u8, ply: usize, _allow_nmp: bool) -> (i16, Option<Mossa>) {
        if self.nodi & 2047 == 0 {
            if self.check_time() { self.stopped = true; }
        }
        if self.stopped { return (0, None); }

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
        
        let mut moves: Vec<_> = raw_moves.into_iter()
            .map(|m| movegen::MossaConInfo { mossa: m, score: 0 })
            .collect();
        
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

            let (val, _) = self.negamax(s, -beta, -alpha, depth - 1, ply + 1, true);
            let score = -val;
            s.annulla_mossa();

            if self.stopped { return (0, None); }

            if score > best_score {
                best_score = score;
                best_m = Some(info.mossa);
                
                if score > alpha {
                    alpha = score;
                    if alpha >= beta {
                        if !self.is_capture_or_promo(info.mossa.flag()) && ply < 128 {
                            self.killers[ply][1] = self.killers[ply][0];
                            self.killers[ply][0] = Some(info.mossa);
                            
                            if let Some(p) = s.get_piece_at(info.mossa.da().indice()) {
                                let bonus = (depth as i32 * depth as i32).min(300);
                                self.history[p.indice()][info.mossa.a().indice()] += bonus;
                            }
                        }
                        break; 
                    }
                }
            }
        }

        if legal_count == 0 {
            if s.re_in_scacco(s.turno()) {
                return (-MATE_SCORE + ply as i16, None);
            } else {
                return (0, None);
            }
        }

        if !self.stopped {
            let bound = if best_score >= beta { Bound::Lower } 
                        else if best_score > alpha { Bound::Exact } 
                        else { Bound::Upper };
            
            if let Ok(mut tt) = self.tt.write() {
                tt.store(hash, best_score, best_m.map(|m| m.to_u16()).unwrap_or(0), depth, bound, ply);
            }
        }

        (best_score, best_m)
    }

    fn quiescence(&mut self, s: &mut Scacchiera, mut alpha: i16, beta: i16) -> i16 {
        if self.nodi & 2047 == 0 && self.check_time() { self.stopped = true; return 0; }
        self.nodi += 1;

        let stand_pat = valuta_posizione(s) as i16;
        if stand_pat >= beta { return beta; }
        if alpha < stand_pat { alpha = stand_pat; }

        let mut raw_moves = Vec::with_capacity(32);
        movegen::genera_tutte_mosse_pseudo_legali(s, &mut raw_moves);
        
        let mut captures: Vec<_> = raw_moves.into_iter()
            .filter(|m| self.is_capture_or_promo(m.flag()))
            .map(|m| movegen::MossaConInfo { mossa: m, score: 0 })
            .collect();
        
        self.score_moves(s, &mut captures, None, 0);

        for info in captures {
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
            let f = m.mossa.flag();
            
            // 1. Hash Move
            if Some(m.mossa) == tt_m { 
                m.score = 2_000_000; 
            }
            // 2. Promozioni (priorità alta)
            else if f == MoveFlag::Promotion || f == MoveFlag::PromotionCapture {
                m.score = 1_000_000;
            }
            // 3. Catture
            else if f == MoveFlag::Capture || f == MoveFlag::PromotionCapture || f == MoveFlag::EnPassant {
                let victim_val = self.mvv_lva_value(s, m.mossa.a().indice());
                let attacker_val = self.mvv_lva_value(s, m.mossa.da().indice());
                m.score = 500_000 + (victim_val * 10 - attacker_val);
            } 
            // 4. Killer Moves
            else if ply < 128 && (Some(m.mossa) == self.killers[ply][0] || Some(m.mossa) == self.killers[ply][1]) {
                m.score = 100_000;
            } 
            // 5. History Heuristic
            else if let Some(p) = s.get_piece_at(m.mossa.da().indice()) {
                m.score = self.history[p.indice()][m.mossa.a().indice()].min(50_000);
            }
        }
        moves.sort_unstable_by_key(|m| -m.score);
    }

    // Helper per controllare se è cattura o promozione senza metodi esterni
    fn is_capture_or_promo(&self, f: MoveFlag) -> bool {
        match f {
            MoveFlag::Capture | MoveFlag::Promotion | MoveFlag::PromotionCapture | MoveFlag::EnPassant => true,
            _ => false
        }
    }

    fn mvv_lva_value(&self, s: &Scacchiera, idx: usize) -> i32 {
        if let Some(p) = s.get_piece_at(idx) {
            match p { 
                Pezzo::Pedone => 1,
                Pezzo::Cavallo => 3,
                Pezzo::Alfiere => 3,
                Pezzo::Torre => 5,
                Pezzo::Regina => 9,
                Pezzo::Re => 0, 
            }
        } else {
            0
        }
    }

    fn check_time(&self) -> bool {
        self.start_time.elapsed().as_millis() as u64 > self.limit_ms
    }
}