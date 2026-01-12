use std::io::{self, BufRead};
use crate::board::{Scacchiera, Mossa};
use crate::search::{iterative_deepening, SearchInfo};
use crate::tt::TranspositionTable;
use crate::zobrist::ZobristKeys;
use std::time::Instant;

pub fn uci_loop() {
    let stdin = io::stdin();
    let z = ZobristKeys::init();
    
    // Inizializzazione della scacchiera alla posizione di partenza
    let mut board = Scacchiera::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", &z);
    let mut tt = TranspositionTable::new(64); // 64 MB di default
    let mut history_hashes: Vec<u64> = Vec::new();

    for line in stdin.lock().lines() {
        let input = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.is_empty() { continue; }

        match parts[0] {
            "uci" => {
                println!("id name Luna 18.2");
                println!("id author Daniele & Gemini");
                println!("uciok");
            }
            "isready" => println!("readyok"),
            "ucinewgame" => {
                board = Scacchiera::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", &z);
                tt.clear();
                history_hashes.clear();
            }
            "position" => {
                if parts.len() < 2 { continue; }
                
                if parts[1] == "startpos" {
                    board = Scacchiera::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", &z);
                } else if parts[1] == "fen" {
                    if parts.len() >= 8 {
                        let fen_str = parts[2..8].join(" ");
                        board = Scacchiera::from_fen(&fen_str, &z);
                    }
                }
                
                history_hashes.clear();
                
                // Caricamento della lista mosse
                if let Some(pos) = parts.iter().position(|&r| r == "moves") {
                    for m_str in &parts[pos + 1..] {
                        let moves = board.genera_mosse();
                        let mut found = false;
                        for m in moves {
                            if m.to_uci() == *m_str {
                                history_hashes.push(board.hash);
                                if !board.esegui_mossa(&m, &z) {
                                    eprintln!("Errore: mossa illegale ricevuta!");
                                }
                                found = true;
                                break;
                            }
                        }
                        if !found { eprintln!("Mossa UCI {} non trovata!", m_str); }
                    }
                }
            }
            "go" => {
                let mut time_ms: u128 = 3000; // Default 3 secondi
                
                for i in 1..parts.len() {
                    match parts[i] {
                        "wtime" if board.turno == crate::board::Colore::Bianco => {
                            if let Ok(t) = parts[i+1].parse::<u128>() { time_ms = t / 30; }
                        }
                        "btime" if board.turno == crate::board::Colore::Nero => {
                            if let Ok(t) = parts[i+1].parse::<u128>() { time_ms = t / 30; }
                        }
                        "movetime" => {
                            if let Ok(t) = parts[i+1].parse::<u128>() { time_ms = t; }
                        }
                        _ => {}
                    }
                }

                // Inizializzazione di SearchInfo con le nuove tabelle euristiche
                let mut info = SearchInfo {
                    start_time: Instant::now(),
                    time_limit_ms: time_ms,
                    nodes: 0,
                    stop: false,
                    history: history_hashes.clone(),
                    killer_moves: [[Mossa { data: 0 }; 2]; 64],
                    history_scores: [[0u32; 64]; 64],
                };

                tt.new_search();
                let best_move = iterative_deepening(&mut board, &mut info, &mut tt, &z);
                println!("bestmove {}", best_move.to_uci());
            }
            "quit" => break,
            _ => {}
        }
    }
}