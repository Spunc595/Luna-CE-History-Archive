use std::io::{self, BufRead};
use std::time::Instant;
use crate::board::{Scacchiera, Mossa, Colore};
use crate::search::{self, SearchInfo}; 
use crate::tt::TranspositionTable; 
use crate::zobrist::ZobristKeys;   

pub fn uci_loop() {
    let mut board = Scacchiera::nuova();
    let stdin = io::stdin();
    
    let mut move_history_str = String::new(); 
    let mut use_own_book = true; 

    // Hash e Zobrist - Inizializzazione corretta
    let mut tt = TranspositionTable::new(64); 
    let zobrist = ZobristKeys::new(); // FIX: cambiato da init() a new()

    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() { continue; }

        match parts[0] {
            "uci" => {
                println!("id name Luna 16.20 (Safe Clone)");
                println!("id author Gemini");
                println!("option name OwnBook type check default true");
                println!("option name Hash type spin default 64 min 1 max 1024");
                println!("uciok");
            },
            "isready" => println!("readyok"),
            "setoption" => {
                if parts.len() >= 5 && parts[1] == "name" && parts[2] == "OwnBook" && parts[3] == "value" {
                    use_own_book = parts[4] == "true";
                }
                if parts.len() >= 5 && parts[1] == "name" && parts[2] == "Hash" && parts[3] == "value" {
                    if let Ok(mb) = parts[4].parse::<usize>() {
                        tt = TranspositionTable::new(mb);
                    }
                }
            },
            "ucinewgame" => {
                board = Scacchiera::nuova();
                move_history_str = String::new();
                tt.clear(); // Ora implementato in tt.rs
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
                // 1. Opening Book (se implementato)
                /* if use_own_book {
                    if let Some(book_move) = crate::book::get_book_move_str(&move_history_str) {
                        println!("bestmove {}", book_move);
                        continue;
                    }
                }
                */

                // 2. Controllo Preliminare Mosse Legali
                let pseudo_moves = board.genera_mosse();
                let mut legal_moves = Vec::new();
                for m in pseudo_moves {
                    let mut temp_board = board.clone();
                    if temp_board.esegui_mossa(&m, None) {
                        legal_moves.push(m);
                    }
                }
                
                if legal_moves.is_empty() {
                    println!("bestmove (none)");
                    continue;
                }

                // 3. Gestione Tempo
                let mut time_limit_ms: u128 = 1000;
                let mut wtime: Option<u64> = None;
                let mut btime: Option<u64> = None;
                let mut movestogo: Option<u64> = None;

                for i in 1..parts.len() {
                    match parts[i] {
                        "wtime" => wtime = parts.get(i+1).and_then(|s| s.parse().ok()),
                        "btime" => btime = parts.get(i+1).and_then(|s| s.parse().ok()),
                        "movestogo" => movestogo = parts.get(i+1).and_then(|s| s.parse().ok()),
                        _ => {}
                    }
                }

                if let (Some(wt), Some(bt)) = (wtime, btime) {
                    let my_time = if board.turno == Colore::Bianco { wt } else { bt };
                    let moves_remaining = movestogo.unwrap_or(30).max(2);
                    time_limit_ms = (my_time / moves_remaining) as u128;
                }

                let mut best_move_found = legal_moves[0];
                let mut info = SearchInfo {
                    start_time: Instant::now(),
                    time_limit_ms,
                    nodes: 0,
                    stop: false,
                };

                // 4. Ricerca Iterative Deepening
                for depth in 1..=64 {
                    let (score_val, best_mv, root_pv) = search::search_root(&mut board, depth, &mut info, &mut tt, &zobrist);

                    if info.stop && depth > 1 { break; }

                    best_move_found = best_mv;

                    let score_str = if score_val < -40000 {
                        format!("mate -{}", (search::MATE_VALUE + score_val) / 2)
                    } else if score_val > 40000 {
                        format!("mate {}", (search::MATE_VALUE - score_val) / 2)
                    } else {
                        format!("cp {}", score_val)
                    };

                    let elapsed = info.start_time.elapsed().as_millis().max(1);
                    let nps = (info.nodes as u128 * 1000) / elapsed;

                    print!("info depth {} score {} nodes {} time {} nps {} pv", 
                        depth, score_str, info.nodes, elapsed, nps);
                    
                    for i in 0..root_pv.len {
                        print!(" {}", root_pv.moves[i]);
                    }
                    println!();

                    // Se abbiamo trovato un matto o il tempo sta per scadere, esci
                    if score_val > 45000 || score_val < -45000 { break; }
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
    // FIX: Aggiunta annotazione di tipo esplicita per evitare errori di inferenza
    if let Some(m) = mosse.iter().find(|m: &&Mossa| m.to_string() == mossa_str) {
        board.esegui_mossa(m, None);
    }
}