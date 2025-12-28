use crate::board::{Scacchiera, Mossa, Colore};
use crate::movegen::genera_mosse_legali;
use crate::search::Motore;
use crate::transposition::TranspositionTable;
use std::io::{self, BufRead, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc};
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
        let mut stdout = io::stdout();
        println!("id name RustChess PRO");
        println!("id author danie & Gemini");
        println!("option name Hash type spin default 64 min 1 max 2048");
        println!("option name Threads type spin default 1 min 1 max 128");
        println!("uciok");
        let _ = stdout.flush();
        for line in stdin.lock().lines() {
            let line = line.unwrap_or_default();
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() { continue; }
            match parts[0] {
                "uci" => { println!("id name RustChess PRO\nid author danie & Gemini\nuciok"); }
                "isready" => println!("readyok"),
                "setoption" => self.handle_setoption(&parts),
                "ucinewgame" => { self.board = Scacchiera::nuova(); self.tt.clear(); }
                "position" => self.handle_position(&parts),
                "go" => self.handle_go(&parts),
                "stop" => self.stop_search.store(true, Ordering::Relaxed),
                "quit" => break,
                _ => {}
            }
            let _ = stdout.flush();
        }
    }

    fn handle_setoption(&mut self, parts: &[&str]) {
        if parts.len() >= 5 && parts[1] == "name" {
            match parts[2] {
                "Hash" => if let Ok(val) = parts[4].parse::<usize>() { self.hash_size_mb = val; self.tt = Arc::new(TranspositionTable::new(val)); }
                "Threads" => if let Ok(val) = parts[4].parse::<usize>() { self.num_threads = val; }
                _ => {}
            }
        }
    }

    fn handle_position(&mut self, parts: &[&str]) {
        let mut new_board = if parts.len() > 1 && parts[1] == "fen" {
            let fen_str = parts[2..].iter().take_while(|&&p| p != "moves").cloned().collect::<Vec<_>>().join(" ");
            Scacchiera::da_fen(&fen_str).unwrap_or(Scacchiera::nuova())
        } else { Scacchiera::nuova() };
        if let Some(pos) = parts.iter().position(|&x| x == "moves") {
            for &m_str in &parts[pos + 1..] {
                let legal_moves = genera_mosse_legali(&mut new_board);
                if let Some(m) = Mossa::from_uci(m_str) {
                    if let Some(legal) = legal_moves.iter().find(|&&l| l.da().indice() == m.da().indice() && l.a().indice() == m.a().indice() && l.promozione() == m.promozione()) { new_board.esegui_mossa(legal); }
                }
            }
        }
        self.board = new_board;
    }

    fn handle_go(&mut self, parts: &[&str]) {
        self.stop_search.store(false, Ordering::Relaxed);
        let mut tempo_ms = 5000;
        let mut wtime = 0; let mut btime = 0; let mut winc = 0; let mut binc = 0;
        for i in 1..parts.len() {
            match parts[i] {
                "wtime" => wtime = parts.get(i+1).unwrap_or(&"0").parse().unwrap_or(0),
                "btime" => btime = parts.get(i+1).unwrap_or(&"0").parse().unwrap_or(0),
                "winc" => winc = parts.get(i+1).unwrap_or(&"0").parse().unwrap_or(0),
                "binc" => binc = parts.get(i+1).unwrap_or(&"0").parse().unwrap_or(0),
                "movetime" => tempo_ms = parts.get(i+1).unwrap_or(&"5000").parse().unwrap_or(5000),
                _ => {}
            }
        }
        if wtime > 0 || btime > 0 {
            let (my_time, my_inc) = if self.board.turno() == Colore::Bianco { (wtime, winc) } else { (btime, binc) };
            tempo_ms = (my_time / 30) + (my_inc / 2);
        }
        let num_threads = self.num_threads;
        let tt_shared = self.tt.clone();
        let stop_flag = self.stop_search.clone();
        let board_copy = self.board.clone();
        for i in 0..num_threads {
            let mut thread_board = board_copy.clone();
            let thread_tt = tt_shared.clone();
            let thread_stop = stop_flag.clone();
            let is_main = i == 0;
            thread::spawn(move || {
                let mut motore = Motore::nuovo(thread_tt);
                motore.set_stop_flag(thread_stop);
                let mossa = motore.trova_mossa_migliore(&mut thread_board, tempo_ms as u64, is_main);
                if is_main {
                    if let Some(m) = mossa { println!("bestmove {}", m); }
                    else {
                        let mosse = genera_mosse_legali(&mut thread_board);
                        if let Some(m) = mosse.first() { println!("bestmove {}", m); }
                        else { println!("bestmove 0000"); }
                    }
                    let _ = io::stdout().flush();
                }
            });
        }
    }
}