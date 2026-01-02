mod board;
mod attacks;
mod movegen;
mod search;
mod evaluation;
mod transposition;

use board::{Scacchiera, Colore};
use search::Motore;
use transposition::TranspositionTable;
use std::io::{self, BufRead};
use std::sync::{Arc, RwLock};

fn main() {
    // -----------------------------------------------------------------
    // FASE 1: Inizializzazione Tabelle (CRITICO)
    // -----------------------------------------------------------------
    // Senza questa chiamata, il motore non sa come muovere Cavalli e Pedoni.
    // Questo risolve l'errore "MOSSA NON TROVATA: b8c6".
    crate::attacks::init();

    // -----------------------------------------------------------------
    // FASE 2: Setup Motore
    // -----------------------------------------------------------------
    let tt = Arc::new(RwLock::new(TranspositionTable::new(64))); // 64MB hash
    let mut motore = Motore::nuovo(tt.clone());
    let mut board = Scacchiera::nuova();

    // -----------------------------------------------------------------
    // FASE 3: UCI Loop
    // -----------------------------------------------------------------
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let text = line.unwrap();
        let parts: Vec<&str> = text.split_whitespace().collect();
        if parts.is_empty() { continue; }

        match parts[0] {
            "uci" => {
                println!("id name Luna Chess Pro 1.8");
                println!("id author Gemini");
                println!("uciok");
            }
            "isready" => println!("readyok"),
            "ucinewgame" => {
                tt.write().unwrap().clear();
                board = Scacchiera::nuova();
            }
            "position" => {
                if parts.len() < 2 { continue; }
                
                // 1. Parsing posizione base (startpos o fen)
                let mut moves_idx = None;
                
                if parts[1] == "startpos" {
                    board = Scacchiera::nuova();
                    moves_idx = parts.iter().position(|&x| x == "moves");
                } else if parts[1] == "fen" {
                    // Trova la fine della stringa FEN (o fine riga o inizio "moves")
                    let fen_end = parts.iter().position(|&x| x == "moves").unwrap_or(parts.len());
                    let fen_string = parts[2..fen_end].join(" ");
                    match Scacchiera::da_fen(&fen_string) {
                        Ok(b) => board = b,
                        Err(e) => eprintln!("Errore FEN: {}", e),
                    }
                    moves_idx = parts.iter().position(|&x| x == "moves");
                }

                // 2. Applicazione mosse successive
                if let Some(pos) = moves_idx {
                    if pos + 1 < parts.len() {
                        applica_mosse(&mut board, &parts[pos + 1..]);
                    }
                }
                
                // Debug: Verifica che l'hash sia rimasto coerente
                check_hash_consistency(&board);
            }
            "go" => {
                // CORRETTO: Usiamo u64 per il tempo (compatibile con la funzione di ricerca)
                let mut tempo: u64 = 300000; 
                
                for i in 1..parts.len() {
                    if (board.turno() == Colore::Bianco && parts[i] == "wtime") ||
                       (board.turno() == Colore::Nero && parts[i] == "btime") {
                        // Parsing sicuro a u64
                        if let Ok(t) = parts[i+1].parse::<u64>() {
                            tempo = t;
                        }
                    }
                }

                if let Some(mossa) = motore.trova_mossa_migliore(&mut board, tempo) {
                    println!("bestmove {}", mossa);
                }
            }
            "quit" => break,
            _ => {}
        }
    }
}

fn applica_mosse(s: &mut Scacchiera, mosse_str: &[&str]) {
    for m_str in mosse_str {
        // Genera mosse legali per trovare quella corrispondente alla stringa UCI
        let mut pseudo = Vec::with_capacity(64);
        movegen::genera_tutte_mosse_pseudo_legali(s, &mut pseudo);
        
        let mut trovata = false;
        for m in pseudo {
            if m.to_string() == *m_str {
                if s.esegui_mossa(&m) {
                    trovata = true;
                    break;
                } else {
                    eprintln!("!!! MOSSA ILLEGALE RILEVATA (esegui_mossa false): {} !!!", m_str);
                }
            }
        }
        
        if !trovata { 
            eprintln!("!!! MOSSA NON TROVATA: {} !!!", m_str); 
            eprintln!("Hint: Se è un pedone o un cavallo, controlla attacks::init()");
        }
    }
}

fn check_hash_consistency(s: &Scacchiera) {
    let incremental_hash = s.hash();
    let computed_hash = s.genera_hash_completo();
    if incremental_hash != computed_hash {
        eprintln!("ERRORE HASH: Incr: {:016X} != Comp: {:016X}", incremental_hash, computed_hash);
    }
}