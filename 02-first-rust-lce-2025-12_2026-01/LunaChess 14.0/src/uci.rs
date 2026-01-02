use crate::board::{Scacchiera, Mossa, Colore};
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
    eval_file: String,
    use_nnue: bool,
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
            eval_file: "luna_net.nnue".to_string(),
            use_nnue: true,
        }
    }

    pub fn run(&mut self) {
        let stdin = io::stdin();
        println!("id name Luna Chess PRO v1.1");
        println!("id author Danie & Gemini");
        println!("option name Hash type spin default 64 min 1 max 2048");
        println!("option name Threads type spin default 1 min 1 max 128");
        println!("option name EvalFile type string default luna_net.nnue");
        println!("option name Use NNUE type check default true");
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
                    // tt.clear() rimosso temporaneamente per via di Arc
                },
                "position" => self.handle_position(&parts),
                "go" => self.handle_go(&parts),
                "stop" => self.stop_search.store(true, Ordering::SeqCst),
                "quit" => {
                    self.stop_search.store(true, Ordering::SeqCst);
                    break;
                },
                _ => {}
            }
            io::stdout().flush().unwrap();
        }
    }

    fn handle_setoption(&mut self, parts: &[&str]) {
        if parts.len() < 5 || parts[1] != "name" { return; }

        let mut option_name = parts[2].to_lowercase();
        if option_name == "use" && parts.len() > 3 && parts[3].to_lowercase() == "nnue" {
            option_name = "usennue".to_string();
        }

        let value_index = parts.iter().position(|&x| x == "value");
        if let Some(idx) = value_index {
            if idx + 1 >= parts.len() { return; }
            let value = parts[idx + 1];

            match option_name.as_str() {
                "hash" => {
                    if let Ok(mb) = value.parse::<usize>() {
                        if mb != self.hash_size_mb {
                            self.hash_size_mb = mb;
                            self.tt = Arc::new(TranspositionTable::new(mb));
                        }
                    }
                },
                "threads" => {
                    if let Ok(t) = value.parse::<usize>() {
                        self.num_threads = t.clamp(1, 128);
                    }
                },
                "evalfile" => self.eval_file = value.to_string(),
                "usennue" => self.use_nnue = value.parse::<bool>().unwrap_or(true),
                _ => {}
            }
        }
    }

    fn handle_position(&mut self, parts: &[&str]) {
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
                let moves = new_board.genera_mosse();
                // Annotazione esplicita del tipo l: &Mossa per risolvere l'errore E0282
                if let Some(m) = moves.iter().find(|l: &&Mossa| l.to_string() == m_str) {
                    new_board.esegui_mossa_raw(m);
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

        // Correzione: board.turno è un campo, non un metodo
        let mut tempo_di_riflessione = if movetime > 0 {
            movetime
        } else if infinite {
            u64::MAX
        } else {
            let (my_time, my_inc) = if self.board.turno == Colore::Bianco { (wtime, winc) } else { (btime, binc) };
            if my_time == 0 { 100 } else { (my_time / 30) + (my_inc * 4 / 5) }
        };

        if tempo_di_riflessione == 0 && !infinite { tempo_di_riflessione = 50; }

        let tt_shared = self.tt.clone();
        let stop_flag = self.stop_search.clone();
        let board_copy = self.board.clone();
        let num_threads = self.num_threads;
        let eval_file_path = self.eval_file.clone();
        let use_nnue_flag = self.use_nnue;

        for i in 0..num_threads {
            let mut thread_board = board_copy.clone();
            let thread_tt = tt_shared.clone();
            let thread_stop = stop_flag.clone();
            let is_main = i == 0;
            let t_eval_file = eval_file_path.clone();
            let t_use_nnue = use_nnue_flag;

            thread::spawn(move || {
                let mut motore = Motore::nuovo(thread_tt);
                motore.set_stop_flag(thread_stop.clone());
                motore.set_nnue_config(t_eval_file, t_use_nnue);
                
                let mossa = motore.trova_mossa_migliore(&mut thread_board, tempo_di_riflessione, is_main);

                if is_main {
                    thread_stop.store(true, Ordering::SeqCst);
                    match mossa {
                        Some(m) => println!("bestmove {}", m),
                        None => {
                            let mosse = thread_board.genera_mosse();
                            // Annotazione esplicita del tipo m: &Mossa
                            println!("bestmove {}", mosse.first().map_or("0000".to_string(), |m: &Mossa| m.to_string()));
                        }
                    }
                    io::stdout().flush().unwrap();
                }
            });
        }
    }
}