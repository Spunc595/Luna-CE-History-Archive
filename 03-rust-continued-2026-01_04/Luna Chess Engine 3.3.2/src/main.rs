mod board;
mod movegen;
mod attacks;
mod zobrist;
mod nnue;
mod evaluation;
mod search;
mod tt;
mod book;

use std::io::{self, BufRead, Write};
use crate::board::{Scacchiera, Colore};
use crate::zobrist::get_zobrist_keys;
use crate::nnue::LunaNNUE;
use crate::evaluation::evaluate;
use crate::search::{iterative_deepening, SearchInfo};
use crate::tt::TranspositionTable;
use crate::book::OpeningBook;

fn main() {
    let z = get_zobrist_keys();
    
    // 1. Caricamento della rete neurale
    let nnue = LunaNNUE::load("luna.nnue"); 
    if nnue.is_some() {
        println!("✅ NNUE: Caricata con successo!");
    } else {
        println!("⚠️ NNUE: File non trovato, uso valutazione classica.");
    }
    
    // 2. Caricamento Libro delle Aperture
    let book = OpeningBook::load("book.txt");
    match book {
        Some(_) => println!("✅ Book: Attivo e pronto!"),
        None => println!("⚠️ Book: 'book.txt' non trovato. Luna giocherà da sola dall'inizio."),
    }

    // 3. Inizializzazione Transposition Table (Default 256MB)
    let mut tt = TranspositionTable::new(256);
    let mut s = Scacchiera::new_iniziale(z);

    println!("Luna Engine v7.2 - Ready for Lichess");
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
                        println!("info string Hash set to {} MB", new_size);
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
                            if let Some(m) = moves.iter().find(|m| m.to_uci() == m_str) {
                                s.esegui_mossa(m, z);
                            }
                        }
                    }
                }
            }
            "go" => {
                // --- CONSULTAZIONE LIBRO ---
                let mut mossa_libro = None;
                if let Some(ref b) = book {
                    mossa_libro = b.get_move(&s);
                }

                if let Some(m) = mossa_libro {
                    println!("bestmove {}", m.to_uci());
                } else {
                    // --- RICERCA ALPHA-BETA ---
                    let mut depth = 64; // Limite massimo teorico
                    let mut movetime: u128 = 5000; // Default 5 secondi
                    
                    // Parsing parametri tempo UCI
                    for i in (1..parts.len()).step_by(2) {
                        if i + 1 >= parts.len() { break; }
                        match parts[i] {
                            "wtime" if s.turno == Colore::Bianco => {
                                let time = parts[i+1].parse::<u128>().unwrap_or(5000);
                                movetime = time / 30; // Gestione conservativa del tempo
                            }
                            "btime" if s.turno == Colore::Nero => {
                                let time = parts[i+1].parse::<u128>().unwrap_or(5000);
                                movetime = time / 30;
                            }
                            "depth" => depth = parts[i+1].parse().unwrap_or(12),
                            "movetime" => movetime = parts[i+1].parse().unwrap_or(5000),
                            _ => {}
                        }
                    }

                    let mut info = SearchInfo::new(movetime, depth as i32);
                    let (mut best_m, _) = iterative_deepening(&mut s, &mut info, &mut tt, &z, nnue.as_ref());
                    
                    // Fallback di sicurezza: se per qualche motivo la ricerca fallisce, prendi la prima legale
                    if best_m.is_null() {
                        let legali = s.genera_mosse_legali(z);
                        if !legali.is_empty() { best_m = legali[0]; }
                    }
                    
                    println!("bestmove {}", best_m.to_uci());
                }
            }
            "eval" => {
                let score = if let Some(ref n) = nnue { n.evaluate(&s) } else { evaluate(&s) };
                println!("Evaluation: {} cp", score);
            }
            "ucinewgame" => {
                tt.clear();
                s = Scacchiera::new_iniziale(z);
            }
            "quit" => break,
            _ => {}
        }
        io::stdout().flush().unwrap();
    }
}