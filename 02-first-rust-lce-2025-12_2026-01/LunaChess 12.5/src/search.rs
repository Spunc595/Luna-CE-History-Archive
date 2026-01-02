use crate::board::{Scacchiera, Mossa, Pezzo, MoveFlag, Colore};
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

    // --- ALBERO DELLE APERTURE INTEGRATO ---
    fn libro_aperture(&self, s: &Scacchiera) -> Option<Mossa> {
        let history = s.history_mosse_str();
        let n = history.len();
        let path = history.join(" ");

        match n {
            0 => self.crea_mossa("e2e4", s),
            2 => match history[1].as_str() {
                "e7e5" => self.crea_mossa("g1f3", s),
                "c7c5" => self.crea_mossa("g1f3", s),
                "e7e6" => self.crea_mossa("d2d4", s),
                "c7c6" => self.crea_mossa("d2d4", s),
                _ => None,
            },
            4 => match path.as_str() {
                "e2e4 e7e5 g1f3 b8c6" => self.crea_mossa("f1c4", s),
                "e2e4 c7c5 g1f3 d7d6" => self.crea_mossa("d2d4", s),
                _ => None,
            },
            6 => match path.as_str() {
                "e2e4 e7e5 g1f3 b8c6 f1c4 f8c5" => self.crea_mossa("c2c3", s),
                "e2e4 c7c5 g1f3 d7d6 d2d4 c5xd4" => self.crea_mossa("f3d4", s),
                _ => None,
            },
            8 => match path.as_str() {
                "e2e4 e7e5 g1f3 b8c6 f1c4 f8c5 c2c3 g8f6" => self.crea_mossa("d2d4", s),
                _ => None,
            },
            10 => match path.as_str() {
                "e2e4 e7e5 g1f3 b8c6 f1c4 f8c5 c2c3 g8f6 d2d4 e5xd4" => self.crea_mossa("e4e5", s),
                _ => None,
            },
            // Linea Lunga Giuoco Piano (fino a 12. Nxg6)
            12 => if path.contains("c2c3 g8f6 d2d3 d6") { self.crea_mossa("c1g5", s) } else { None },
            14 => if path.contains("g5 h6") { self.crea_mossa("g5h4", s) } else { None },
            16 => if path.contains("h4 Be6") { self.crea_mossa("b1d2", s) } else { None },
            18 => if path.contains("d2 Ne7") { self.crea_mossa("h4f6", s) } else { None },
            20 => if path.contains("f6 gxf6") { self.crea_mossa("f3h4", s) } else { None },
            22 => if path.contains("h4 Ng6") { self.crea_mossa("h4g6", s) } else { None },
            _ => None,
        }
    }

    fn crea_mossa(&self, uci: &str, s: &Scacchiera) -> Option<Mossa> {
        let mut mosse = Vec::with_capacity(64);
        movegen::genera_tutte_mosse_pseudo_legali(s, &mut mosse);
        mosse.into_iter().find(|m| m.to_string() == uci)
    }

    pub fn trova_mossa_migliore(&mut self, s: &mut Scacchiera, time_left_ms: u64, inc_ms: u64) -> Option<Mossa> {
        self.nodi = 0;
        self.start_time = Instant::now();
        self.stopped = false;

        if let Some(m_libro) = self.libro_aperture(s) {
            println!("info string apertura Luna Chess: Ultima Teoria Manuale");
            return Some(m_libro);
        }

        self.limit_ms = (time_left_ms / 20) + (inc_ms / 2);
        let mut best_m = None;
        let mut last_score = 0;

        for depth in 1..=64 {
            let (score, mossa) = self.aspiration_window(s, last_score, depth);
            if self.stopped { break; }
            last_score = score;
            if let Some(m) = mossa {
                best_m = Some(m);
                self.stampa_info(depth, score, m);
            }
            if score.abs() > MATE_SCORE - 100 { break; }
        }
        best_m
    }

    fn aspiration_window(&mut self, s: &mut Scacchiera, last_score: i16, depth: u8) -> (i16, Option<Mossa>) {
        let mut delta = 15;
        let mut alpha = (last_score - delta).max(-INFINITO);
        let mut beta = (last_score + delta).min(INFINITO);
        loop {
            let res = self.negamax(s, alpha, beta, depth, 0, true);
            if self.stopped { return res; }
            if res.0 <= alpha { alpha = (alpha - delta).max(-INFINITO); delta *= 2; }
            else if res.0 >= beta { beta = (beta + delta).min(INFINITO); delta *= 2; }
            else { return res; }
        }
    }

    fn negamax(&mut self, s: &mut Scacchiera, mut alpha: i16, mut beta: i16, mut depth: u8, ply: usize, allow_nmp: bool) -> (i16, Option<Mossa>) {
        if self.nodi & 2047 == 0 && self.check_time() { self.stopped = true; }
        if self.stopped { return (0, None); }

        alpha = alpha.max(-MATE_SCORE + ply as i16);
        beta = beta.min(MATE_SCORE - (ply as i16 + 1));
        if alpha >= beta { return (alpha, None); }

        if depth == 0 { return (self.quiescence(s, alpha, beta), None); }

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

        let in_check = s.re_in_scacco(s.turno());
        if in_check { depth += 1; }

        if allow_nmp && !in_check && depth >= 3 && s.ha_pezzi_maggiori(s.turno()) {
            s.esegui_mossa_nulla();
            let (sc, _) = self.negamax(s, -beta, -beta + 1, depth - 3, ply + 1, false);
            s.annulla_mossa();
            if -sc >= beta { return (beta, None); }
        }

        let mut raw_moves = Vec::with_capacity(64);
        movegen::genera_tutte_mosse_pseudo_legali(s, &mut raw_moves);
        let mut moves: Vec<_> = raw_moves.into_iter().map(|m| movegen::MossaConInfo { mossa: m, score: 0 }).collect();
        self.score_moves(s, &mut moves, tt_move, ply);

        let mut best_score = -INFINITO;
        let mut best_m = None;
        let mut legal_count = 0;

        for i in 0..moves.len() {
            if !s.esegui_mossa(&moves[i].mossa) { continue; }
            if s.re_in_scacco(s.turno().opposto()) { s.annulla_mossa(); continue; }
            self.nodi += 1;
            legal_count += 1;

            let mut score;
            // PVS (Principal Variation Search) + LMR (Late Move Reduction)
            if legal_count == 1 {
                score = -self.negamax(s, -beta, -alpha, depth - 1, ply + 1, true).0;
            } else {
                let reduction = if depth >= 3 && legal_count > 3 && !moves[i].mossa.flag().is_capture() && !in_check {
                    1 + (legal_count as u8 / 8)
                } else { 0 };

                score = -self.negamax(s, -alpha - 1, -alpha, depth.saturating_sub(1 + reduction), ply + 1, true).0;
                if score > alpha {
                    score = -self.negamax(s, -beta, -alpha, depth - 1, ply + 1, true).0;
                }
            }

            s.annulla_mossa();
            if self.stopped { return (0, None); }

            if score > best_score {
                best_score = score;
                best_m = Some(moves[i].mossa);
                if score > alpha {
                    alpha = score;
                    if alpha >= beta {
                        self.update_heuristics(moves[i].mossa, depth, ply, s);
                        break;
                    }
                }
            }
        }

        if legal_count == 0 { return (if in_check { -MATE_SCORE + ply as i16 } else { 0 }, None); }
        self.store_tt(hash, best_score, best_m, depth, alpha, beta, ply);
        (best_score, best_m)
    }

    fn quiescence(&mut self, s: &mut Scacchiera, mut alpha: i16, beta: i16) -> i16 {
        self.nodi += 1;
        let stand_pat = valuta_posizione(s) as i16;
        if stand_pat >= beta { return beta; }
        alpha = alpha.max(stand_pat);

        let mut raw_moves = Vec::with_capacity(32);
        let in_check = s.re_in_scacco(s.turno());
        if in_check { movegen::genera_tutte_mosse_pseudo_legali(s, &mut raw_moves); }
        else { movegen::genera_solo_catture(s, &mut raw_moves); }

        let mut moves: Vec<_> = raw_moves.into_iter().map(|m| movegen::MossaConInfo { mossa: m, score: 0 }).collect();
        self.score_moves(s, &mut moves, None, 0);

        for m in moves {
            if !in_check {
                let victim = s.get_piece_at(m.mossa.a().indice()).unwrap_or(Pezzo::Pedone);
                if stand_pat + victim.valore() as i16 + 200 < alpha { continue; }
            }
            if !s.esegui_mossa(&m.mossa) { continue; }
            if s.re_in_scacco(s.turno().opposto()) { s.annulla_mossa(); continue; }
            let score = -self.quiescence(s, -beta, -alpha);
            s.annulla_mossa();
            if score >= beta { return beta; }
            alpha = alpha.max(score);
        }
        alpha
    }

    fn score_moves(&self, s: &Scacchiera, moves: &mut [movegen::MossaConInfo], tt_m: Option<Mossa>, ply: usize) {
        for m in moves.iter_mut() {
            if Some(m.mossa) == tt_m { m.score = 2000000; }
            else if m.mossa.flag().is_promo() { m.score = 1000000; }
            else if m.mossa.flag().is_capture() {
                let victim = s.get_piece_at(m.mossa.a().indice()).unwrap_or(Pezzo::Pedone);
                let attacker = s.get_piece_at(m.mossa.da().indice()).unwrap_or(Pezzo::Pedone);
                m.score = 500000 + (victim.valore() * 10 - attacker.valore());
            } 
            else if ply < 128 && (Some(m.mossa) == self.killers[ply][0] || Some(m.mossa) == self.killers[ply][1]) { m.score = 100000; }
            else if let Some(p) = s.get_piece_at(m.mossa.da().indice()) { m.score = self.history[p.indice()][m.mossa.a().indice()]; }
        }
        moves.sort_unstable_by_key(|m| -m.score);
    }

    fn update_heuristics(&mut self, m: Mossa, depth: u8, ply: usize, s: &Scacchiera) {
        if ply < 128 {
            self.killers[ply][1] = self.killers[ply][0];
            self.killers[ply][0] = Some(m);
            if let Some(p) = s.get_piece_at(m.da().indice()) {
                self.history[p.indice()][m.a().indice()] += (depth as i32 * depth as i32);
            }
        }
    }

    fn store_tt(&self, h: u64, s: i16, m: Option<Mossa>, d: u8, a: i16, b: i16, ply: usize) {
        let bound = if s >= b { Bound::Lower } else if s > a { Bound::Exact } else { Bound::Upper };
        if let Ok(mut tt) = self.tt.write() { tt.store(h, s, m.map(|m| m.to_u16()).unwrap_or(0), d, bound, ply); }
    }

    fn check_time(&self) -> bool { self.start_time.elapsed().as_millis() as u64 > self.limit_ms }

    fn stampa_info(&self, d: u8, s: i16, m: Mossa) {
        let el = self.start_time.elapsed().as_millis();
        let nps = if el > 0 { (self.nodi as u128 * 1000) / el } else { 0 };
        println!("info depth {} score cp {} nodes {} time {} nps {} pv {}", d, s, self.nodi, el, nps, m);
    }
}