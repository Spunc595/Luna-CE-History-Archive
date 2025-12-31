use crate::board::{Scacchiera, Mossa};
use crate::evaluation::valuta_posizione;
use crate::transposition::{TranspositionTable, NodeType};
use std::sync::{Arc, RwLock};
use std::time::Instant;

pub const INFINITO: i32 = 1000000;

pub struct Motore {
    pub tt: Arc<RwLock<TranspositionTable>>,
    pub nodes: u64,
    pub startTime: Instant,
}

impl Motore {
    pub fn nuovo(tt: Arc<RwLock<TranspositionTable>>) -> Self {
        Motore { tt, nodes: 0, startTime: Instant::now() }
    }

    pub fn trova_mossa_migliore(&mut self, s: &mut Scacchiera, tempo_limite_ms: u32, uci_output: bool) -> Option<Mossa> {
        self.nodes = 0;
        self.startTime = Instant::now();
        let mut mossa_migliore = None;
        
        for depth in 1..=64 {
            let score = self.negamax(s, depth, -INFINITO, INFINITO);
            
            if let Some(entry) = self.tt.read().unwrap().get(s.hash()) {
                mossa_migliore = Some(entry.mossa);
                if uci_output {
                    let elapsed = self.startTime.elapsed().as_millis().max(1);
                    println!("info depth {} score cp {} nodes {} nps {} time {} pv {}", 
                        depth, score, self.nodes, (self.nodes as u128 * 1000) / elapsed, elapsed, entry.mossa);
                }
            }
            if self.startTime.elapsed().as_millis() > (tempo_limite_ms / 15) as u128 { break; }
        }
        mossa_migliore
    }

    pub fn negamax(&mut self, s: &mut Scacchiera, depth: i32, mut alpha: i32, beta: i32) -> i32 {
        if depth <= 0 { return self.quiescence(s, alpha, beta); }
        self.nodes += 1;

        let alpha_orig = alpha;
        if let Some(entry) = self.tt.read().unwrap().get(s.hash()) {
            if entry.depth >= depth {
                match entry.node_type {
                    NodeType::Exact => return entry.score,
                    NodeType::LowerBound => alpha = alpha.max(entry.score),
                    NodeType::UpperBound => if entry.score <= alpha_orig { return entry.score },
                }
                if alpha >= beta { return entry.score; }
            }
        }

        let mut mosse = crate::movegen::genera_mosse_legali(s);
        if mosse.is_empty() {
            return if crate::movegen::is_in_check(s, s.turno()) { -INFINITO + 100 } else { 0 };
        }

        // Move Ordering: Catture per prime
        mosse.sort_by_key(|m| if m.flag().is_capture() { -100 } else { 0 });

        let mut score_migliore = -INFINITO;
        let mut mossa_migliore_locale = mosse[0];

        for mossa in mosse {
            s.esegui_mossa(&mossa);
            let score = -self.negamax(s, depth - 1, -beta, -alpha);
            s.annulla_mossa();

            if score > score_migliore {
                score_migliore = score;
                mossa_migliore_locale = mossa;
            }
            alpha = alpha.max(score_migliore);
            if alpha >= beta { break; }
        }

        let node_type = if score_migliore <= alpha_orig { NodeType::UpperBound }
                        else if score_migliore >= beta { NodeType::LowerBound }
                        else { NodeType::Exact };

        self.tt.write().unwrap().save(s.hash(), depth, score_migliore, node_type, mossa_migliore_locale);
        score_migliore
    }

    fn quiescence(&mut self, s: &mut Scacchiera, mut alpha: i32, beta: i32) -> i32 {
        let stand_pat = valuta_posizione(s);
        if stand_pat >= beta { return beta; }
        alpha = alpha.max(stand_pat);

        let mut mosse = crate::movegen::genera_mosse_legali(s);
        mosse.retain(|m| m.flag().is_capture());

        for mossa in mosse {
            s.esegui_mossa(&mossa);
            let score = -self.quiescence(s, -beta, -alpha);
            s.annulla_mossa();
            if score >= beta { return beta; }
            alpha = alpha.max(score);
        }
        alpha
    }
}