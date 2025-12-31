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
        Motore {
            tt,
            nodes: 0,
            startTime: Instant::now(),
        }
    }

    pub fn trova_mossa_migliore(&mut self, s: &mut Scacchiera, tempo_limite_ms: u32, uci_output: bool) -> Option<Mossa> {
        self.nodes = 0;
        self.startTime = Instant::now();
        let mut mossa_migliore = None;
        
        for depth in 1..=64 {
            let score = self.negamax(s, depth, -INFINITO, INFINITO);
            
            // Chiamata corretta s.hash() e tt.get()
            if let Some(entry) = self.tt.read().unwrap().get(s.hash()) {
                mossa_migliore = Some(entry.mossa);
                
                if uci_output {
                    let elapsed = self.startTime.elapsed().as_millis();
                    println!("info depth {} score cp {} nodes {} time {} pv {}", 
                        depth, score, self.nodes, elapsed, entry.mossa);
                }
            }

            if self.startTime.elapsed().as_millis() > (tempo_limite_ms / 10) as u128 {
                break;
            }
        }
        mossa_migliore
    }

    pub fn negamax(&mut self, s: &mut Scacchiera, depth: i32, mut alpha: i32, beta: i32) -> i32 {
        self.nodes += 1;
        let alpha_orig = alpha;

        // Lookup in TT
        if let Some(entry) = self.tt.read().unwrap().get(s.hash()) {
            if entry.depth >= depth {
                match entry.node_type {
                    NodeType::Exact => return entry.score,
                    NodeType::LowerBound => alpha = alpha.max(entry.score),
                    NodeType::UpperBound => {} // Continua ricerca
                }
                if alpha >= beta { return entry.score; }
            }
        }

        if depth <= 0 {
            return valuta_posizione(s);
        }

        let mosse = crate::movegen::genera_mosse_legali(s);
        if mosse.is_empty() {
            // Verifica se è scacco matto o patta (controlla il nome in movegen.rs)
            // Se in_scacco non esiste, usa la funzione corretta del tuo movegen
            return if crate::movegen::is_in_check(s, s.turno()) { -INFINITO + 100 } else { 0 };
        }

        let mut mossa_migliore_locale = mosse[0];
        let mut score_migliore = -INFINITO;

        for mossa in mosse {
            s.esegui_mossa(&mossa);
            let score = -self.negamax(s, depth - 1, -beta, -alpha);
            s.annulla_mossa(); // Corretto: rimosso l'argomento &mossa

            if score > score_migliore {
                score_migliore = score;
                mossa_migliore_locale = mossa;
            }
            alpha = alpha.max(score_migliore);
            if alpha >= beta { break; }
        }

        let node_type = if score_migliore <= alpha_orig {
            NodeType::UpperBound
        } else if score_migliore >= beta {
            NodeType::LowerBound
        } else {
            NodeType::Exact
        };

        // Salvataggio corretto in TT
        self.tt.write().unwrap().save(s.hash(), depth, score_migliore, node_type, mossa_migliore_locale);

        score_migliore
    }
}