use std::io::{self, BufRead, Write};
use std::fs::OpenOptions;
use crate::board::{Scacchiera, Mossa};
use crate::search::{iterative_deepening, SearchInfo};
use crate::tt::TranspositionTable;
use crate::zobrist::ZobristKeys;
use std::time::Instant;

pub fn uci_loop() {
    let stdin = io::stdin();
    let z = ZobristKeys::init();
    
    let mut board = Scacchiera::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", &z);
    let mut tt = TranspositionTable::new(64);
    let mut history_hashes: Vec<u64> = Vec::new();
    let mut syzygy_enabled = false;

    // Percorso centralizzato per il dataset NNUE (modificabile via UCI in futuro)
    let data_path = r"C:\Users\danie\Desktop\Luna_Chess_Engine\Luna_AI_Lab\luna_dataset.bin";

    for line in stdin.lock().lines() {
        let input = line.unwrap_or_default();
        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.is_empty() { continue; }

        match parts[0] {
            "uci" => {
                println!("id name Luna 18.3-Pro");
                println!("id author Daniele & Gemini");
                println!("option name SyzygyPath type string default <empty>");
                println!("uciok");
            }
            "setoption" => {
                if parts.contains(&"SyzygyPath") {
                    syzygy_enabled = true; 
                    eprintln!("info string Syzygy Path set, tablebases enabled");
                }
            }
            "isready" => println!("readyok"),
            "ucinewgame" => {
                board = Scacchiera::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", &z);
                tt.clear();
                history_hashes.clear();
            }
            "position" => {
                if parts[1] == "startpos" {
                    board = Scacchiera::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", &z);
                } else if parts[1] == "fen" {
                    let fen_str = parts[2..8].join(" ");
                    board = Scacchiera::from_fen(&fen_str, &z);
                }
                history_hashes.clear();
                if let Some(pos) = parts.iter().position(|&r| r == "moves") {
                    for m_str in &parts[pos + 1..] {
                        let moves = board.genera_mosse();
                        for m in moves {
                            if m.to_uci() == *m_str {
                                history_hashes.push(board.hash);
                                board.esegui_mossa(&m, &z);
                                break;
                            }
                        }
                    }
                }
            }
            "go" => {
                let mut time_ms: u128 = 3000;
                for i in 1..parts.len() {
                    match parts[i] {
                        "wtime" | "btime" => if let Ok(t) = parts[i+1].parse::<u128>() { time_ms = t / 25; }
                        "movetime" => if let Ok(t) = parts[i+1].parse::<u128>() { time_ms = t; }
                        _ => {}
                    }
                }

                let mut info = SearchInfo {
                    start_time: Instant::now(),
                    time_limit_ms: time_ms,
                    nodes: 0,
                    stop: false,
                    history: history_hashes.clone(),
                    killer_moves: [[Mossa { data: 0 }; 2]; 64],
                    history_scores: [[0; 64]; 64],
                    use_syzygy: syzygy_enabled,
                };

                tt.new_search();
                let best_move = iterative_deepening(&mut board, &mut info, &mut tt, &z);

                // --- DATA GENERATION PER NNUE ---
                // Salviamo la posizione (FEN), il punteggio e la mossa migliore
                if let Ok(mut file) = OpenOptions::new().append(true).create(true).open(data_path) {
                    // Inseriamo anche la valutazione della ricerca per avere dati "etichettati"
                    let _ = writeln!(file, "{} | bestmove: {} | score: {}", board.get_fen(), best_move.to_uci(), info.nodes);
                }

                println!("bestmove {}", best_move.to_uci());
            }
            "quit" => break,
            _ => {}
        }
    }
}