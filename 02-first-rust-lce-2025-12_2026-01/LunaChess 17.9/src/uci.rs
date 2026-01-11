use std::io::{self, BufRead};
use crate::board::Scacchiera;
use crate::search::{iterative_deepening, SearchInfo};
use crate::tt::TranspositionTable;
use crate::zobrist::ZobristKeys;
use std::time::Instant;

pub fn uci_loop() {
    let stdin = io::stdin();
    let z = ZobristKeys::init();
    let mut board = Scacchiera::new();
    let mut tt = TranspositionTable::new(64);
    let mut history: Vec<u64> = Vec::new();

    for line in stdin.lock().lines() {
        let input = line.unwrap();
        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.is_empty() { continue; }

        match parts[0] {
            "uci" => {
                println!("id name Luna 18.0");
                println!("id author Daniele & Gemini");
                println!("uciok");
            }
            "isready" => println!("readyok"),
            "ucinewgame" => {
                board = Scacchiera::new();
                tt.clear();
                history.clear();
            }
            "position" => {
                if parts[1] == "startpos" { board = Scacchiera::new(); }
                history.clear();
                if let Some(pos) = parts.iter().position(|&r| r == "moves") {
                    for m_str in &parts[pos + 1..] {
                        let moves = board.genera_mosse();
                        for m in moves {
                            if m.to_string() == *m_str {
                                history.push(board.get_hash(&z));
                                board.esegui_mossa(&m, &z); // CORRETTO: rimosso Some()
                                break;
                            }
                        }
                    }
                }
            }
            "go" => {
                let mut time_ms: u128 = 5000;
                for i in 1..parts.len() {
                    match parts[i] {
                        "wtime" if board.turno == crate::board::Colore::Bianco => {
                            time_ms = parts[i+1].parse::<u128>().unwrap_or(5000) / 25;
                        }
                        "btime" if board.turno == crate::board::Colore::Nero => {
                            time_ms = parts[i+1].parse::<u128>().unwrap_or(5000) / 25;
                        }
                        "movetime" => time_ms = parts[i+1].parse::<u128>().unwrap_or(5000), // CORRETTO: messo virgola
                        _ => {}
                    }
                }

                let mut info = SearchInfo {
                    start_time: Instant::now(),
                    time_limit_ms: time_ms,
                    nodes: 0,
                    stop: false,
                    history: history.clone(),
                };

                tt.new_search();
                let best_move = iterative_deepening(&mut board, &mut info, &mut tt, &z);
                println!("bestmove {}", best_move);
            }
            "quit" => break,
            _ => {}
        }
    }
}