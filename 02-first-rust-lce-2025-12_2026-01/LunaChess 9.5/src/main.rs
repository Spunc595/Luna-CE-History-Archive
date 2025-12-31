use std::io::{self, BufRead};
use std::sync::Arc;

mod board;
mod attacks;
mod movegen;
mod evaluation;
mod search;
mod transposition;
mod book;
mod polyglot_keys;

use crate::board::{Scacchiera, Colore}; // Rimossi import inutilizzati
use crate::search::Motore;
use crate::transposition::TranspositionTable;
use crate::book::OpeningBook;

fn main() {
    println!("LCE Pro - Rust Engine");
    
    // Ora attacks::init() è visibile (vedi file attacks.rs)
    attacks::init();

    let tt = Arc::new(TranspositionTable::new(64)); 
    let mut motore = Motore::nuovo(tt.clone());
    let mut board = Scacchiera::nuova();
    
    let mut book = OpeningBook::new();
    book.load("book.bin");

    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let cmd = line.trim();
        if cmd.is_empty() { continue; }

        let parts: Vec<&str> = cmd.split_whitespace().collect();
        let command = parts[0];

        match command {
            "uci" => {
                println!("id name LCE Pro 1.0");
                println!("id author TuoNome");
                println!("option name Hash type spin default 64 min 1 max 1024");
                println!("uciok");
            },
            "isready" => println!("readyok"),
            "ucinewgame" => {
                tt.clear();
                board = Scacchiera::nuova();
            },
            "position" => {
                let mut move_index = 0;
                if parts.len() > 1 {
                    if parts[1] == "startpos" {
                        board = Scacchiera::nuova();
                        move_index = 2;
                    } else if parts[1] == "fen" {
                        let mut fen = String::new();
                        let mut i = 2;
                        while i < parts.len() && parts[i] != "moves" {
                            fen.push_str(parts[i]);
                            fen.push(' ');
                            i += 1;
                        }
                        if let Ok(b) = Scacchiera::da_fen(fen.trim()) {
                            board = b;
                        }
                        move_index = i;
                    }
                }

                if move_index < parts.len() && parts[move_index] == "moves" {
                    for m_str in &parts[move_index+1..] {
                        // CORREZIONE: Passiamo &mut board perché movegen lo richiede
                        let moves = crate::movegen::genera_mosse_legali(&mut board);
                        if let Some(m) = moves.into_iter().find(|m| m.to_string() == *m_str) {
                            board.esegui_mossa(&m);
                        }
                    }
                }
            },
            "go" => {
                if let Some(book_move) = book.get_book_move(&board) {
                    println!("info string book move");
                    println!("bestmove {}", book_move);
                    // board.esegui_mossa(&book_move); // Opzionale per debug
                    continue; 
                }

                let mut wtime: u64 = 0;
                let mut btime: u64 = 0;
                let mut movetime: u64 = 0;
                let mut infinite = false;

                for i in 1..parts.len() {
                    match parts[i] {
                        "wtime" => if let Ok(t) = parts.get(i+1).unwrap_or(&"0").parse() { wtime = t; },
                        "btime" => if let Ok(t) = parts.get(i+1).unwrap_or(&"0").parse() { btime = t; },
                        "movetime" => if let Ok(t) = parts.get(i+1).unwrap_or(&"0").parse() { movetime = t; },
                        "infinite" => infinite = true,
                        _ => {}
                    }
                }

                let time_limit = if movetime > 0 {
                    movetime 
                } else if infinite {
                    0 
                } else {
                    if board.turno == Colore::Bianco { wtime } else { btime }
                };

                let best = motore.trova_mossa_migliore(&mut board, time_limit, true);
                
                if let Some(m) = best {
                    println!("bestmove {}", m);
                } else {
                    println!("bestmove 0000"); 
                }
            },
            "stop" => {
                // In modalità single-thread CLI, stop è difficile da intercettare durante la ricerca.
                // Il time management del motore lo fermerà automaticamente.
            },
            "quit" => break,
            _ => {}
        }
    }
}