use std::io::{self, BufRead};
use std::sync::{Arc, RwLock};
use std::time::Instant;

use crate::board::{Scacchiera, Colore};
use crate::search::Motore;
use crate::nnue::Network;
use crate::transposition::TranspositionTable;

mod board;
mod movegen;
mod attacks;
mod evaluation; // Questo carica evaluation/mod.rs
mod search;
mod transposition;
mod nnue;
mod move_ordering;

fn main() {
    crate::attacks::init();
    
    // Carica NNUE. Se fallisce, il programma non crasha subito ma evaluation userà PST.
    let nnue_net = Arc::new(Network::carica("luna_net.nnue"));
    let tt = Arc::new(RwLock::new(TranspositionTable::new(128)));
    
    let mut scacchiera = Scacchiera::nuova();
    scacchiera.inizializza_acc_nnue(&nnue_net);

    let mut motore = Motore {
        nodi: 0,
        start_time: Instant::now(),
        limit_ms: 0,
        stopped: false,
        tt: tt.clone(),
        nnue: nnue_net.clone(),
    };

    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let msg = line.unwrap();
        let parts: Vec<&str> = msg.split_whitespace().collect();
        if parts.is_empty() { continue; }

        match parts[0] {
            "uci" => {
                println!("id name Luna Chess 14.6");
                println!("id author Gemini & Daniele");
                println!("option name Hash type spin default 128 min 1 max 2048");
                println!("uciok");
            }
            "isready" => println!("readyok"),
            "ucinewgame" => {
                tt.write().unwrap().clear();
                scacchiera = Scacchiera::nuova();
                scacchiera.inizializza_acc_nnue(&nnue_net);
            }
            "position" => {
                if parts.len() > 1 {
                    if parts[1] == "startpos" {
                        scacchiera = Scacchiera::nuova();
                    } else if parts[1] == "fen" {
                        let fen = parts[2..8].join(" ");
                        if let Ok(s) = Scacchiera::da_fen(&fen) { scacchiera = s; }
                    }
                    
                    if let Some(pos) = parts.iter().position(|&x| x == "moves") {
                        for m_str in &parts[pos + 1..] {
                            let mut moves = Vec::new();
                            crate::movegen::genera_tutte_mosse_pseudo_legali(&scacchiera, &mut moves);
                            if let Some(m) = moves.into_iter().find(|m| m.to_string() == *m_str) {
                                scacchiera.esegui_mossa(&m, &nnue_net);
                            }
                        }
                    }
                    scacchiera.inizializza_acc_nnue(&nnue_net);
                }
            }
            "go" => {
                let mut time_limit = 2000;
                let search_term = if scacchiera.turno == Colore::Bianco { "wtime" } else { "btime" };
                
                if let Some(idx) = parts.iter().position(|&x| x == search_term) {
                    if let Some(t_str) = parts.get(idx + 1) {
                        if let Ok(t) = t_str.parse::<u64>() {
                            time_limit = t / 40; 
                        }
                    }
                }
                // Il motore stamperà "bestmove" da solo alla fine
                motore.trova_mossa_migliore(&mut scacchiera, time_limit);
            }
            "quit" => break,
            _ => {}
        }
    }
}