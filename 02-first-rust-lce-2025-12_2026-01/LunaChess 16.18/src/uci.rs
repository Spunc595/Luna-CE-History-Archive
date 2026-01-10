use std::io::{self, BufRead};
use std::time::Instant;
use crate::board::{Scacchiera, Mossa, Colore};
use crate::search::{self, SearchInfo};

pub fn uci_loop() {
    let mut board = Scacchiera::nuova();
    let stdin = io::stdin();

    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() { continue; }

        match parts[0] {
            "uci" => println!("id name Luna 16.20\nid author Daniele & Gemini\nuciok"),
            "isready" => println!("readyok"),
            "ucinewgame" => board = Scacchiera::nuova(),
            "position" => {
                if parts.len() > 1 && parts[1] == "startpos" {
                    board = Scacchiera::nuova();
                    if parts.len() > 2 && parts[2] == "moves" {
                        for m_str in &parts[3..] {
                            applica_mossa_uci(&mut board, m_str);
                        }
                    }
                }
            }
            "go" => {
                let mut time = 5000;
                for i in 1..parts.len() {
                    if (parts[i] == "wtime" && board.turno == Colore::Bianco) || 
                       (parts[i] == "btime" && board.turno == Colore::Nero) {
                        if let Some(t) = parts.get(i+1).and_then(|s| s.parse::<u64>().ok()) {
                            time = (t / 30).max(50);
                        }
                    }
                }

                let mut info = SearchInfo {
                    start_time: Instant::now(),
                    time_limit_ms: time as u128,
                    nodes: 0,
                    stop: false,
                };

                let mut best_m = Mossa { data: 0 };
                for d in 1..=12 {
                    let (score, m) = search::search_root(&mut board, d, &mut info);
                    if info.stop && d > 1 { break; }
                    best_m = m;
                    println!("info depth {} score cp {} nodes {}", d, score, info.nodes);
                }
                println!("bestmove {}", best_m);
            }
            "quit" => break,
            _ => {}
        }
    }
}

fn applica_mossa_uci(board: &mut Scacchiera, m_str: &str) {
    let moves = board.genera_mosse();
    for m in moves {
        if m.to_string() == m_str {
            board.esegui_mossa(&m, None);
            return;
        }
    }
}