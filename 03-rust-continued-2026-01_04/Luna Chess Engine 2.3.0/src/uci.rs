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
    pub fn new() -> Self {
        let keys = ZobristKeys::default();
        
        // 1. Caricamento Book
        let book = OpeningBook::load("book.txt");
        if book.is_some() {
            println!("info string Book loaded successfully");
        } else {
            println!("info string Book NOT found (book.txt missing) - create it for openings");
        }

        // 2. Caricamento NNUE
        let nnue_raw = LunaNNUE::load("luna.nnue");
        let nnue = if let Some(net) = nnue_raw {
            println!("info string NNUE loaded successfully");
            Some(Arc::new(net))
        } else {
            println!("info string NNUE load FAILED or file missing -> Using Hand-Crafted Evaluation (Fallback)");
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
                println!("id name Luna 0.6 NNUE");
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
        // Controllo apertura
        if let Some(book) = &self.book {
            if let Some(book_move) = book.get_move(&self.board) {
                println!("bestmove {}", book_move.to_uci());
                return;
            }
        }

        let mut depth = 64;
        let mut time = 10000;
        
        for i in 0..parts.len() {
            if parts[i] == "depth" {
                if let Ok(d) = parts[i+1].parse() { depth = d; }
            } else if parts[i] == "wtime" && self.board.turno == Colore::Bianco {
                if let Ok(t) = parts[i+1].parse::<u128>() { time = t / 30; }
            } else if parts[i] == "btime" && self.board.turno == Colore::Nero {
                if let Ok(t) = parts[i+1].parse::<u128>() { time = t / 30; }
            } else if parts[i] == "movetime" {
                 if let Ok(t) = parts[i+1].parse::<u128>() { time = t; }
            }
        }

        let mut board_copy = self.board.clone();
        let tt_arc = self.tt.clone();
        let nnue_arc = self.nnue.clone(); // Clona il puntatore Arc (economico)
        
        thread::spawn(move || {
            let mut info = SearchInfo::new(time, depth);
            let mut tt = tt_arc.lock().unwrap();
            let z = ZobristKeys::default();
            tt.new_search();
            
            // Passiamo il riferimento alla NNUE (se esiste)
            let nnue_ref = nnue_arc.as_deref();
            
            let (best, _) = iterative_deepening(&mut board_copy, &mut info, &mut tt, &z, nnue_ref);
            println!("bestmove {}", best.to_uci());
        });
    }
}