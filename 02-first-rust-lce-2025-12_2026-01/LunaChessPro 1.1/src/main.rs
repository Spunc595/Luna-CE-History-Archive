#[macro_use]
extern crate lazy_static;

use std::io::{self, BufRead};
use std::sync::{Arc, RwLock};

mod board;
mod attacks;
mod movegen;
mod evaluation;
mod search;
mod transposition;
mod nnue; 

use crate::board::{Scacchiera, Colore};
use crate::search::Motore;
use crate::transposition::TranspositionTable;
use crate::nnue::NNUE; // Importiamo quella definita in nnue.rs

fn main() {
    attacks::init();
    
    // Inizializza la rete (che ora vive in nnue.rs)
    lazy_static::initialize(&NNUE);

    let tt = Arc::new(RwLock::new(TranspositionTable::new(64)));
    let mut motore = Motore::nuovo(tt.clone());
    let mut board = Scacchiera::nuova();

    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() { continue; }

        match parts[0] {
            "uci" => {
                println!("id name Luna Chess Engine 11.0 NNUE");
                println!("id author Gemini");
                println!("option name Hash type spin default 64 min 1 max 1024");
                if NNUE.is_some() {
                    println!("option name NNUE type check default true");
                }
                println!("uciok");
            }
            "isready" => println!("readyok"),
            "ucinewgame" => {
                if let Ok(mut tt_w) = tt.write() { tt_w.clear(); }
                board = Scacchiera::nuova();
            }
            "position" => setup_position(&mut board, &parts),
            "go" => {
                let mut tempo_ms = 0;
                let mut inc_ms = 0;
                let mut _depth_limit = 64; // Underscore per evitare warning

                for i in 1..parts.len() {
                    match parts[i] {
                        "wtime" if board.turno() == Colore::Bianco => tempo_ms = parts[i+1].parse().unwrap_or(0),
                        "btime" if board.turno() == Colore::Nero => tempo_ms = parts[i+1].parse().unwrap_or(0),
                        "winc" if board.turno() == Colore::Bianco => inc_ms = parts[i+1].parse().unwrap_or(0),
                        "binc" if board.turno() == Colore::Nero => inc_ms = parts[i+1].parse().unwrap_or(0),
                        "depth" => _depth_limit = parts[i+1].parse().unwrap_or(64),
                        _ => {}
                    }
                }
                
                let tempo_totale = tempo_ms + inc_ms * 2; 
                if let Some(mossa) = motore.trova_mossa_migliore(&mut board, tempo_totale, true) {
                    println!("bestmove {}", mossa);
                }
            }
            "setoption" => {
                if parts.len() >= 5 && parts[2] == "Hash" && parts[3] == "value" {
                    if let Ok(mb) = parts[4].parse::<usize>() {
                        if let Ok(mut tt_w) = tt.write() {
                            if mb > 0 && mb <= 2048 { *tt_w = TranspositionTable::new(mb); }
                        }
                    }
                }
            }
            "quit" => break,
            _ => {}
        }
    }
}

fn setup_position(board: &mut Scacchiera, parts: &[&str]) {
    let mut move_start = 0;
    if parts.len() < 2 { return; }
    
    if parts[1] == "startpos" {
        *board = Scacchiera::nuova();
        move_start = 2;
    } else if parts[1] == "fen" {
        let mut fen_parts = Vec::new();
        let mut i = 2;
        while i < parts.len() && parts[i] != "moves" {
            fen_parts.push(parts[i]);
            i += 1;
        }
        let fen = fen_parts.join(" ");
        if let Ok(b) = Scacchiera::da_fen(&fen) { *board = b; }
        move_start = i;
    }

    if move_start < parts.len() && parts[move_start] == "moves" {
        for i in (move_start + 1)..parts.len() {
            let moves = movegen::genera_mosse_legali(board);
            if let Some(m) = moves.into_iter().find(|m| m.to_string() == parts[i]) {
                board.esegui_mossa(&m);
            }
        }
    }
}