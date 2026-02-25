mod board;
mod movegen;
mod attacks;
mod zobrist;
mod nnue;
mod evaluation;
mod search;
mod tt;
mod book; // <--- 1. AGGIUNGI IL MODULO

use std::io::{self, BufRead, Write};
use crate::board::{Scacchiera, Colore};
use crate::zobrist::get_zobrist_keys;
use crate::nnue::LunaNNUE;
use crate::evaluation::evaluate;
use crate::search::{iterative_deepening, SearchInfo};
use crate::tt::TranspositionTable;
use crate::book::OpeningBook; // <--- 2. IMPORTA LA STRUCT

fn main() {
    let z = get_zobrist_keys();
    // Assicurati che il nome del file .nnue sia quello corretto (es. luna_safe_62k.nnue)
    let nnue = LunaNNUE::load("luna.nnue"); 
    
    // --- 3. CARICAMENTO LIBRO ---
    let book = OpeningBook::load("book.bin");
    match book {
        Some(_) => println!("✅ Book: Attivo e carico!"),
        None => println!("⚠️ Book: File 'book.txt' non trovato. Si userà solo la rete."),
    }
    // ----------------------------

    let mut tt = TranspositionTable::new(512);
    let mut s = Scacchiera::new_iniziale(z);

    println!("Luna Engine v7.2 - Istruita Ready");
    io::stdout().flush().unwrap();

    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() { continue; }

        match parts[0] {
            "uci" => {
                println!("id name Luna_Hybrid_7.2");
                println!("id author Daniele");
                println!("option name Hash type spin default 256 min 1 max 1024");
                println!("uciok");
            }
            "isready" => println!("readyok"),
            "setoption" => {
                if parts.len() >= 5 && parts[2] == "Hash" {
                    if let Ok(new_size) = parts[4].parse::<usize>() {
                        tt = TranspositionTable::new(new_size);
                    }
                }
            }
            "position" => {
                if parts.len() > 1 {
                    if parts[1] == "startpos" {
                        s = Scacchiera::new_iniziale(z);
                    } else if parts[1] == "fen" {
                        let m_idx = parts.iter().position(|&p| p == "moves").unwrap_or(parts.len());
                        let fen_str = parts[2..m_idx].join(" ");
                        s = Scacchiera::from_fen(&fen_str, z);
                    }
                    if let Some(m_idx) = parts.iter().position(|&p| p == "moves") {
                        for &m_str in &parts[m_idx + 1..] {
                            let moves = s.genera_mosse_legali(z);
                            for m in moves {
                                if m.to_uci() == m_str { s.esegui_mossa(&m, z); break; }
                            }
                        }
                    }
                }
            }
            "go" => {
                // --- 4. CONSULTAZIONE LIBRO ---
                let mut mossa_trovata = false;
                if let Some(ref b) = book {
                    // Cerca nel libro
                    if let Some(book_move) = b.get_move(&s) {
                        println!("bestmove {}", book_move.to_uci());
                        mossa_trovata = true;
                    }
                }

                // Se NON abbiamo trovato la mossa nel libro, usiamo il cervello
                if !mossa_trovata {
                    let mut depth = 12;
                    let mut movetime: u128 = 5000;
                    
                    for i in (1..parts.len()).step_by(2) {
                        if i + 1 >= parts.len() { break; }
                        match parts[i] {
                            "wtime" if s.turno == Colore::Bianco => movetime = parts[i+1].parse::<u128>().unwrap_or(5000) / 25,
                            "btime" if s.turno == Colore::Nero => movetime = parts[i+1].parse::<u128>().unwrap_or(5000) / 25,
                            "depth" => depth = parts[i+1].parse().unwrap_or(12),
                            "movetime" => movetime = parts[i+1].parse().unwrap_or(5000),
                            _ => {}
                        }
                    }

                    let mut info = SearchInfo::new(movetime, depth as i32);
                    let (mut best_m, _) = iterative_deepening(&mut s, &mut info, &mut tt, &z, nnue.as_ref());
                    
                    let legali = s.genera_mosse_legali(z);
                    if !legali.iter().any(|m| m.data == best_m.data) {
                        if !legali.is_empty() { best_m = legali[0]; }
                    }
                    println!("bestmove {}", best_m.to_uci());
                }
            }
            "quit" => break,
            "eval" => {
                 // Codice eval esistente...
                 let score = if let Some(ref n) = nnue { n.evaluate(&s) } else { evaluate(&s) };
                 println!("Evaluation: {} cp", score);
            }
            _ => {}
        }
        io::stdout().flush().unwrap();
    }
}