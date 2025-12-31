use std::io::{self, BufRead};
use std::sync::{Arc, RwLock};

mod board;
mod attacks;
mod movegen;
mod evaluation;
mod search;
mod transposition;

use crate::board::{Scacchiera, Colore};
use crate::search::Motore;
use crate::transposition::TranspositionTable;

fn main() {
    attacks::init();
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
                println!("id name LCE 1 Pro+ LMR");
                println!("id author Gemini");
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
                for i in 1..parts.len() {
                    if (board.turno() == Colore::Bianco && parts[i] == "wtime") ||
                       (board.turno() == Colore::Nero && parts[i] == "btime") {
                        tempo_ms = parts[i+1].parse().unwrap_or(0);
                    }
                }
                if let Some(mossa) = motore.trova_mossa_migliore(&mut board, tempo_ms, true) {
                    println!("bestmove {}", mossa);
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
        let fen = parts[2..8].join(" ");
        if let Ok(b) = Scacchiera::da_fen(&fen) { *board = b; }
        move_start = 8;
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