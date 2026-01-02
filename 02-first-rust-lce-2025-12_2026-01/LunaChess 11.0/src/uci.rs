use crate::board::{Scacchiera, Mossa, Colore};
use crate::movegen::genera_mosse_legali;
use crate::search::Motore;
use crate::transposition::TranspositionTable;
use std::io::{self, BufRead, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

pub struct UciEngine {
    board: Scacchiera,
    hash_size_mb: usize,
    num_threads: usize,
    stop_search: Arc<AtomicBool>,
    tt: Arc<TranspositionTable>,
}

impl UciEngine {
    pub fn nuova() -> Self {
        let default_hash = 64;
        UciEngine {
            board: Scacchiera::nuova(),
            hash_size_mb: default_hash,
            num_threads: 1,
            stop_search: Arc::new(AtomicBool::new(false)),
            tt: Arc::new(TranspositionTable::new(default_hash)),
        }
    }

    pub fn run(&mut self) {
        let stdin = io::stdin();
        
        // Messaggio iniziale UCI obbligatorio
        println!("id name RustChess PRO v1.1");
        println!("id author danie & Gemini");
        println!("option name Hash type spin default 64 min 1 max 2048");
        println!("option name Threads type spin default 1 min 1 max 128");
        println!("uciok");
        io::stdout().flush().unwrap();

        for line in stdin.lock().lines() {
            let line = line.unwrap_or_default();
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() { continue; }

            match parts[0] {
                "uci" => println!("uciok"),
                "isready" => println!("readyok"),
                "setoption" => self.handle_setoption(&parts),
                "ucinewgame" => {
                    self.board = Scacchiera::nuova();
                    self.tt.clear();
                }
                "position" => self.handle_position(&parts),
                "go" => self.handle_go(&parts),
                "stop" => self.stop_search.store(true, Ordering::SeqCst),
                "quit" => {
                    self.stop_search.store(true, Ordering::SeqCst);
                    break;
                }
                _ => {}
            }
            io::stdout().flush().unwrap();
        }
    }

    fn handle_setoption(&mut self, parts: &[&str]) {
        // Formato: setoption name Hash value 128
        if parts.len() >= 5 && parts[1] == "name" {
            let name = parts[2].to_lowercase();
            let value = parts[4];

            match name.as_str() {
                "hash" => {
                    if let Ok(mb) = value.parse::<usize>() {
                        if mb != self.hash_size_mb {
                            self.hash_size_mb = mb;
                            self.tt = Arc::new(TranspositionTable::new(mb));
                        }
                    }
                }
                "threads" => {
                    if let Ok(t) = value.parse::<usize>() {
                        self.num_threads = t.clamp(1, 128);
                    }
                }
                _ => {}
            }
        }
    }

    fn handle_position(&mut self, parts: &[&str]) {
        // position [startpos | fen <fenstring>] moves <move1> ... <movei>
        let mut new_board = if parts.len() > 1 && parts[1] == "startpos" {
            Scacchiera::nuova()
        } else if parts.len() > 2 && parts[1] == "fen" {
            let fen_parts: Vec<&str> = parts[2..].iter()
                .take_while(|&&p| p != "moves")
                .cloned().collect();
            Scacchiera::da_fen(&fen_parts.join(" ")).unwrap_or(Scacchiera::nuova())
        } else {
            Scacchiera::nuova()
        };

        if let Some(pos) = parts.iter().position(|&x| x == "moves") {
            for &m_str in &parts[pos + 1..] {
                // Parsing robusto della mossa UCI
                let legal_moves = genera_mosse_legali(&mut new_board);
                if let Some(m) = legal_moves.iter().find(|l| l.to_string() == m_str) {
                    new_board.esegui_mossa(m);
                }
            }
        }
        self.board = new_board;
    }

    fn handle_go(&mut self, parts: &[&str]) {
        self.stop_search.store(false, Ordering::SeqCst);
        
        let mut wtime = 0; let mut btime = 0;
        let mut winc = 0; let mut binc = 0;
        let mut movetime = 0;
        let mut infinite = false;

        for i in 1..parts.len() {
            match parts[i] {
                "wtime" => wtime = parts.get(i+1).and_then(|v| v.parse().ok()).unwrap_or(0),
                "btime" => btime = parts.get(i+1).and_then(|v| v.parse().ok()).unwrap_or(0),
                "winc" => winc = parts.get(i+1).and_then(|v| v.parse().ok()).unwrap_or(0),
                "binc" => binc = parts.get(i+1).and_then(|v| v.parse().ok()).unwrap_or(0),
                "movetime" => movetime = parts.get(i+1).and_then(|v| v.parse().ok()).unwrap_or(0),
                "infinite" => infinite = true,
                _ => {}
            }
        }

        // Calcolo intelligente del tempo (Time Management)
        let mut tempo_di_riflessione = if movetime > 0 {
            movetime
        } else if infinite {
            u64::MAX // Cerca finché non riceve lo 'stop'
        } else {
            let (my_time, my_inc) = if self.board.turno() == Colore::Bianco { (wtime, winc) } else { (btime, binc) };
            // Formula standard: 1/30 del tempo rimanente + quasi tutto l'incremento
            (my_time / 30) + (my_inc * 4 / 5)
        };

        // Assicuriamoci di non sforare mai in tempo zero
        if tempo_di_riflessione == 0 && !infinite { tempo_di_riflessione = 100; }

        let tt_shared = self.tt.clone();
        let stop_flag = self.stop_search.clone();
        let board_copy = self.board.clone();
        let num_threads = self.num_threads;

        // Parallel Search (Lazy SMP Style)
        for i in 0..num_threads {
            let mut thread_board = board_copy.clone();
            let thread_tt = tt_shared.clone();
            let thread_stop = stop_flag.clone();
            let is_main = i == 0;

            thread::spawn(move || {
                let mut motore = Motore::nuovo(thread_tt);
                motore.set_stop_flag(thread_stop.clone());
                
                let mossa = motore.trova_mossa_migliore(&mut thread_board, tempo_di_riflessione, is_main);

                if is_main {
                    // Solo il thread principale stampa la bestmove
                    thread_stop.store(true, Ordering::SeqCst); // Ferma gli altri thread
                    match mossa {
                        Some(m) => println!("bestmove {}", m),
                        None => {
                            let mosse = genera_mosse_legali(&mut thread_board);
                            println!("bestmove {}", mosse.first().map_or("0000".to_string(), |m| m.to_string()));
                        }
                    }
                    io::stdout().flush().unwrap();
                }
            });
        }
    }
}