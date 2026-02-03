use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use rand::Rng;
use crate::board::{Scacchiera, Mossa}; // IMPORTANTE: Importiamo da board, non ridefiniamo qui!
use crate::zobrist::ZobristKeys;

/// Rappresenta il libro delle aperture in memoria
pub struct OpeningBook {
    /// Mappa da FEN (o hash) a lista di mosse in notazione UCI
    entries: HashMap<String, Vec<String>>,
}

impl OpeningBook {
    pub fn new() -> Self {
        OpeningBook {
            entries: HashMap::new(),
        }
    }

    /// Carica il libro da un file di testo (formato: FEN mossa)
    pub fn load(path: &str) -> Option<Self> {
        let file = File::open(path).ok()?;
        let reader = BufReader::new(file);
        let mut entries = HashMap::new();

        for line in reader.lines() {
            if let Ok(l) = line {
                let parts: Vec<&str> = l.split_whitespace().collect();
                if parts.len() >= 2 {
                    // Ricostruisce la parte FEN (tutto tranne l'ultima parola che è la mossa)
                    let fen = parts[0..parts.len()-1].join(" ");
                    let move_str = parts.last().unwrap().to_string();
                    
                    entries.entry(fen).or_insert_with(Vec::new).push(move_str);
                }
            }
        }
        
        if entries.is_empty() { return None; }
        Some(OpeningBook { entries })
    }

    /// Cerca una mossa nel libro per la posizione corrente
    pub fn get_move(&self, board: &Scacchiera) -> Option<Mossa> {
        // Usa la funzione to_fen definita in board.rs
        let fen_full = board.to_fen();
        
        // Semplifichiamo il FEN per il libro (prendiamo solo pezzi, turno, castling, ep)
        let parts: Vec<&str> = fen_full.split_whitespace().collect();
        let fen_key = parts[0..4].join(" "); 

        if let Some(candidates) = self.entries.get(&fen_key) {
            if candidates.is_empty() { return None; }
            
            // Genera le mosse legali per vedere quale corrisponde alla stringa del libro
            let legali = board.genera_mosse_legali(&ZobristKeys::default());
            
            // Filtra: teniamo solo le mosse legali che sono presenti nel libro
            let book_moves: Vec<Mossa> = legali.into_iter()
                .filter(|m| candidates.contains(&m.to_uci()))
                .collect();

            // Scegli una mossa a caso tra quelle disponibili
            if !book_moves.is_empty() {
                let mut rng = rand::thread_rng();
                let idx = rng.gen_range(0..book_moves.len());
                return Some(book_moves[idx]);
            }
        }
        
        None
    }
}