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
    // 1. Inizializzazione Bitboards
    crate::attacks::init();

    // 2. Caricamento della rete neurale
    // Usiamo un caricamento sicuro: se il file manca, il motore lo segnala senza crashare subito
    let nnue_net = Arc::new(Network::carica("luna_net.nnue"));
    
    // 3. Inizializzazione Transposition Table (128 MB)
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
            "isready" => {
                println!("readyok");
            }
            "ucinewgame" => {
                let mut tt_lock = tt.write().unwrap();
                tt_lock.clear(); 
                scacchiera = Scacchiera::nuova();
                scacchiera.inizializza_acc_nnue(&nnue_net);
            }
            "position" => {
                // Il protocollo UCI dice: position [startpos | fen] moves [m1 m2...]
                if parts.len() > 1 {
                    let mut moves_start = 0;
                    
                    if parts[1] == "startpos" {
                        scacchiera = Scacchiera::nuova();
                        moves_start = 2;
                    } else if parts[1] == "fen" {
                        // Ricostruisce la stringa FEN (le successive 6 parti dopo "fen")
                        let fen_str = parts[2..8].join(" ");
                        if let Ok(s) = Scacchiera::da_fen(&fen_str) {
                            scacchiera = s;
                        }
                        moves_start = 8;
                    }

                    scacchiera.inizializza_acc_nnue(&nnue_net);

                    // Cerca la parola "moves" per applicare la sequenza di gioco
                    if let Some(pos) = parts.iter().position(|&x| x == "moves") {
                        for m_str in &parts[pos + 1..] {
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
                let mut time_limit = 2000; // Default
                
                // Gestione avanzata del tempo UCI
                let search_term = if scacchiera.turno == Colore::Bianco { "wtime" } else { "btime" };
                let inc_term = if scacchiera.turno == Colore::Bianco { "winc" } else { "binc" };

                let mut t_rem = 0;
                let mut t_inc = 0;

                if let Some(idx) = parts.iter().position(|&x| x == search_term) {
                    if let Some(val) = parts.get(idx + 1) {
                        t_rem = val.parse::<u64>().unwrap_or(0);
                    }
                }
                if let Some(idx) = parts.iter().position(|&x| x == inc_term) {
                    if let Some(val) = parts.get(idx + 1) {
                        t_inc = val.parse::<u64>().unwrap_or(0);
                    }
                }

                if t_rem > 0 {
                    // Formula standard per non cadere per il tempo
                    time_limit = (t_rem / 30) + (t_inc / 2);
                }

                // Avvia la ricerca. Nota: bestmove deve essere stampato DENTRO trova_mossa_migliore
                // o subito dopo il suo ritorno per garantire la comunicazione con Arena.
                if let Some(m) = motore.trova_mossa_migliore(&mut scacchiera, time_limit) {
                    println!("bestmove {}", m);
                }
            }
            "quit" => break,
            _ => {}
        }
    }
}