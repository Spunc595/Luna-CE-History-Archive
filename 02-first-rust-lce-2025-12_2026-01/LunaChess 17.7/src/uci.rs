use std::io::{self, BufRead};
use std::time::Instant;
use crate::board::{Scacchiera, Mossa, Colore};
use crate::search::{self, SearchInfo};
use crate::tt::TranspositionTable;
use crate::zobrist::ZobristKeys;

pub fn uci_loop() {
    let mut board = Scacchiera::new();
    let mut tt = TranspositionTable::new(64);
    let zobrist = ZobristKeys::new();
    let stdin = io::stdin();

    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() { continue; }

        match parts[0] {
            "uci" => {
                println!("id name Luna 17.7");
                println!("id author Daniele");
                println!("uciok");
            }
            "isready" => println!("readyok"),
            "ucinewgame" => { board = Scacchiera::new(); tt.clear(); }
            "position" => {
                if parts.len() > 1 {
                    if parts[1] == "startpos" { board = Scacchiera::new(); }
                    else if parts[1] == "fen" && parts.len() >= 8 {
                        let fen_str = parts[2..8].join(" ");
                        board = Scacchiera::from_fen(&fen_str);
                    }
                }
                if let Some(idx) = parts.iter().position(|&x| x == "moves") {
                    for m_str in &parts[idx + 1..] { applica_mossa_uci(&mut board, m_str); }
                }
            }
            "go" => {
                tt.new_search();
                let mut time_available = 0;
                let mut increment = 0;
                let mut movetime = 0;
                let mut is_infinite = true;
                for i in 1..parts.len() {
                    match parts[i] {
                        "wtime" if board.turno == Colore::Bianco => { time_available = parts[i+1].parse().unwrap_or(0); is_infinite = false; }
                        "btime" if board.turno == Colore::Nero => { time_available = parts[i+1].parse().unwrap_or(0); is_infinite = false; }
                        "winc" if board.turno == Colore::Bianco => increment = parts[i+1].parse().unwrap_or(0),
                        "binc" if board.turno == Colore::Nero => increment = parts[i+1].parse().unwrap_or(0),
                        "movetime" => { movetime = parts[i+1].parse().unwrap_or(0); is_infinite = false; }
                        _ => {}
                    }
                }
                let time_limit_ms = if movetime > 0 { (movetime as f64 * 0.95) as u128 }
                else if is_infinite { u128::MAX }
                else {
                    let base = time_available as f64 / 40.0;
                    let inc_part = increment as f64 * 0.8;
                    ((base + inc_part) as u128).min((time_available / 5) as u128).max(10)
                };
                let mut info = SearchInfo { start_time: Instant::now(), time_limit_ms, nodes: 0, stop: false };
                let best_m = search::iterative_deepening(&mut board, &mut info, &mut tt, &zobrist);
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
        if format!("{}", m) == m_str {
            let mut temp_board = board.clone();
            if temp_board.esegui_mossa(&m, None) {
                board.esegui_mossa(&m, None);
                return;
            }
        }
    }
}