use std::io::{self, BufRead};
use crate::board::{Scacchiera, Mossa, Colore};
use crate::search::{iterative_deepening, SearchInfo};
use crate::tt::TranspositionTable;
use crate::zobrist::ZobristKeys;
use crate::nnue::LunaNNUE;
use std::time::Instant;

/// Il loop UCI ora accetta l'istanza della NNUE già caricata dal main.
pub fn uci_loop(nnue: Option<LunaNNUE>) {
    let stdin = io::stdin();
    let z = ZobristKeys::init();
    
    // Inizializzazione posizione di partenza
    let mut board = Scacchiera::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", &z);
    
    // Tabella di trasposizione: 64MB di default
    let mut tt = TranspositionTable::new(64); 
    let mut history_hashes: Vec<u64> = Vec::new();

    // Creiamo un riferimento opzionale per la ricerca (per non spostare la proprietà di nnue)
    let nnue_ref = nnue.as_ref();

    for line in stdin.lock().lines() {
        let input = line.unwrap_or_default();
        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.is_empty() { continue; }

        match parts[0] {
            "uci" => {
                println!("id name Luna 0.5 NNUE");
                println!("id author Daniele & Alessandro");
                
                // Opzioni configurabili dal bot/GUI
                println!("option name Hash type spin default 64 min 1 max 2048");
                
                // Feedback sullo stato della rete neurale
                if nnue_ref.is_some() {
                    println!("info string NNUE Engine pronto all'uso.");
                } else {
                    println!("info string ATTENZIONE: NNUE non caricata. Uso valutazione classica.");
                }
                
                println!("uciok");
            },
            
            "isready" => println!("readyok"),
            
            "setoption" => {
                // Gestione opzioni (es. cambiare la dimensione della Hash durante il gioco)
                if parts.len() >= 5 && parts[2] == "Hash" {
                    if let Ok(new_size) = parts[4].parse::<usize>() {
                        tt = TranspositionTable::new(new_size);
                    }
                }
            },

            "ucinewgame" => {
                tt.clear();
                history_hashes.clear();
                board = Scacchiera::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", &z);
            },

            "position" => {
                if parts.len() < 2 { continue; }
                
                // Reset della posizione
                if parts[1] == "startpos" {
                    board = Scacchiera::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", &z);
                } else if parts[1] == "fen" {
                    // Uniamo i pezzi della FEN (le 6 parti standard)
                    let fen_str = parts[2..8.min(parts.len())].join(" ");
                    board = Scacchiera::from_fen(&fen_str, &z);
                }

                history_hashes.clear();
                
                // Esecuzione della lista di mosse (se presenti)
                if let Some(pos) = parts.iter().position(|&r| r == "moves") {
                    for m_str in &parts[pos + 1..] {
                        let moves = board.genera_mosse();
                        if let Some(m) = moves.iter().find(|m| m.to_uci() == *m_str) {
                            history_hashes.push(board.hash);
                            board.esegui_mossa(m, &z);
                        }
                    }
                }
            },

            "go" => {
                let mut time_available: u128 = 300000; // Default 5 minuti
                let mut increment: u128 = 0;
                let mut depth_limit = 0;
                let mut move_time = 0;

                // Parsing dei parametri di tempo inviati dal bot
                for i in 1..parts.len() {
                    match parts[i] {
                        "wtime" if board.turno == Colore::Bianco => time_available = parts[i+1].parse().unwrap_or(300000),
                        "btime" if board.turno == Colore::Nero => time_available = parts[i+1].parse().unwrap_or(300000),
                        "winc" if board.turno == Colore::Bianco => increment = parts[i+1].parse().unwrap_or(0),
                        "binc" if board.turno == Colore::Nero => increment = parts[i+1].parse().unwrap_or(0),
                        "movetime" => move_time = parts[i+1].parse().unwrap_or(0),
                        "depth" => depth_limit = parts[i+1].parse().unwrap_or(0),
                        _ => {}
                    }
                }

                // 1. Controllo Libro delle Aperture
                let mut book_found = false;
                if let Some(book_move) = crate::book::get_book_move(&board) {
                     println!("bestmove {}", book_move.to_uci());
                     book_found = true;
                }

                if !book_found {
                    // 2. Calcolo del Tempo per questa mossa
                    // Strategia: 1/20 del tempo rimasto + metà dell'incremento
                    let time_limit = if move_time > 0 { 
                        move_time 
                    } else { 
                        (time_available / 20) + (increment / 2) 
                    };

                    // Buffer di sicurezza di 50ms per evitare timeout su Lichess (latenza di rete)
                    let safe_time = if time_limit > 50 { time_limit - 50 } else { time_limit };

                    let mut info = SearchInfo {
                        start_time: Instant::now(),
                        time_limit_ms: safe_time,
                        nodes: 0,
                        stop: false,
                        history: history_hashes.clone(),
                        killer_moves: [[Mossa { data: 0 }; 2]; 64],
                        history_scores: [[0; 64]; 64],
                        depth_limit: if depth_limit > 0 { depth_limit as i32 } else { 64 },
                    };

                    // 3. Avvio Ricerca Iterativa
                    let (best_move, _) = iterative_deepening(&mut board, &mut info, &mut tt, &z, nnue_ref);
                    
                    // Invio mossa finale al bot
                    println!("bestmove {}", best_move.to_uci());
                }
            },

            "stop" => {
                // Comando per fermare la ricerca immediatamente (non implementato nel thread, 
                // ma utile se aggiungerai il threading)
            },

            "quit" => break,
            
            _ => {}
        }
    }
}