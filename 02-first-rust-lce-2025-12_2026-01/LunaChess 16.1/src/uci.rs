// src/uci.rs
use std::io::{self, BufRead};
use crate::board::{Scacchiera, Mossa};
use crate::search;
use crate::nnue::Network;

pub fn uci_loop(net: &Network) {
    let mut board = Scacchiera::nuova();
    let stdin = io::stdin();

    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() { continue; }

        match parts[0] {
            "uci" => {
                println!("id name Luna 16");
                println!("id author Gemini");
                println!("uciok");
            }
            "isready" => println!("readyok"),
            "position" => {
                if parts.len() > 1 && parts[1] == "startpos" {
                    board = Scacchiera::nuova();
                    if parts.len() > 2 && parts[2] == "moves" {
                        for mossa_str in &parts[3..] {
                            applica_mossa_uci(&mut board, mossa_str);
                        }
                    }
                }
            }
            "go" => {
                let depth = 6;
                let score = search::search(&mut board, depth, -search::INFINITY, search::INFINITY);
                let moves = board.genera_mosse();
                if let Some(best_move) = moves.first() {
                    println!("info depth {} score cp {}", depth, score);
                    println!("bestmove {}", best_move);
                }
            }
            "quit" => break,
            _ => {}
        }
    }
}

fn applica_mossa_uci(board: &mut Scacchiera, mossa_str: &str) {
    let mosse = board.genera_mosse();
    // Specifichiamo &Mossa per aiutare il compilatore
    if let Some(m) = mosse.into_iter().find(|m: &Mossa| m.to_string() == mossa_str) {
        board.esegui_mossa(&m, None);
    }
}