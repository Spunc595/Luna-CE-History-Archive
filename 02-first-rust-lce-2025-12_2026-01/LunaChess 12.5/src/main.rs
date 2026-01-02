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
    crate::attacks::init();

    // -----------------------------------------------------------------
    // FASE 2: Setup Motore
    // -----------------------------------------------------------------
    // 64MB è una buona dimensione standard per la Transposition Table
    let tt = Arc::new(RwLock::new(TranspositionTable::new(64))); 
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
                println!("id author Gemini & Thought Partner");
                println!("uciok");
            }
            "isready" => println!("readyok"),
            "ucinewgame" => {
                if let Ok(mut tt_write) = tt.write() {
                    tt_write.clear();
                }
                board = Scacchiera::nuova();
                // Reset delle euristiche della ricerca (Killers e History)
                motore = Motore::nuovo(tt.clone());
            }
            "position" => {
                if parts.len() < 2 { continue; }
                
                let mut moves_idx = None;
                
                if parts[1] == "startpos" {
                    board = Scacchiera::nuova();
                    moves_idx = parts.iter().position(|&x| x == "moves");
                } else if parts[1] == "fen" {
                    let fen_end = parts.iter().position(|&x| x == "moves").unwrap_or(parts.len());
                    let fen_string = parts[2..fen_end].join(" ");
                    match Scacchiera::da_fen(&fen_string) {
                        Ok(b) => board = b,
                        Err(e) => eprintln!("Errore FEN: {}", e),
                    }
                    moves_idx = parts.iter().position(|&x| x == "moves");
                }

                if let Some(pos) = moves_idx {
                    if pos + 1 < parts.len() {
                        applica_mosse(&mut board, &parts[pos + 1..]);
                    }
                }
                
                check_hash_consistency(&board);
            }
            "go" => {
                let mut time_left: u64 = 300000; // 5 minuti default
                let mut inc: u64 = 0;
                
                // Parsing UCI per il tempo
                for i in 1..parts.len() {
                    let is_white = board.turno() == Colore::Bianco;
                    if (is_white && parts[i] == "wtime") || (!is_white && parts[i] == "btime") {
                        if let Ok(t) = parts.get(i+1).unwrap_or(&"0").parse::<u64>() {
                            time_left = t;
                        }
                    }
                    if (is_white && parts[i] == "winc") || (!is_white && parts[i] == "binc") {
                        if let Ok(incremental) = parts.get(i+1).unwrap_or(&"0").parse::<u64>() {
                            inc = incremental;
                        }
                    }
                }

                // Avvia la ricerca
                if let Some(mossa) = motore.trova_mossa_migliore(&mut board, time_left, inc) {
                    println!("bestmove {}", mossa);
                }
            }
            "eval" => {
                // Comando di debug utile per vedere il punteggio statico
                let score = evaluation::valuta_posizione(&board);
                println!("static evaluation: {} cp", score);
            }
            "quit" => break,
            _ => {}
        }
    }
}

/// Applica una sequenza di mosse in formato stringa (es. "e2e4", "e7e5") alla scacchiera
fn applica_mosse(s: &mut Scacchiera, mosse_str: &[&str]) {
    for m_str in mosse_str {
        let mut pseudo = Vec::with_capacity(64);
        movegen::genera_tutte_mosse_pseudo_legali(s, &mut pseudo);
        
        let mut trovata = false;
        for m in pseudo {
            if m.to_string() == *m_str {
                // Verifichiamo che la mossa non lasci il proprio re sotto scacco
                // esegui_mossa restituisce true se la mossa è stata fatta, 
                // ma dobbiamo anche verificare la legalità effettiva
                if s.esegui_mossa(&m) {
                    // Se dopo aver fatto la mossa il nostro re (che ora è il turno opposto)
                    // è sotto scacco, allora la mossa era illegale
                    if s.re_in_scacco(s.turno().opposto()) {
                        s.annulla_mossa();
                        continue;
                    }
                    trovata = true;
                    break;
                }
            }
        }
        
        if !trovata { 
            eprintln!("!!! MOSSA NON VALIDA O NON TROVATA: {} !!!", m_str); 
        }
    }
}

/// Verifica se l'hash incrementale calcolato durante le mosse
/// coincide con un ricalcolo totale da zero (Zobrist Hashing)
fn check_hash_consistency(s: &Scacchiera) {
    let incremental_hash = s.hash();
    let computed_hash = s.genera_hash_completo();
    if incremental_hash != computed_hash {
        eprintln!("ERRORE COERENZA HASH: Incr: {:016X} != Comp: {:016X}", incremental_hash, computed_hash);
    }
}