use std::io::{self, BufRead};
use std::time::Instant;
use crate::board::{Scacchiera, Mossa};
use crate::search::{self, SearchInfo, INFINITY};
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
                println!("id name Luna 16.8 (Stable)");
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
                // 1. BOOK CHECK
                let mut move_found_in_book = false;
                if use_own_book {
                    if let Some(book_move) = book::get_book_move_str(&move_history_str) {
                        println!("bestmove {}", book_move);
                        move_found_in_book = true;
                    }
                }
                if move_found_in_book { continue; }

                // 2. PREPARAZIONE RICERCA
                // --- FIX CRITICO: FILTRO MOSSE LEGALI ---
                // Prima di cercare, generiamo solo le mosse che non lasciano il re sotto scacco.
                let pseudo_moves = board.genera_mosse();
                let mut legal_moves = Vec::new();
                for m in pseudo_moves {
                    if board.esegui_mossa(&m, None) {
                        board.annulla_mossa(&m, None, None);
                        legal_moves.push(m);
                    }
                }

                // Se non ci sono mosse legali, è Matto o Stallo.
                // Arena si aspetta "bestmove (none)" o simile, oppure non inviamo nulla e lasciamo che il tempo scada,
                // ma la cosa corretta è non crashare.
                if legal_moves.is_empty() {
                    // Non possiamo muovere. 
                    println!("bestmove (none)");
                    continue;
                }
                // ----------------------------------------

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

                let mut best_move_found = legal_moves[0]; // Sicuro perché legal_moves non è vuoto
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
                    
                    // Ordiniamo solo le mosse legali
                    legal_moves.sort_by_cached_key(|m| -search::punteggio_mossa(&board, m));
                    
                    let mut best_move_this_depth = legal_moves[0];
                    let mut max_score = -INFINITY;
                    let mut move_found_in_search = false;

                    for mv in &legal_moves {
                        // esegui_mossa qui è ridondante per la legalità (già controllata),
                        // ma serve per aggiornare lo stato della board per la ricorsione.
                        if board.esegui_mossa(mv, None) {
                            let score = -search::search(&mut board, depth - 1, -beta, -alpha, &mut info);
                            board.annulla_mossa(mv, None, None);

                            if info.stop { break; }

                            if score > max_score {
                                max_score = score;
                                best_move_this_depth = *mv;
                                move_found_in_search = true;
                            }
                            if score > alpha { alpha = score; }
                        }
                    }

                    if info.stop {
                        break; 
                    } else {
                        // Se abbiamo trovato una mossa valida nella ricerca, aggiorniamo il best globale
                        if move_found_in_search {
                            best_move_found = best_move_this_depth;
                        }
                        
                        // Formattazione speciale per il punteggio di matto
                        let score_str = if max_score < -900000 {
                            format!("mate -{}", (1000000 + max_score + 1) / 2)
                        } else if max_score > 900000 {
                            format!("mate {}", (1000000 - max_score + 1) / 2)
                        } else {
                            format!("cp {}", max_score)
                        };

                        println!("info depth {} score {} nodes {} time {} pv {}", 
                            depth, score_str, info.nodes, info.start_time.elapsed().as_millis(), best_move_this_depth);
                    }
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