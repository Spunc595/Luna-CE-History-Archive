use crate::board::{Scacchiera, Mossa, Colore};
use crate::movegen::genera_mosse_legali;
use crate::search::Motore;
use crate::transposition::TranspositionTable;
use std::io::{self, BufRead, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

pub struct UciEngine { board: Scacchiera, hash: usize, threads: usize, stop: Arc<AtomicBool>, tt: Arc<TranspositionTable> }
impl UciEngine {
    pub fn nuova() -> Self {
        let h = 64; let tt = Arc::new(TranspositionTable::new(h));
        UciEngine { board: Scacchiera::nuova(), hash: h, threads: 1, stop: Arc::new(AtomicBool::new(false)), tt }
    }
    pub fn run(&mut self) {
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            let l = line.unwrap(); let p: Vec<&str> = l.split_whitespace().collect();
            if p.is_empty() { continue; }
            match p[0] {
                "uci" => { println!("id name Luna PRO\nid author danie\nuciok"); }
                "isready" => println!("readyok"),
                "setoption" => self.handle_opt(&p),
                "position" => self.handle_pos(&p),
                "go" => self.handle_go(&p),
                "stop" => self.stop.store(true, Ordering::SeqCst),
                "quit" => break,
                _ => {}
            }
            let _ = io::stdout().flush();
        }
    }
    fn handle_opt(&mut self, p: &[&str]) {
        if p.len() >= 5 && p[2] == "Hash" { self.hash = p[4].parse().unwrap(); self.tt = Arc::new(TranspositionTable::new(self.hash)); }
        if p.len() >= 5 && p[2] == "Threads" { self.threads = p[4].parse().unwrap(); }
    }
    fn handle_pos(&mut self, p: &[&str]) {
        let mut b = if p.len() > 1 && p[1] == "fen" { Scacchiera::da_fen(&p[2..6].join(" ")).unwrap() } else { Scacchiera::nuova() };
        if let Some(idx) = p.iter().position(|&x| x == "moves") {
            for &m_str in &p[idx+1..] {
                let legali = genera_mosse_legali(&mut b);
                let m = Mossa::from_uci(m_str).unwrap();
                if let Some(l) = legali.into_iter().find(|&x| x.da().indice() == m.da().indice() && x.a().indice() == m.a().indice()) { b.esegui_mossa(&l); }
            }
        }
        self.board = b;
    }
    fn handle_go(&mut self, p: &[&str]) {
        self.stop.store(true, Ordering::SeqCst); // Ferma vecchi thread
        thread::sleep(std::time::Duration::from_millis(5));
        self.stop.store(false, Ordering::SeqCst);
        let mut t = 5000;
        for i in 0..p.len() {
            if p[i] == "wtime" && self.board.turno() == Colore::Bianco { t = p[i+1].parse::<u64>().unwrap() / 30; }
            if p[i] == "btime" && self.board.turno() == Colore::Nero { t = p[i+1].parse::<u64>().unwrap() / 30; }
            if p[i] == "movetime" { t = p[i+1].parse().unwrap(); }
        }
        for i in 0..self.threads {
            let mut bc = self.board.clone(); let ttc = self.tt.clone(); let stc = self.stop.clone(); let is_m = i == 0;
            thread::spawn(move || {
                let mut m = Motore::nuovo(ttc); m.set_stop_flag(stc);
                let best = m.trova_mossa_migliore(&mut bc, t, is_m);
                if is_m { if let Some(bm) = best { println!("bestmove {}", bm); } else { println!("bestmove 0000"); } let _ = io::stdout().flush(); }
            });
        }
    }
}