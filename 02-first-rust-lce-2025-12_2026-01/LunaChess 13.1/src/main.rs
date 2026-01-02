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
mod evaluation;
mod search;
mod transposition;
mod nnue;
mod move_ordering;

fn main() {
    // 1. Inizializzazione tabelle precalcolate per le mosse (Bitboards)
    crate::attacks::init();

    // 2. Caricamento della rete neurale NNUE
    // Nota: Il file "luna_net.nnue" deve trovarsi nella stessa cartella dell'eseguibile
    let nnue_net = Arc::new(Network::carica("luna_net.nnue"));
    
    // 3. Inizializzazione Transposition Table (128 MB di default)
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
                println!("id name Luna Chess 14.5");
                println!("id author Gemini & Daniele");
                println!("option name Hash type spin default 128 min 1 max 2048");
                println!("uciok");
            }
            "isready" => println!("readyok"),
            "ucinewgame" => {
                let mut tt_lock = tt.write().unwrap();
                tt_lock.clear(); 
                scacchiera = Scacchiera::nuova();
                scacchiera.inizializza_acc_nnue(&nnue_net);
            }
            "position" => {
                if parts.len() > 1 {
                    if parts[1] == "startpos" {
                        scacchiera = Scacchiera::nuova();
                    } else if parts[1] == "fen" {
                        let fen_str = parts[2..8].join(" ");
                        if let Ok(s) = Scacchiera::da_fen(&fen_str) {
                            scacchiera = s;
                        }
                    }
                    
                    // Fondamentale: ricalcola l'accumulatore per la nuova posizione
                    scacchiera.inizializza_acc_nnue(&nnue_net);

                    // Gestione delle mosse: "position startpos moves e2e4 e7e5..."
                    if let Some(moves_idx) = parts.iter().position(|&x| x == "moves") {
                        for m_str in &parts[moves_idx + 1..] {
                            let mut moves = Vec::new();
                            crate::movegen::genera_tutte_mosse_pseudo_legali(&scacchiera, &mut moves);
                            if let Some(m) = moves.into_iter().find(|m| m.to_string() == *m_str) {
                                scacchiera.esegui_mossa(&m, &nnue_net);
                            }
                        }
                    }
                }
            }
            "go" => {
                let mut time_limit = 2000; // Tempo di fallback (2 secondi)
                
                // Cerca il tempo rimanente per il colore corrente (wtime o btime)
                let search_term = if scacchiera.turno == Colore::Bianco { "wtime" } else { "btime" };
                let inc_term = if scacchiera.turno == Colore::Bianco { "winc" } else { "binc" };

                let mut remaining_time = 0;
                let mut increment = 0;

                if let Some(idx) = parts.iter().position(|&x| x == search_term) {
                    if let Some(t_str) = parts.get(idx + 1) {
                        remaining_time = t_str.parse::<u64>().unwrap_or(0);
                    }
                }
                
                if let Some(idx) = parts.iter().position(|&x| x == inc_term) {
                    if let Some(i_str) = parts.get(idx + 1) {
                        increment = i_str.parse::<u64>().unwrap_or(0);
                    }
                }

                // Gestione tempo UCI: usa il tempo totale / 40 + l'incremento
                if remaining_time > 0 {
                    time_limit = (remaining_time / 40) + increment;
                }

                // Avvia la ricerca Alpha-Beta con Iterative Deepening
                if let Some(m) = motore.trova_mossa_migliore(&mut scacchiera, time_limit) {
                    println!("bestmove {}", m);
                }
            }
            "quit" => break,
            _ => {}
        }
    }
}