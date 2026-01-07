use crate::board::Scacchiera;
use crate::search::Motore;
use crate::transposition::TranspositionTable;
use std::io::{self, BufRead};
use std::sync::{Arc, Mutex, atomic::AtomicBool};
use std::thread;

pub fn lancia_loop_uci() {
    let mut board = Scacchiera::nuova();
    let tt = Arc::new(Mutex::new(TranspositionTable::new(64)));
    let stop_signal = Arc::new(AtomicBool::new(false));
    let mut motore = Motore::nuovo(tt.clone());
    motore.set_stop_flag(stop_signal.clone());

    let stdin = io::stdin();
    for linea in stdin.lock().lines() {
        let l = linea.unwrap();
        let parti: Vec<&str> = l.split_whitespace().collect();
        if parti.is_empty() { continue; }

        match parti[0] {
            "uci" => {
                println!("id name Luna Chess Engine 15.8");
                println!("id author Gemini & Daniel");
                println!("option name Hash type spin default 64 min 1 max 1024");
                println!("uciok");
            }
            "isready" => println!("readyok"),
            "setoption" => {
                if parti.len() >= 5 && parti[2] == "Hash" {
                    if let Ok(mb) = parti[4].parse::<usize>() {
                        if let Ok(mut table) = tt.lock() {
                            *table = TranspositionTable::new(mb);
                        }
                    }
                }
            }
            "ucinewgame" => {
                if let Ok(mut table) = tt.lock() { table.clear(); }
                board = Scacchiera::nuova();
            }
            "position" => parse_position(&mut board, &parti),
            "go" => {
                let time_limit = parse_go_time(&board, &parti);
                if let Ok(mut table) = tt.lock() { table.increment_age(); }

                let mut board_copy = board.clone();
                let stop_signal_thread = stop_signal.clone();
                stop_signal_thread.store(false, std::sync::atomic::Ordering::Relaxed);
                
                let tt_thread = tt.clone();
                thread::spawn(move || {
                    let mut motore_thread = Motore::nuovo(tt_thread);
                    motore_thread.set_stop_flag(stop_signal_thread);
                    if let Some(mossa) = motore_thread.trova_mossa_migliore(&mut board_copy, time_limit, true) {
                        println!("bestmove {}", mossa);
                    }
                });
            }
            "stop" => stop_signal.store(true, std::sync::atomic::Ordering::Relaxed),
            "quit" => break,
            _ => {}
        }
    }
}

fn parse_position(board: &mut Scacchiera, parti: &[&str]) {
    let mut index = 1;
    if index < parti.len() && parti[index] == "startpos" {
        *board = Scacchiera::nuova();
        index += 1;
    } 
    if index < parti.len() && parti[index] == "moves" {
        for i in (index + 1)..parti.len() {
            let mossa_str = parti[i];
            let mosse = board.genera_mosse();
            if let Some(m) = mosse.into_iter().find(|m| m.to_string() == mossa_str) {
                board.esegui_mossa(&m, None);
            }
        }
    }
}

fn parse_go_time(board: &Scacchiera, parti: &[&str]) -> u64 {
    let mut time = 300000;
    let mut inc = 0;
    let is_white = board.turno == crate::board::Colore::Bianco;

    for i in 1..parti.len() {
        if (parti[i] == "wtime" && is_white) || (parti[i] == "btime" && !is_white) {
            time = parti.get(i+1).and_then(|s| s.parse().ok()).unwrap_or(time);
        }
        if (parti[i] == "winc" && is_white) || (parti[i] == "binc" && !is_white) {
            inc = parti.get(i+1).and_then(|s| s.parse().ok()).unwrap_or(0);
        }
    }
    (time / 30) + (inc / 2)
}