// src/uci.rs
use std::io::{self, BufRead};
use std::time::Instant;
use crate::board::{Scacchiera, Mossa};
use crate::search::{self, SearchInfo, INFINITY};
use crate::nnue::Network;

pub fn uci_loop(net: &Network) {
    let mut board = Scacchiera::nuova();
    let stdin = io::stdin();

    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() { continue; }

        match parts[0] {
            "uci" => {
                println!("id name Luna 16.5");
                println!("id author Gemini");
                println!("uciok");
            }
            "isready" => println!("readyok"),
            "ucinewgame" => board = Scacchiera::nuova(),
            "position" => {
                if parts.len() > 1 && parts[1] == "startpos" {
                    board = Scacchiera::nuova();
                    if parts.len() > 2 && parts[2] == "moves" {
                        for mossa_str in &parts[3..] {
                            applica_mossa_uci(&mut board, mossa_str);
                        }
                    }
                }
            }
            "go" => {
                let mut time_limit_ms = 1000; // Default 1 secondo
                let mut wtime: Option<u64> = None;
                let mut btime: Option<u64> = None;

                // Parsing basilare del tempo
                for i in 1..parts.len() {
                    if parts[i] == "wtime" { wtime = parts.get(i+1).and_then(|s| s.parse().ok()); }
                    if parts[i] == "btime" { btime = parts.get(i+1).and_then(|s| s.parse().ok()); }
                }

                // Gestione del tempo
                if let (Some(wt), Some(bt)) = (wtime, btime) {
                    let my_time = if board.turno == crate::board::Colore::Bianco { wt } else { bt };
                    // Usa circa il 4% del tempo rimanente + margine
                    time_limit_ms = (my_time / 25).max(100) as u128; 
                }

                let mut best_move_found = None;
                let mut info = SearchInfo {
                    start_time: Instant::now(),
                    time_limit_ms,
                    nodes: 0,
                    stop: false,
                };

                // ITERATIVE DEEPENING
                for depth in 1..=20 {
                    let mut alpha = -INFINITY;
                    let beta = INFINITY;
                    
                    let moves = board.genera_mosse();
                    if moves.is_empty() { break; }
                    
                    // Ordina le mosse prima di cercare
                    let mut sorted_moves = moves;
                    sorted_moves.sort_by_cached_key(|m| -search::punteggio_mossa(&board, m));
                    
                    let mut best_move_this_depth = sorted_moves[0];
                    let mut max_score = -INFINITY;

                    for mv in sorted_moves {
                         if board.esegui_mossa(&mv, None) {
                            let score = -search::search(&mut board, depth - 1, -beta, -alpha, &mut info);
                            board.annulla_mossa(&mv, None, None);

                            if info.stop { break; }

                            if score > max_score {
                                max_score = score;
                                best_move_this_depth = mv;
                            }
                            if score > alpha { alpha = score; }
                        }
                    }

                    if info.stop {
                        break; 
                    } else {
                        best_move_found = Some(best_move_this_depth);
                        // Output info stile UCI
                        println!("info depth {} score cp {} nodes {} time {} pv {}", 
                            depth, max_score, info.nodes, info.start_time.elapsed().as_millis(), best_move_this_depth);
                    }
                }

                if let Some(bm) = best_move_found {
                    println!("bestmove {}", bm);
                } else {
                    // Fallback se il tempo è scaduto subito
                    let moves = board.genera_mosse();
                    println!("bestmove {}", moves[0]);
                }
            }
            "quit" => break,
            _ => {}
        }
    }
}

fn applica_mossa_uci(board: &mut Scacchiera, mossa_str: &str) {
    let mosse = board.genera_mosse();
    if let Some(m) = mosse.iter().find(|m| m.to_string() == mossa_str) {
        board.esegui_mossa(m, None);
    }
}