mod board;
mod movegen;
mod attacks;
mod zobrist;
mod nnue;
mod evaluation;
mod search;
mod tt;
mod book;

use std::io::{self, BufRead, Write};
use crate::board::{Scacchiera, Colore};
use crate::zobrist::get_zobrist_keys;
use crate::nnue::LunaNNUE;
use crate::evaluation::evaluate;
use crate::search::{iterative_deepening, SearchInfo};
use crate::tt::TranspositionTable;
use crate::book::OpeningBook;

fn main() {
    // 1. INIZIALIZZAZIONE DEI SOTTOSISTEMI GLOBALI
    // Carica le chiavi Zobrist deterministiche per il calcolo degli hash[cite: 8].
    let z = get_zobrist_keys();
    
    // Tenta il caricamento del file binario della rete neurale in modalità provvisoria (safe)[cite: 8].
    let nnue = LunaNNUE::load("luna_52000_safe.nnue"); 
    
    // Tenta il caricamento del libro delle aperture testuale ("book.txt")[cite: 8].
    let mut book = OpeningBook::load("book.txt");
    match book {
        Some(_) => println!("✅ Book: Attivo e carico!"),
        None => println!("⚠️ Book: File 'book.txt' non trovato. Si userà solo la rete."),
    }

    // Alloca la Transposition Table iniziale con una dimensione predefinita di 256 MB[cite: 8].
    let mut tt = TranspositionTable::new(256);
    
    // Configura la scacchiera sulla posizione di partenza standard[cite: 8].
    let mut s = Scacchiera::new_iniziale(z);

    // Messaggio di benvenuto UCI standard
    println!("Luna Engine v7.2 - Istruita Ready");
    io::stdout().flush().unwrap();

    // 2. CICLO DI ASCOLTO DEI COMANDI (UCI LOOP)
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() { continue; }

        match parts[0] {
            // Identificazione del motore e delle opzioni configurabili[cite: 8]
            "uci" => {
                println!("id name Luna_Hybrid_7.2");
                println!("id author Daniele");
                println!("option name Hash type spin default 256 min 1 max 1024");
                println!("uciok");
            }
            
            // Sincronizzazione con l'interfaccia grafica (GUI)[cite: 8]
            "isready" => println!("readyok"),
            
            // Gestione dei parametri modificabili dinamici (es. dimensione della cache TT)[cite: 8]
            "setoption" => {
                if parts.len() >= 5 && parts[2] == "Hash" {
                    if let Ok(new_size) = parts[4].parse::<usize>() {
                        tt = TranspositionTable::new(new_size);
                    }
                }
            }
            
            // Configurazione dello stato della scacchiera[cite: 8]
            "position" => {
                if parts.len() > 1 {
                    if parts[1] == "startpos" {
                        s = Scacchiera::new_iniziale(z);
                    } else if parts[1] == "fen" {
                        // Isola la stringa FEN escludendo l'eventuale sequenza di mosse successive[cite: 8].
                        let m_idx = parts.iter().position(|&p| p == "moves").unwrap_or(parts.len());
                        let fen_str = parts[2..m_idx].join(" ");
                        s = Scacchiera::from_fen(&fen_str, z);
                    }
                    
                    // Se sono presenti mosse storiche a corredo, le esegue in sequenza per aggiornare la scacchiera[cite: 8].
                    if let Some(m_idx) = parts.iter().position(|&p| p == "moves") {
                        for &m_str in &parts[m_idx + 1..] {
                            let moves = s.genera_mosse_legali(z);
                            for m in moves {
                                if m.to_uci() == m_str { 
                                    s.esegui_mossa(&m, z); 
                                    break; 
                                }
                            }
                        }
                    }
                }
            }
            
            // Avvio della routine di calcolo/ricerca della mossa migliore[cite: 8]
            "go" => {
                let mut mossa_trovata = false;
                
                // --- CONSULTAZIONE LIBRO DELLE APERTURE ---
                if let Some(ref mut b) = book {
                    if let Some(book_move) = b.get_move(&mut s) {
                        println!("bestmove {}", book_move.to_uci());
                        mossa_trovata = true;
                    }
                }

                // --- CALCOLO ITERATIVO DEL MOTORE ---
                if !mossa_trovata {
                    // Valori di riserva in caso di comando "go" privo di vincoli temporali[cite: 8].
                    let mut depth = 12;
                    let mut movetime: u128 = 5000;
                    
                    // Parsing dei parametri di tempo e profondità della mossa[cite: 8]
                    for i in (1..parts.len()).step_by(2) {
                        if i + 1 >= parts.len() { break; }
                        match parts[i] {
                            // Gestione tempo residuo: alloca il 4% del tempo totale rimanente per mossa (tempo / 25)[cite: 8].
                            "wtime" if s.turno == Colore::Bianco => movetime = parts[i+1].parse::<u128>().unwrap_or(5000) / 25,
                            "btime" if s.turno == Colore::Nero => movetime = parts[i+1].parse::<u128>().unwrap_or(5000) / 25,
                            "depth" => depth = parts[i+1].parse().unwrap_or(12),
                            "movetime" => movetime = parts[i+1].parse().unwrap_or(5000),
                            _ => {}
                        }
                    }

                    // Inizializza la struttura di controllo del tempo e lancia la ricerca iterativa[cite: 8].
                    let mut info = SearchInfo::new(movetime, depth as i32);
                    let (mut best_m, _) = iterative_deepening(&mut s, &mut info, &mut tt, &z, nnue.as_ref());
                    
                    // Controllo di integrità: verifica che la mossa restituita sia presente tra le legali correnti[cite: 8].
                    let legali = s.genera_mosse_legali(z);
                    if !legali.iter().any(|m| m.data == best_m.data) {
                        if !legali.is_empty() { 
                            best_m = legali[0]; // Fallback di emergenza sulla prima mossa legale[cite: 8]
                        }
                    }
                    println!("bestmove {}", best_m.to_uci());
                }
            }
            
            // Arresto immediato dell'eseguibile[cite: 8]
            "quit" => break,
            
            // Ispezione statica del punteggio della posizione corrente[cite: 8]
            "eval" => {
                 // Utilizza la valutazione della rete neurale se disponibile, altrimenti ricade su quella classica[cite: 8].
                 let score = if let Some(ref n) = nnue { n.evaluate(&s) } else { evaluate(&s) };
                 println!("Evaluation: {} cp", score);
            }
            _ => {}
        }
        io::stdout().flush().unwrap();
    }
}