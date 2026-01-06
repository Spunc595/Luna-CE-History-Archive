use crate::board::{Scacchiera, Mossa, Colore, MoveFlag};
use crate::search::Motore;
use crate::transposition::TranspositionTable;
use crate::nnue::Network; // Assicurati che nnue.rs sia accessibile
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
    // La rete viene mantenuta qui per essere usata nell'aggiornamento della board
    net: Option<Network>, 
}

impl UciEngine {
    pub fn nuova() -> Self {
        let default_hash = 64;
        let eval_file = "luna_net.nnue".to_string();
        
        // Proviamo a caricare subito la rete se esiste, per evitare ritardi dopo
        let net = if std::path::Path::new(&eval_file).exists() {
            Some(Network::carica(&eval_file))
        } else {
            None
        };

        UciEngine {
            board: Scacchiera::nuova(),
            hash_size_mb: default_hash,
            num_threads: 1,
            stop_search: Arc::new(AtomicBool::new(false)),
            tt: Arc::new(TranspositionTable::new(default_hash)),
            eval_file,
            use_nnue: true,
            net,
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
                    // Importante: Inizializzare accumulatori
                    if let Some(net) = &self.net {
                        self.board.inizializza_acc_nnue(net);
                    }
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
                "evalfile" => {
                    self.eval_file = value.to_string();
                    if self.use_nnue {
                         self.net = Some(Network::carica(&self.eval_file));
                    }
                },
                "usennue" => {
                    self.use_nnue = value.parse::<bool>().unwrap_or(true);
                    if self.use_nnue && self.net.is_none() {
                        self.net = Some(Network::carica(&self.eval_file));
                    } else if !self.use_nnue {
                        self.net = None;
                    }
                },
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
                
                // Parsing Robusto: Decodifica stringa -> coordinate
                let mut found_move = None;
                
                // Tentativo 1: Matching esatto stringa (veloce)
                if let Some(m) = moves.iter().find(|m| m.to_string() == m_str) {
                    found_move = Some(*m);
                } else {
                    // Tentativo 2: Parsing manuale coordinate (g8f6 -> 62, 45)
                    // Questo risolve i bug se to_string() ha formattazioni diverse
                    if m_str.len() >= 4 {
                        let bytes = m_str.as_bytes();
                        let f1 = bytes[0] as i8 - b'a' as i8;
                        let r1 = bytes[1] as i8 - b'1' as i8;
                        let f2 = bytes[2] as i8 - b'a' as i8;
                        let r2 = bytes[3] as i8 - b'1' as i8;
                        
                        if f1 >= 0 && f1 < 8 && r1 >= 0 && r1 < 8 &&
                           f2 >= 0 && f2 < 8 && r2 >= 0 && r2 < 8 {
                            
                            let from = (r1 * 8 + f1) as usize;
                            let to = (r2 * 8 + f2) as usize;
                            
                            // Cerca mossa con questi from/to
                            for m in &moves {
                                if m.da() == from && m.a() == to {
                                    // Gestione promozione (es: a7a8q)
                                    if m_str.len() == 5 {
                                        if m.move_flag() == MoveFlag::Promotion {
                                            found_move = Some(*m); // Assumiamo Donna per ora, o raffiniamo
                                            break; 
                                        }
                                    } else {
                                        found_move = Some(*m);
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }

                match found_move {
                    Some(m) => {
                        // Passiamo None qui perché aggiorneremo tutto alla fine per sicurezza
                        new_board.esegui_mossa(&m, None);
                    },
                    None => {
                        println!("info string ERROR: Move {} not found in generated moves!", m_str);
                        // Stampa mosse legali per debug
                        print!("info string Available moves: ");
                        for mv in &moves { print!("{} ", mv); }
                        println!("");
                    }
                }
            }
        }
        
        // Aggiornamento CRITICO della NNUE dopo aver impostato la posizione
        if let Some(net) = &self.net {
            new_board.inizializza_acc_nnue(net);
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
                            println!("bestmove {}", mosse.first().map_or("0000".to_string(), |m: &Mossa| m.to_string()));
                        }
                    }
                    io::stdout().flush().unwrap();
                }
            });
        }
    }
}