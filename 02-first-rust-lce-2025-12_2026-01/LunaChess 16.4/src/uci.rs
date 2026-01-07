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
                println!("id name Luna 16.4");
                println!("id author Gemini");
                println!("uciok");
            }
            "isready" => println!("readyok"),
            "ucinewgame" => board = Scacchiera::nuova(),
            "position" => {
                if parts.len() > 1 {
                    if parts[1] == "startpos" {
                        board = Scacchiera::nuova();
                        if parts.len() > 2 && parts[2] == "moves" {
                            for mossa_str in &parts[3..] {
                                applica_mossa_uci(&mut board, mossa_str);
                            }
                        }
                    }
                }
            }
            "go" => {
                // Ricerca a profondità 6 (veloce e stabile)
                let depth = 6;
                let _score = search::search(&mut board, depth, -search::INFINITY, search::INFINITY);
                let mut moves = board.genera_mosse();
                
                // Usiamo lo stesso ordinamento della ricerca per dare la bestmove
                moves.sort_by_cached_key(|m| -search::punteggio_mossa(&board, m));
                
                if let Some(best_move) = moves.first() {
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
    if let Some(m) = mosse.iter().find(|m| m.to_string() == mossa_str) {
        board.esegui_mossa(m, None);
    }
}