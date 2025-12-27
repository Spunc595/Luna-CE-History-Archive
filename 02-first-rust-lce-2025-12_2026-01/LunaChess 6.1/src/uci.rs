// src/uci.rs

use crate::board::{Scacchiera, Mossa, Colore};
use crate::movegen::genera_mosse_legali;
use crate::search::Motore; // Assicurati che in search.rs ci sia "pub struct Motore"
use std::io::{self, BufRead, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

pub struct UciEngine {
    board: Scacchiera,
    hash_size: usize, 
    is_running: Arc<AtomicBool>,
    stop_search: Arc<AtomicBool>,
    debug_mode: bool,
}

impl UciEngine {
    pub fn nuova() -> Self {
        UciEngine {
            board: Scacchiera::nuova(),
            hash_size: 64, // Default 64MB Transposition Table
            is_running: Arc::new(AtomicBool::new(true)),
            stop_search: Arc::new(AtomicBool::new(false)),
            debug_mode: false,
        }
    }

    pub fn run(&mut self) {
        let stdin = io::stdin();
        let mut stdout = io::stdout();
        println!("id name RustChess Engine");
        println!("id author RustChess Team");
        println!("uciok");
        let _ = stdout.flush();
        
        // Loop principale di lettura comandi
        for line in stdin.lock().lines() {
            let line = line.unwrap_or_default().trim().to_string();
            if line.is_empty() { continue; }
            
            // Gestione uscita
            if !self.is_running.load(Ordering::Relaxed) { break; }
            
            self.process_command(&line);
            let _ = stdout.flush();
        }
    }

    fn process_command(&mut self, command: &str) {
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() { return; }

        match parts[0] {
            "uci" => {
                println!("id name RustChess Engine");
                println!("id author RustChess Team");
                println!("option name Hash type spin default 64 min 1 max 1024");
                println!("uciok");
            },
            "isready" => println!("readyok"),
            "setoption" => self.handle_setoption(&parts),
            "ucinewgame" => { 
                self.board = Scacchiera::nuova(); 
                self.stop_search.store(false, Ordering::Relaxed);
            },
            "position" => self.handle_position(&parts),
            "go" => self.handle_go(&parts),
            "stop" => { 
                self.stop_search.store(true, Ordering::Relaxed); 
            },
            "quit" => { 
                self.stop_search.store(true, Ordering::Relaxed); 
                self.is_running.store(false, Ordering::Relaxed); 
            },
            "print" | "d" => println!("{}", self.board),
            "perft" => self.handle_perft(&parts),
            _ => {} // Ignora comandi sconosciuti
        }
    }

    fn handle_setoption(&mut self, parts: &[&str]) {
        // Esempio: setoption name Hash value 128
        if parts.len() >= 5 && parts[1] == "name" && parts[2] == "Hash" && parts[3] == "value" {
            if let Ok(val) = parts[4].parse::<usize>() {
                self.hash_size = val;
            }
        }
    }

    fn handle_position(&mut self, parts: &[&str]) {
        // Formato: position startpos [moves e2e4 ...]
        // Formato: position fen <fen> [moves ...]
        
        let mut new_board = if parts.len() > 1 && parts[1] == "fen" {
            // Ricostruisce la stringa FEN (può contenere spazi)
            let mut fen_parts = Vec::new();
            let mut move_idx = None;
            
            for (i, &part) in parts.iter().enumerate().skip(2) {
                if part == "moves" {
                    move_idx = Some(i);
                    break;
                }
                fen_parts.push(part);
            }
            
            let fen = fen_parts.join(" ");
            Scacchiera::da_fen(&fen).unwrap_or(Scacchiera::nuova())
        } else {
            Scacchiera::nuova() // startpos
        };

        // Applica le mosse se presenti
        if let Some(pos) = parts.iter().position(|&x| x == "moves") {
            for s in &parts[pos+1..] {
                // Tenta di parsare la mossa UCI
                if let Some(m) = Mossa::from_uci(s) {
                    // Nota: Mossa::from_uci non ha i flag corretti (es. non sa se è cattura).
                    // Dobbiamo trovare la mossa legale corrispondente generata dal motore
                    // per avere i flag corretti (fondamentale per board.esegui_mossa).
                    
                    let legal_moves = genera_mosse_legali(&mut new_board);
                    let mut found = false;
                    for legal in legal_moves {
                        // Confronta solo sorgente, destinazione e promozione
                        if legal.da() == m.da() && legal.a() == m.a() && legal.promozione() == m.promozione() {
                            new_board.esegui_mossa(&legal);
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        // Fallback se qualcosa non va (es. mossa non legale per il motore ma inviata da GUI)
                        // Proviamo comunque ad eseguirla raw, ma è rischioso
                        new_board.esegui_mossa(&m);
                    }
                }
            }
        }
        self.board = new_board;
    }

    fn handle_go(&mut self, parts: &[&str]) {
        // Reset stop flag
        self.stop_search.store(false, Ordering::Relaxed);
        
        let mut wtime: Option<u64> = None;
        let mut btime: Option<u64> = None;
        let mut winc: u64 = 0;
        let mut binc: u64 = 0;
        let mut movetime: Option<u64> = None;
        let mut infinite = false;

        let mut i = 1;
        while i < parts.len() {
            match parts[i] {
                "wtime" => { wtime = parts.get(i+1).and_then(|s| s.parse().ok()); i+=1; }
                "btime" => { btime = parts.get(i+1).and_then(|s| s.parse().ok()); i+=1; }
                "winc" => { winc = parts.get(i+1).and_then(|s| s.parse().ok()).unwrap_or(0); i+=1; }
                "binc" => { binc = parts.get(i+1).and_then(|s| s.parse().ok()).unwrap_or(0); i+=1; }
                "movetime" => { movetime = parts.get(i+1).and_then(|s| s.parse().ok()); i+=1; }
                "infinite" => { infinite = true; }
                _ => {}
            }
            i += 1;
        }

        // Calcolo tempo allocato
        let time_ms: u64 = if infinite {
            u64::MAX // Tempo "infinito"
        } else if let Some(mt) = movetime {
            mt
        } else {
            // Gestione tempo standard (Fischer o Sudden Death)
            let (t, inc) = match self.board.turno {
                Colore::Bianco => (wtime, winc),
                Colore::Nero => (btime, binc),
            };
            if let Some(ms) = t {
                // Strategia semplice: usa 1/30 del tempo rimanente + incremento
                let alloc: u64 = (ms / 30) + inc;
                // Assicurati di avere almeno un minimo per non andare in timeout immediato
                alloc.max(50).min(ms.saturating_sub(50)) 
            } else {
                1000 // Default 1 secondo se non specificato
            }
        };

        // Prepara i dati per il thread
        let board_clone = self.board.clone();
        let hash_size = self.hash_size;
        let stop_flag = self.stop_search.clone(); // Clona il puntatore all'atomic bool
        
        // Lancia il thread di ricerca
        thread::spawn(move || {
            let mut engine = Motore::nuovo(hash_size);
            
            // Collega il flag di stop al motore
            engine.set_stop_flag(stop_flag);
            
            // Copia mutabile locale per la ricerca
            let mut search_board = board_clone; 
            
            let best = engine.trova_mossa_migliore(&mut search_board, time_ms);
            
            // Invia la mossa alla GUI
            if let Some(m) = best {
                println!("bestmove {}", m);
            } else {
                println!("bestmove 0000"); // Null move in caso di stallo/errore
            }
        });
    }

    fn handle_perft(&self, parts: &[&str]) {
        let depth = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
        println!("Perft depth: {}", depth);
        
        let start = Instant::now();
        let mut board_copy = self.board.clone();
        let nodes = self.run_perft_recursive(&mut board_copy, depth);
        let elapsed = start.elapsed();
        
        println!("Nodes: {}", nodes);
        println!("Time: {:?}", elapsed);
        println!("NPS: {:.0}", nodes as f64 / elapsed.as_secs_f64());
    }

    fn run_perft_recursive(&self, board: &mut Scacchiera, depth: u32) -> u64 {
        if depth == 0 { return 1; }
        
        // Usa generazione legale con clone (più sicura per debug)
        let mosse = genera_mosse_legali(board);
        
        if depth == 1 { 
            return mosse.len() as u64; 
        }
        
        let mut nodes = 0;
        for m in mosse {
            // Usa clone strategy anche qui per consistenza con movegen
            let mut child = board.clone();
            if child.esegui_mossa(&m) {
                nodes += self.run_perft_recursive(&mut child, depth - 1);
            }
        }
        nodes
    }
}