use std::io::{self, BufRead};
use std::time::Instant;
use crate::board::{Scacchiera, Mossa};
use crate::search::{self, SearchInfo}; 
use crate::tt::TranspositionTable; // Importa la TT
use crate::zobrist::ZobristKeys;   // Importa Zobrist
use crate::nnue::Network;
use crate::book;

pub fn uci_loop(_net: &Network) {
    let mut board = Scacchiera::nuova();
    let stdin = io::stdin();
    
    let mut move_history_str = String::new(); 
    let mut use_own_book = true; 

    // --- 1. INIZIALIZZAZIONE MEMORIA (TT & ZOBRIST) ---
    // Creiamo la tabella di trasposizione (default 64MB)
    let mut tt = TranspositionTable::new(64); 
    // Inizializziamo i numeri casuali per l'hashing
    let zobrist = ZobristKeys::init();

    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() { continue; }

        match parts[0] {
            "uci" => {
                println!("id name Luna 16.20 (Transposition Table)");
                println!("id author Gemini");
                
                // Opzioni UCI
                println!("option name OwnBook type check default true");
                println!("option name Hash type spin default 64 min 1 max 1024");
                
                println!("uciok");
            },
            "isready" => println!("readyok"),
            "setoption" => {
                // Gestione OwnBook
                if parts.len() >= 5 && parts[1] == "name" && parts[2] == "OwnBook" && parts[3] == "value" {
                    use_own_book = parts[4] == "true";
                }
                // Gestione Hash (Ridimensiona la memoria)
                if parts.len() >= 5 && parts[1] == "name" && parts[2] == "Hash" && parts[3] == "value" {
                    if let Ok(mb) = parts[4].parse::<usize>() {
                        tt = TranspositionTable::new(mb);
                    }
                }
            },
            "ucinewgame" => {
                board = Scacchiera::nuova();
                move_history_str = String::new();
                // Importante: Pulisci la TT quando inizia una nuova partita
                tt.clear(); 
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
                // --- 2. GESTIONE OPENING BOOK ---
                let mut move_found_in_book = false;
                if use_own_book {
                    if let Some(book_move) = book::get_book_move_str(&move_history_str) {
                        println!("bestmove {}", book_move);
                        move_found_in_book = true;
                    }
                }
                if move_found_in_book { continue; }

                // --- 3. SAFETY CHECK (Mosse Legali) ---
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

                // --- 4. GESTIONE TEMPO ---
                let mut time_limit_ms = 1000;
                let mut wtime: Option<u64> = None;
                let mut btime: Option<u64> = None;

                for i in 1..parts.len() {
                    if parts[i] == "wtime" { wtime = parts.get(i+1).and_then(|s| s.parse().ok()); }
                    if parts[i] == "btime" { btime = parts.get(i+1).and_then(|s| s.parse().ok()); }
                }

                if let (Some(wt), Some(bt)) = (wtime, btime) {
                    let my_time = if board.turno == crate::board::Colore::Bianco { wt } else { bt };
                    // Usa circa il 5% del tempo rimanente
                    time_limit_ms = (my_time / 20).max(50) as u128; 
                }

                let mut best_move_found = legal_moves[0];
                let mut info = SearchInfo {
                    start_time: Instant::now(),
                    time_limit_ms,
                    nodes: 0,
                    stop: false,
                };

                // --- 5. RICERCA (ITERATIVE DEEPENING) ---
                // Qui avviene la magia della Transposition Table
                for depth in 1..=20 {
                    
                    // CORREZIONE CRITICA: search_root ritorna (punteggio, mossa).
                    // Dobbiamo catturarli entrambi.
                    // Passiamo anche &mut tt e &zobrist che abbiamo creato all'inizio.
                    let (score_val, best_mv) = search::search_root(&mut board, depth, &mut info, &mut tt, &zobrist);

                    // Se il tempo è scaduto durante la ricerca, fermiamo tutto
                    if info.stop { break; }

                    // Aggiorniamo la miglior mossa trovata a questa profondità
                    best_move_found = best_mv;

                    // Formattazione Punteggio (per l'interfaccia grafica)
                    let score_str = if score_val < -40000 {
                        format!("mate -{}", (49000 + score_val + 1) / 2)
                    } else if score_val > 40000 {
                        format!("mate {}", (49000 - score_val + 1) / 2)
                    } else {
                        format!("cp {}", score_val)
                    };

                    // Output Info
                    // Nota: best_move_found qui è la prima mossa della PV.
                    println!("info depth {} score {} nodes {} time {} pv {}", 
                        depth, score_str, info.nodes, info.start_time.elapsed().as_millis(), best_move_found);
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