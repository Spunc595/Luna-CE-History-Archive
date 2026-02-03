use std::io::{self, BufRead};
use crate::board::{Scacchiera, Colore};
use crate::search::{SearchInfo, iterative_deepening};
use crate::tt::TranspositionTable;
use crate::zobrist::ZobristKeys;
use crate::nnue::LunaNNUE;
use crate::book::OpeningBook;
use std::thread;
use std::sync::{Arc, Mutex};

pub struct UCI {
    board: Scacchiera,
    tt: Arc<Mutex<TranspositionTable>>,
    book: Option<OpeningBook>,
    // Usiamo Arc per condividere la rete neurale (pesante) tra i thread senza clonarla
    nnue: Option<Arc<LunaNNUE>>,
}

impl UCI {
    /// Crea una nuova istanza UCI.
    /// Riceve la rete NNUE già caricata dal main (se presente).
    pub fn new(nnue_option: Option<LunaNNUE>) -> Self {
        let keys = ZobristKeys::default();
        
        // 1. Caricamento Book (opzionale)
        let book = OpeningBook::load("book.txt");
        if book.is_some() {
            println!("info string Book loaded successfully");
        }

        // 2. Gestione NNUE
        let nnue = if let Some(net) = nnue_option {
            Some(Arc::new(net))
        } else {
            None
        };

        UCI {
            board: Scacchiera::new_iniziale(&keys),
            tt: Arc::new(Mutex::new(TranspositionTable::new(64))),
            book,
            nnue,
        }
    }

    pub fn run_loop(&mut self) {
        let stdin = io::stdin();
        let mut handle = stdin.lock();
        let mut buffer = String::new();

        loop {
            buffer.clear();
            if handle.read_line(&mut buffer).unwrap_or(0) == 0 { break; }
            let command = buffer.trim();
            if command == "quit" { break; }
            self.process_command(command);
        }
    }

    fn process_command(&mut self, cmd: &str) {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.is_empty() { return; }

        match parts[0] {
            "uci" => {
                println!("id name Luna 0.6 NNUE HalfKA");
                println!("id author Daniele & Alessandro");
                println!("option name Hash type spin default 64 min 1 max 65536");
                println!("uciok");
            },
            "isready" => println!("readyok"),
            "ucinewgame" => {
                let mut tt = self.tt.lock().unwrap();
                tt.clear();
            },
            "position" => self.parse_position(&parts),
            "go" => self.parse_go(&parts),
            _ => {}
        }
    }

    fn parse_position(&mut self, parts: &[&str]) {
        let zobrist = ZobristKeys::default();
        let mut fen_idx = 0;
        
        if parts.len() > 1 && parts[1] == "startpos" {
            self.board = Scacchiera::new_iniziale(&zobrist);
            fen_idx = 2;
        } else if parts.len() > 1 && parts[1] == "fen" {
            let mut fen = String::new();
            let mut i = 2;
            while i < parts.len() && parts[i] != "moves" {
                fen.push_str(parts[i]);
                fen.push(' ');
                i += 1;
            }
            self.board = Scacchiera::from_fen(&fen, &zobrist);
            fen_idx = i;
        }

        if fen_idx < parts.len() && parts[fen_idx] == "moves" {
            for m_str in &parts[fen_idx+1..] {
                let moves = self.board.genera_mosse_legali(&zobrist);
                if let Some(m) = moves.iter().find(|m| m.to_uci() == *m_str) {
                    self.board.esegui_mossa(m, &zobrist);
                }
            }
        }
    }

    fn parse_go(&mut self, parts: &[&str]) {
        // Controllo apertura dal Book
        if let Some(book) = &self.book {
            if let Some(book_move) = book.get_move(&self.board) {
                println!("bestmove {}", book_move.to_uci());
                return;
            }
        }

        let mut depth = 64;
        let mut wtime: i64 = -1;
        let mut btime: i64 = -1;
        let mut winc: i64 = 0;
        let mut binc: i64 = 0;
        let mut movetime: i64 = -1;
        
        // Parsing parametri UCI sicuro
        for i in 0..parts.len() {
            if parts[i] == "depth" && i + 1 < parts.len() {
                if let Ok(d) = parts[i+1].parse() { depth = d; }
            } else if parts[i] == "wtime" && i + 1 < parts.len() {
                if let Ok(t) = parts[i+1].parse() { wtime = t; }
            } else if parts[i] == "btime" && i + 1 < parts.len() {
                if let Ok(t) = parts[i+1].parse() { btime = t; }
            } else if parts[i] == "winc" && i + 1 < parts.len() {
                if let Ok(t) = parts[i+1].parse() { winc = t; }
            } else if parts[i] == "binc" && i + 1 < parts.len() {
                if let Ok(t) = parts[i+1].parse() { binc = t; }
            } else if parts[i] == "movetime" && i + 1 < parts.len() {
                 if let Ok(t) = parts[i+1].parse() { movetime = t; }
            }
        }

        let mut time: u128 = 10000; // default 10 secondi
        let mut is_timed_game = false;

        // --- NUOVO TIME MANAGEMENT DINAMICO ---
        let total_time = if self.board.turno == Colore::Bianco { wtime } else { btime };
        let inc = if self.board.turno == Colore::Bianco { winc } else { binc };

        if movetime > 0 {
            time = movetime as u128;
            is_timed_game = true;
        } else if total_time > 0 {
            // Se abbiamo più di 30 secondi (Rapid/Blitz), usiamo un divisore coraggioso (18)
            // Se siamo nel Bullet profondo o sotto i 30 secondi, restiamo prudenti (40)
            let divisor = if total_time > 30000 { 18 } else { 40 };
            
            // Calcolo: (Tempo totale / Divisore) + 90% dell'incremento
            time = (total_time as u128 / divisor) + (inc as u128 * 9 / 10);
            is_timed_game = true;
        }

        // Forza la profondità massima nelle partite a tempo (ignora blocchi esterni)
        if is_timed_game {
            depth = 64;
        }

        // Preparazione dati per il thread
        let mut board_copy = self.board.clone();
        let tt_arc = self.tt.clone();
        let nnue_arc = self.nnue.clone(); 
        
        thread::spawn(move || {
            let mut info = SearchInfo::new(time, depth);
            let mut tt = tt_arc.lock().unwrap();
            let z = ZobristKeys::default();
            tt.new_search();
            
            let nnue_ref = nnue_arc.as_deref();
            let (best, _) = iterative_deepening(&mut board_copy, &mut info, &mut tt, &z, nnue_ref);
            println!("bestmove {}", best.to_uci());
        });
    }
}