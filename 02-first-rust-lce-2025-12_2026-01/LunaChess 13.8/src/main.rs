use std::io::{self, BufRead};
use std::sync::Arc;
use std::time::Instant;

use crate::board::{Scacchiera, Colore};
use crate::nnue::Network;

// --- DEFINIZIONE MODULI ---
// Questi file devono esistere nella cartella src/
mod board;
mod movegen;
mod attacks;
mod search;
mod move_ordering;
mod nnue;
mod transposition; // Se non hai questo file, commenta questa riga

// evaluation è una cartella, quindi cercherà src/evaluation/mod.rs
mod evaluation; 

fn main() {
    // 1. Inizializzazione Tabelle di Attacco
    crate::attacks::init();
    
    // 2. Caricamento NNUE
    // Assicurati che il file .nnue sia nella stessa cartella dell'eseguibile o del progetto
    println!("Caricamento rete neurale...");
    let nnue_path = "luna_net.nnue"; 
    
    // Se il caricamento fallisce, creiamo una rete vuota per non far crashare, 
    // ma il motore giocherà a caso se evaluate() dipende solo dalla rete.
    let nnue_net = match std::fs::File::open(nnue_path) {
        Ok(_) => Arc::new(Network::carica(nnue_path)),
        Err(_) => {
            eprintln!("ATTENZIONE: File {} non trovato! Assicurati di averlo.", nnue_path);
            // In un caso reale dovresti uscire o usare una valutazione fallback.
            // Per ora usiamo una rete non inizializzata (rischio crash se usata).
            // Idealmente: panic!("Rete non trovata");
            std::process::exit(1);
        }
    };
    println!("Rete caricata.");

    let mut scacchiera = Scacchiera::nuova();
    scacchiera.inizializza_acc_nnue(&nnue_net);

    let stdin = io::stdin();
    let mut buffer = String::new();

    // Loop principale UCI
    for line in stdin.lock().lines() {
        let msg = line.unwrap();
        let parts: Vec<&str> = msg.split_whitespace().collect();
        if parts.is_empty() { continue; }

        match parts[0] {
            "uci" => {
                println!("id name Luna Chess 1.0");
                println!("id author Gemini & Daniele");
                println!("uciok");
            }
            "isready" => println!("readyok"),
            "ucinewgame" => {
                scacchiera = Scacchiera::nuova();
                scacchiera.inizializza_acc_nnue(&nnue_net);
            }
            "position" => {
                // Esempio: position startpos moves e2e4 e7e5
                if parts.len() > 1 {
                    // 1. Reset Scacchiera
                    if parts[1] == "startpos" {
                        scacchiera = Scacchiera::nuova();
                    } else if parts[1] == "fen" {
                        // Unisci le parti del FEN (che contengono spazi)
                        // position fen rnbqk... w KQkq - 0 1 moves ...
                        let mut fen_parts = Vec::new();
                        for p in &parts[2..] {
                            if *p == "moves" { break; }
                            fen_parts.push(*p);
                        }
                        let fen = fen_parts.join(" ");
                        if let Ok(s) = Scacchiera::da_fen(&fen) { 
                            scacchiera = s; 
                        }
                    }
                    
                    // Ricalcola accumulatore da zero per sicurezza sulla nuova posizione base
                    scacchiera.inizializza_acc_nnue(&nnue_net);

                    // 2. Applicazione Mosse
                    if let Some(pos) = parts.iter().position(|&x| x == "moves") {
                        for m_str in &parts[pos + 1..] {
                            // Generiamo le mosse legali per trovare quella corrispondente alla stringa
                            let moves = scacchiera.genera_mosse();
                            
                            if let Some(m) = moves.into_iter().find(|m| m.to_string() == *m_str) {
                                // Eseguiamo la mossa aggiornando incrementalmente l'NNUE
                                scacchiera.esegui_mossa(&m, &nnue_net);
                            } else {
                                eprintln!("Mossa non valida o illegale ricevuta: {}", m_str);
                            }
                        }
                    }
                }
            }
            "go" => {
                // Parsing parametri tempo (wtime, btime, winc, binc)
                let mut wtime: u128 = 0;
                let mut btime: u128 = 0;
                let mut winc: u128 = 0;
                let mut binc: u128 = 0;
                let mut movestogo: u128 = 30; // Default

                for i in 1..parts.len() {
                    match parts[i] {
                        "wtime" => if let Some(v) = parts.get(i+1) { wtime = v.parse().unwrap_or(0); },
                        "btime" => if let Some(v) = parts.get(i+1) { btime = v.parse().unwrap_or(0); },
                        "winc" => if let Some(v) = parts.get(i+1) { winc = v.parse().unwrap_or(0); },
                        "binc" => if let Some(v) = parts.get(i+1) { binc = v.parse().unwrap_or(0); },
                        "movestogo" => if let Some(v) = parts.get(i+1) { movestogo = v.parse().unwrap_or(30); },
                        _ => {}
                    }
                }

                // Gestione Tempo
                let time_available = if scacchiera.turno == Colore::Bianco { wtime } else { btime };
                let inc = if scacchiera.turno == Colore::Bianco { winc } else { binc };
                
                // Logica semplice: usa il 5% del tempo + incremento, o 1 secondo fisso se infinito
                let mut time_limit = if time_available > 0 {
                    (time_available / 20) + (inc / 2)
                } else {
                    1000 // Default 1 secondo per mossa fissa o analisi indefinita
                };
                
                // Sicurezza: lasciamo almeno 50ms per trasmettere la mossa
                if time_available > 0 && time_limit >= time_available {
                    time_limit = time_available.saturating_sub(50);
                }

                // Chiamata alla ricerca
                // Nota: depth 64 è praticamente infinito, si fermerà per il time_limit
                let best_move = crate::search::search_position(&mut scacchiera, 64, time_limit, &nnue_net);

                if let Some(mv) = best_move {
                    println!("bestmove {}", mv);
                } else {
                    // Caso raro (stallo o matto immediato senza mosse, o bug)
                    println!("bestmove 0000"); 
                }
            }
            "quit" => break,
            "d" | "display" => {
                // Utile per debuggare visualmente
                println!("Fen: ... (implementare display board se vuoi)");
                println!("Valutazione statica: {}", crate::evaluation::evaluate(&scacchiera, &nnue_net));
            },
            _ => {}
        }
    }
}