use std::io::{self, BufRead};
use crate::board::Scacchiera;
use crate::search::{iterative_deepening, SearchInfo};
use crate::tt::TranspositionTable;
use crate::zobrist::ZobristKeys;
use std::time::Instant;

pub fn uci_loop() {
    let stdin = io::stdin();
    let z = ZobristKeys::init();
    let mut board = Scacchiera::new(); // Scacchiera::new() ora internamente usa from_fen
    let mut tt = TranspositionTable::new(64);
    let mut history: Vec<u64> = Vec::new();

    for line in stdin.lock().lines() {
        let input = match line {
            Ok(l) => l,
            Err(_) => break,
        };
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
                // Reset della scacchiera alla posizione iniziale o FEN
                if parts[1] == "startpos" {
                    board = Scacchiera::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", &z);
                } else if parts[1] == "fen" {
                    // Se Arena invia una stringa FEN (es. position fen ...)
                    let fen_str = parts[2..6].join(" ");
                    board = Scacchiera::from_fen(&fen_str, &z);
                }
                
                history.clear();
                
                // Applichiamo la lista delle mosse "moves"
                if let Some(pos) = parts.iter().position(|&r| r == "moves") {
                    for m_str in &parts[pos + 1..] {
                        let moves = board.genera_mosse();
                        let mut found = false;
                        for m in moves {
                            if m.to_string() == *m_str {
                                history.push(board.hash); // Usiamo l'hash già aggiornato nella board
                                if !board.esegui_mossa(&m, &z) {
                                    eprintln!("Attenzione: Mossa {} illegale ignorata", m_str);
                                }
                                found = true;
                                break;
                            }
                        }
                        if !found {
                            eprintln!("Errore critico: Mossa {} non trovata dal generatore", m_str);
                        }
                    }
                }
            }
            "go" => {
                let mut time_ms: u128 = 5000;
                
                // Parsing avanzato del tempo
                for i in 1..parts.len() {
                    match parts[i] {
                        "wtime" if board.turno == crate::board::Colore::Bianco => {
                            let total_time = parts[i+1].parse::<u128>().unwrap_or(5000);
                            time_ms = total_time / 25; // Gestione tempo: usa il 4% del tempo totale
                        }
                        "btime" if board.turno == crate::board::Colore::Nero => {
                            let total_time = parts[i+1].parse::<u128>().unwrap_or(5000);
                            time_ms = total_time / 25;
                        }
                        "movetime" => {
                            time_ms = parts[i+1].parse::<u128>().unwrap_or(5000);
                        }
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