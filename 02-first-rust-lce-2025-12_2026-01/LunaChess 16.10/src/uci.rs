use std::io::{self, BufRead};
use std::time::Instant;
use crate::board::{Scacchiera, Mossa};
use crate::search::{self, SearchInfo, INFINITY, PvLine}; // Import PvLine
use crate::nnue::Network;
use crate::book;

pub fn uci_loop(net: &Network) {
    let mut board = Scacchiera::nuova();
    let stdin = io::stdin();
    
    let mut move_history_str = String::new(); 
    let mut use_own_book = true; 

    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() { continue; }

        match parts[0] {
            "uci" => {
                println!("id name Luna 16.13 (Prophet)");
                println!("id author Gemini");
                println!("option name OwnBook type check default true");
                println!("uciok");
            },
            "isready" => println!("readyok"),
            "setoption" => {
                if parts.len() >= 5 && parts[1] == "name" && parts[2] == "OwnBook" && parts[3] == "value" {
                    use_own_book = parts[4] == "true";
                }
            },
            "ucinewgame" => {
                board = Scacchiera::nuova();
                move_history_str = String::new();
            },
            "position" => {
                if parts.len() > 1 && parts[1] == "startpos" {
                    board = Scacchiera::nuova();
                    move_history_str = String::new();
                    if parts.len() > 2 && parts[2] == "moves" {
                        for (i, mossa_str) in parts[3..].iter().enumerate() {
                            applica_mossa_uci(&mut board, mossa_str);
                            if i > 0 { move_history_str.push(' '); }
                            move_history_str.push_str(mossa_str);
                        }
                    }
                }
            },
            "go" => {
                // 1. BOOK
                let mut move_found_in_book = false;
                if use_own_book {
                    if let Some(book_move) = book::get_book_move_str(&move_history_str) {
                        println!("bestmove {}", book_move);
                        move_found_in_book = true;
                    }
                }
                if move_found_in_book { continue; }

                // 2. SAFETY CHECK
                let pseudo_moves = board.genera_mosse();
                let mut legal_moves = Vec::new();
                for m in pseudo_moves {
                    if board.esegui_mossa(&m, None) {
                        board.annulla_mossa(&m, None, None);
                        legal_moves.push(m);
                    }
                }
                if legal_moves.is_empty() {
                    println!("bestmove (none)");
                    continue;
                }

                // 3. TIME MANAGEMENT
                let mut time_limit_ms = 1000;
                let mut wtime: Option<u64> = None;
                let mut btime: Option<u64> = None;

                for i in 1..parts.len() {
                    if parts[i] == "wtime" { wtime = parts.get(i+1).and_then(|s| s.parse().ok()); }
                    if parts[i] == "btime" { btime = parts.get(i+1).and_then(|s| s.parse().ok()); }
                }

                if let (Some(wt), Some(bt)) = (wtime, btime) {
                    let my_time = if board.turno == crate::board::Colore::Bianco { wt } else { bt };
                    time_limit_ms = (my_time / 25).max(50) as u128; 
                }

                let mut best_move_found = legal_moves[0];
                let mut info = SearchInfo {
                    start_time: Instant::now(),
                    time_limit_ms,
                    nodes: 0,
                    stop: false,
                };

                // 4. ITERATIVE DEEPENING CON PV
                for depth in 1..=20 {
                    let mut alpha = -search::INFINITY;
                    let beta = search::INFINITY;
                    
                    // Creiamo una PV line per la radice
                    let mut root_pv = PvLine::new();

                    // Chiamiamo search direttamente dalla radice per popolare root_pv
                    let score = search::search(&mut board, depth, alpha, beta, 0, &mut info, &mut root_pv);

                    if info.stop { break; }

                    // Se abbiamo una linea PV valida, la prima mossa è la migliore
                    if root_pv.len > 0 {
                        best_move_found = root_pv.moves[0];
                    }

                    // Formattazione Punteggio
                    let score_str = if score < -40000 {
                        format!("mate -{}", (49000 + score + 1) / 2)
                    } else if score > 40000 {
                        format!("mate {}", (49000 - score + 1) / 2)
                    } else {
                        format!("cp {}", score)
                    };

                    // COSTRUZIONE STRINGA PV
                    let mut pv_str = String::new();
                    for i in 0..root_pv.len {
                        pv_str.push_str(&format!("{} ", root_pv.moves[i]));
                    }

                    // OUTPUT COMPLETO
                    println!("info depth {} score {} nodes {} time {} pv {}", 
                        depth, score_str, info.nodes, info.start_time.elapsed().as_millis(), pv_str);
                }

                println!("bestmove {}", best_move_found);
            },
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