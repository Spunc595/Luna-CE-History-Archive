use std::io::{self, BufRead};
use std::sync::{Arc, Mutex};
use std::thread;
use crate::board::{Scacchiera, Colore};
use crate::search::{SearchInfo, iterative_deepening, MAX_DEPTH};
use crate::tt::TranspositionTable;
use crate::zobrist::ZobristKeys;
use crate::nnue::LunaNNUE;
use crate::book::OpeningBook;

pub struct UCI {
    board: Scacchiera,
    tt: Arc<Mutex<TranspositionTable>>,
    book: Option<OpeningBook>,
    nnue: Option<Arc<LunaNNUE>>,
    zobrist: ZobristKeys,
}

impl UCI {
    pub fn new(nnue_option: Option<LunaNNUE>) -> Self {
        let zobrist = ZobristKeys::default();
        let nnue = nnue_option.map(Arc::new);
        let mut board = Scacchiera::new_iniziale(&zobrist);
        
        if let Some(net) = &nnue {
            board.inizializza_nnue(net);
        }

        UCI {
            board,
            tt: Arc::new(Mutex::new(TranspositionTable::new(256))),
            book: OpeningBook::load("book.bin"),
            nnue,
            zobrist,
        }
    }

    pub fn run_loop(&mut self) {
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            let line = line.unwrap();
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() { continue; }

            match parts[0] {
                "uci" => {
                    println!("id name Luna_Hybrid_7.2");
                    println!("id author Daniele");
                    println!("option name Hash type spin default 256 min 1 max 1024");
                    println!("uciok");
                }
                "isready" => println!("readyok"),
                "ucinewgame" => {
                    self.tt.lock().unwrap().clear();
                    self.board = Scacchiera::new_iniziale(&self.zobrist);
                    if let Some(net) = &self.nnue { self.board.inizializza_nnue(net); }
                }
                "position" => self.parse_position(&parts),
                "go" => self.parse_go(&parts),
                "eval" => self.print_eval(),
                "quit" => break,
                _ => {}
            }
        }
    }

    fn print_eval(&mut self) {
        if let Some(net) = &self.nnue {
            // Sincronizzazione di sicurezza prima del calcolo manuale
            self.board.inizializza_nnue(net);
            let score = net.evaluate(&self.board.accumulator, self.board.turno);
            println!("Evaluation: {} cp (NNUE)", score);
        } else {
            // Fallback se la rete non è carica
            println!("Evaluation: {} cp (Static HCE)", crate::evaluation::evaluate(&self.board, None));
        }
    }

    fn parse_position(&mut self, parts: &[&str]) {
        let mut moves_idx = None;

        if parts.len() < 2 { return; }

        if parts[1] == "startpos" {
            self.board = Scacchiera::new_iniziale(&self.zobrist);
            moves_idx = parts.iter().position(|&x| x == "moves");
        } else if parts[1] == "fen" {
            let m_pos = parts.iter().position(|&x| x == "moves").unwrap_or(parts.len());
            let fen_str = parts[2..m_pos].join(" ");
            self.board = Scacchiera::from_fen(&fen_str, &self.zobrist);
            moves_idx = parts.iter().position(|&x| x == "moves");
        }

        // Ricalcolo totale obbligatorio dell'accumulatore NNUE dopo un cambio posizione radicale
        if let Some(net) = &self.nnue {
            self.board.inizializza_nnue(net);
        }

        // Applica le mosse della sequenza UCI (es. e2e4 e7e5...)
        if let Some(idx) = moves_idx {
            for m_str in &parts[idx + 1..] {
                let moves = self.board.genera_mosse_legali(&self.zobrist);
                if let Some(m) = moves.iter().find(|m| m.to_uci() == *m_str) {
                    self.board.esegui_mossa(m, &self.zobrist, self.nnue.as_deref());
                }
            }
        }
    }

    fn parse_go(&mut self, parts: &[&str]) {
        let mut depth = MAX_DEPTH as i32;
        let mut movetime: u128 = 0;
        let mut wtime: u128 = 0;
        let mut btime: u128 = 0;
        let mut winc: u128 = 0;
        let mut binc: u128 = 0;

        // Parsing dei parametri dell'orologio inviati dalla GUI/Lichess
        for i in 1..parts.len() {
            match parts[i] {
                "depth" => if i+1 < parts.len() { depth = parts[i+1].parse().unwrap_or(12) },
                "movetime" => if i+1 < parts.len() { movetime = parts[i+1].parse().unwrap_or(0) },
                "wtime" => if i+1 < parts.len() { wtime = parts[i+1].parse().unwrap_or(0) },
                "btime" => if i+1 < parts.len() { btime = parts[i+1].parse().unwrap_or(0) },
                "winc" => if i+1 < parts.len() { winc = parts[i+1].parse().unwrap_or(0) },
                "binc" => if i+1 < parts.len() { binc = parts[i+1].parse().unwrap_or(0) },
                _ => {}
            }
        }

        // LOGICA DI GESTIONE TEMPO
        let time_limit = if movetime > 0 {
            movetime // Tempo fisso richiesto esplicitamente
        } else if (self.board.turno == Colore::Bianco && wtime > 0) || 
                  (self.board.turno == Colore::Nero && btime > 0) {
            
            let (my_time, my_inc) = if self.board.turno == Colore::Bianco {
                (wtime, winc)
            } else {
                (btime, binc)
            };

            // Strategia: usa 1/20 del tempo rimasto + l'incremento intero
            (my_time / 20) + my_inc
        } else {
            0 // Nessun limite di tempo (usa depth o cerca all'infinito)
        };

        // Preparazione dati per il thread di ricerca
        let mut board_copy = self.board.clone();
        let tt_arc = self.tt.clone();
        let nnue_arc = self.nnue.clone();
        let z_keys = self.zobrist.clone();

        thread::spawn(move || {
            let mut info = SearchInfo::new(time_limit, depth);
            let mut tt = tt_arc.lock().unwrap();
            
            // Avvio ricerca Alpha-Beta con Iterative Deepening
            let (best, _) = iterative_deepening(
                &mut board_copy, 
                &mut info, 
                &mut tt, 
                &z_keys, 
                nnue_arc.as_deref()
            );
            
            println!("bestmove {}", best.to_uci());
        });
    }
}