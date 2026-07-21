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
use crate::evaluation::{evaluate, EvalParams}; // Importato EvalParams
use crate::search::{iterative_deepening, SearchInfo, MAX_PLY};
use crate::tt::TranspositionTable;
use crate::book::OpeningBook;

/// Percorso di `luna.nnue` accanto all'ESEGUIBILE, non alla working
/// directory del processo. Un GUI/wrapper UCI (es. lichess-bot) può
/// lanciare il motore da una cartella qualunque: un percorso relativo puro
/// ("luna.nnue") verrebbe risolto contro quella working directory, non
/// contro la cartella dove si trova effettivamente il file, che l'utente
/// mette sempre a fianco del binario. Se per qualunque motivo non si riesce
/// a determinare il percorso dell'eseguibile, si ricade sul nome relativo
/// semplice (comportamento precedente), non su un errore fatale.
fn nnue_path_next_to_exe() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join("luna.nnue")))
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| "luna.nnue".to_string())
}

fn main() {
    let z = get_zobrist_keys();
    let nnue_path = nnue_path_next_to_exe();
    println!("info string Cerco NNUE in: {}", nnue_path);
    let nnue = LunaNNUE::load(&nnue_path);
    if nnue.is_none() {
        println!("info string NNUE non caricata: '{}' non trovato o non compatibile (uso valutazione classica PST).", nnue_path);
    }

    // Inizializziamo i parametri di valutazione
    let params = EvalParams::default(); 

    let mut book = OpeningBook::load("book.bin");
    match book {
        Some(_) => println!("✅ Book: Attivo e carico!"),
        None => println!("⚠️ Book: File 'book.bin' non trovato."),
    }

    let mut tt = TranspositionTable::new(256);
    let mut s = Scacchiera::new_iniziale(z);
    s.refresh_nnue(nnue.as_ref());

    println!("Luna CE v2.0.0");
    io::stdout().flush().unwrap();

    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() { continue; }

        match parts[0] {
            "uci" => {
                println!("id name Luna CE v2.0.0");
                println!("id author Daniele Marpino");
                println!("option name Hash type spin default 256 min 1 max 1024");
                println!("uciok");
            }
            "isready" => println!("readyok"),
            "setoption" => {
                if parts.len() >= 5 && parts[2] == "Hash" {
                    if let Ok(new_size) = parts[4].parse::<usize>() {
                        tt = TranspositionTable::new(new_size);
                    }
                }
            }
            "position" => {
                if parts.len() > 1 {
                    if parts[1] == "startpos" {
                        s = Scacchiera::new_iniziale(z);
                    } else if parts[1] == "fen" {
                        let m_idx = parts.iter().position(|&p| p == "moves").unwrap_or(parts.len());
                        let fen_str = parts[2..m_idx].join(" ");
                        s = Scacchiera::from_fen(&fen_str, z);
                    }
                    // Ricalcolo completo una sola volta per la nuova posizione;
                    // da qui in poi esegui_mossa mantiene l'accumulatore
                    // aggiornato in modo incrementale.
                    s.refresh_nnue(nnue.as_ref());
                    if let Some(m_idx) = parts.iter().position(|&p| p == "moves") {
                        for &m_str in &parts[m_idx + 1..] {
                            let moves = s.genera_mosse_legali(z);
                            for m in moves {
                                if m.to_uci() == m_str { s.esegui_mossa(&m, z, nnue.as_ref()); break; }
                            }
                        }
                    }
                }
            }
            "go" => {
                let mut mossa_trovata = false;
                if let Some(ref mut b) = book {
                    if let Some(book_move) = b.get_move(&mut s) {
                        println!("bestmove {}", book_move.to_uci());
                        mossa_trovata = true;
                    }
                }

                if !mossa_trovata {
                    // Default = MAX_PLY, non un numero arbitrario di ply:
                    // in un comando "go wtime/btime" standard (il caso
                    // normale per qualunque GUI/wrapper UCI, incl.
                    // lichess-bot) non arriva mai il token "depth", quindi
                    // questo valore di default è a tutti gli effetti quello
                    // sempre in uso in partita. Fissarlo a un numero basso
                    // (come il precedente 12) significa fermare la ricerca
                    // molto prima di aver esaurito il tempo assegnato,
                    // sprecando gran parte del budget: con MAX_PLY come
                    // tetto, è la logica di stop basata sul tempo in
                    // `iterative_deepening` (soft/hard limit, stabilità
                    // della best move) a decidere davvero quando fermarsi,
                    // com'è sempre stata pensata per fare.
                    let mut depth = MAX_PLY as i32;
                    let mut movetime: u128 = 5000;

                    for i in (1..parts.len()).step_by(2) {
                        if i + 1 >= parts.len() { break; }
                        match parts[i] {
                            "wtime" if s.turno == Colore::Bianco => movetime = parts[i+1].parse::<u128>().unwrap_or(5000) / 25,
                            "btime" if s.turno == Colore::Nero => movetime = parts[i+1].parse::<u128>().unwrap_or(5000) / 25,
                            "depth" => depth = parts[i+1].parse().unwrap_or(MAX_PLY as i32),
                            "movetime" => movetime = parts[i+1].parse().unwrap_or(5000),
                            _ => {}
                        }
                    }

                    let mut info = SearchInfo::new(movetime, depth as i32);
                    // Passiamo &params qui
                    let (mut best_m, _) = iterative_deepening(&mut s, &mut info, &mut tt, &z, nnue.as_ref(), &params);
                    
                    let legali = s.genera_mosse_legali(z);
                    if !legali.iter().any(|m| m.data == best_m.data) {
                        if !legali.is_empty() { best_m = legali[0]; }
                    }
                    println!("bestmove {}", best_m.to_uci());
                }
            }
            "quit" => break,
            "eval" => {
                 // Passiamo &params qui
                 let score = if let Some(ref n) = nnue { n.evaluate_from_accumulator(&s.nnue_acc, s.turno == Colore::Bianco) } else { evaluate(&s, &params) };
                 println!("Evaluation: {} cp", score);
            }
            _ => {}
        }
        io::stdout().flush().unwrap();
    }
}