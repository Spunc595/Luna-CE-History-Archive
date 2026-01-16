use std::io::{self, BufRead, Write};
use std::fs::OpenOptions;
use crate::board::{Scacchiera, Mossa, Colore};
use crate::search::{iterative_deepening, SearchInfo};
use crate::tt::TranspositionTable;
use crate::zobrist::ZobristKeys;
use crate::nnue::LunaNNUE;
use std::time::Instant;

pub fn uci_loop() {
    let stdin = io::stdin();
    let z = ZobristKeys::init();
    
    let mut board = Scacchiera::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", &z);
    let mut tt = TranspositionTable::new(64); 
    let mut history_hashes: Vec<u64> = Vec::new();

    let nnue_path = "luna_brain.nnue";
    let nnue = LunaNNUE::load(nnue_path);
    let nnue_ref = nnue.as_ref();

    // PERCORSO ASSOLUTO DEL DATASET
    let data_path = r"C:\Users\danie\Desktop\Luna_Chess_Engine\Luna_AI_Lab\luna_dataset.bin";

    for line in stdin.lock().lines() {
        let input = line.unwrap_or_default();
        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.is_empty() { continue; }

        match parts[0] {
            "uci" => {
                println!("id name Luna 2.5-Lichess");
                println!("id author Daniele & Gemini");
                println!("option name Hash type spin default 64 min 1 max 2048");
                println!("uciok");
            }
            "setoption" => {
                if let Some(val_idx) = parts.iter().position(|&r| r == "value") {
                    if let Ok(mb) = parts[val_idx + 1].parse::<usize>() {
                        tt = TranspositionTable::new(mb);
                    }
                }
            }
            "isready" => println!("readyok"),
            "ucinewgame" => {
                tt.clear();
                history_hashes.clear();
                board = Scacchiera::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", &z);
            }
            "position" => {
                if parts.len() < 2 { continue; }
                if parts[1] == "startpos" {
                    board = Scacchiera::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", &z);
                } else if parts[1] == "fen" {
                    let fen_str = parts[2..8.min(parts.len())].join(" ");
                    board = Scacchiera::from_fen(&fen_str, &z);
                }
                
                history_hashes.clear();
                if let Some(pos) = parts.iter().position(|&r| r == "moves") {
                    for m_str in &parts[pos + 1..] {
                        let moves = board.genera_mosse();
                        if let Some(m) = moves.iter().find(|m| m.to_uci() == *m_str) {
                            history_hashes.push(board.hash);
                            board.esegui_mossa(m, &z);
                        }
                    }
                }
            }
            "go" => {
                let mut time_available: u128 = 3000;
                let mut increment: u128 = 0;
                let mut depth_limit = 0;
                let mut move_time = 0;

                for i in 1..parts.len() {
                    match parts[i] {
                        "wtime" if board.turno == Colore::Bianco => { time_available = parts[i+1].parse().unwrap_or(3000); }
                        "btime" if board.turno == Colore::Nero => { time_available = parts[i+1].parse().unwrap_or(3000); }
                        "winc" if board.turno == Colore::Bianco => { increment = parts[i+1].parse().unwrap_or(0); }
                        "binc" if board.turno == Colore::Nero => { increment = parts[i+1].parse().unwrap_or(0); }
                        "movetime" => { move_time = parts[i+1].parse().unwrap_or(0); }
                        "depth" => { depth_limit = parts[i+1].parse().unwrap_or(0); }
                        _ => {}
                    }
                }

                let time_limit = if move_time > 0 { move_time } else { (time_available / 20) + (increment / 2) };
                let safe_time = if time_limit > 50 { time_limit - 50 } else { time_limit };

                let mut info = SearchInfo {
                    start_time: Instant::now(),
                    time_limit_ms: safe_time,
                    nodes: 0,
                    stop: false,
                    history: history_hashes.clone(),
                    killer_moves: [[Mossa { data: 0 }; 2]; 64],
                    history_scores: [[0; 64]; 64],
                    depth_limit: depth_limit as i32,
                };

                let (best_move, best_score) = iterative_deepening(&mut board, &mut info, &mut tt, &z, nnue_ref);

                // --- SCRITTURA NEL FILE ESISTENTE ---
                if info.nodes > 2000 && best_move.data != 0 {
                    if let Ok(mut file) = OpenOptions::new().append(true).create(true).open(data_path) {
                        let _ = writeln!(file, "{} | score: {} | bestmove: {}", board.get_fen(), best_score, best_move.to_uci());
                    }
                }

                println!("bestmove {}", best_move.to_uci());
            }
            "quit" => break,
            _ => {}
        }
    }
}