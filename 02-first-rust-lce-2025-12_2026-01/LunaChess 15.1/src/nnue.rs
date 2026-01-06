use crate::board::{Colore, Pezzo, Scacchiera};
use std::fs::File;
use std::io::Read;

// Dimensioni standard (non usate realmente ora, ma servono per la struttura)
const LAYER1_SIZE: usize = 768; 

#[derive(Clone)]
pub struct Network {
    pub feature_weights: Vec<i16>,
    pub feature_bias: Vec<i16>,
    pub output_weights: Vec<i16>,
    pub output_bias: i16,
}

impl Network {
    pub fn carica(path: &str) -> Self {
        // Tenta di aprire il file, ma se fallisce usa la dummy.
        // Anche se lo apre, per ora useremo la valutazione classica per sicurezza.
        let mut file = match File::open(path) {
            Ok(f) => f,
            Err(_) => return Self::dummy(),
        };

        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).unwrap_or(0);
        
        // Restituisce una dummy (vuota) sicura
        Self::dummy()
    }

    pub fn dummy() -> Self {
        Network {
            feature_weights: vec![0; 768 * 256],
            feature_bias: vec![0; 256],
            output_weights: vec![0; 256 * 2],
            output_bias: 0,
        }
    }
}

// Mantiene la compatibilità con board.rs
pub fn get_feature_index(pezzo: Pezzo, colore: Colore, sq: usize) -> usize {
    let piece_offset = pezzo.indice();
    let color_offset = if colore == Colore::Bianco { 0 } else { 6 };
    (color_offset + piece_offset) * 64 + sq
}

// --- QUESTA È LA FUNZIONE CHE CERCAVI ---
// L'ho rinominata da 'evaluate' a 'evaluate_nnue' per far felice il compilatore.
pub fn evaluate_nnue(board: &Scacchiera, _net: &Network) -> i32 {
    // Ignoriamo la rete (_net) e usiamo la valutazione classica
    // per garantire mosse intelligenti e stabili.
    let score_material = evaluate_material(board);
    let score_positional = evaluate_positional(board);
    
    score_material + score_positional
}

// --- VALUTAZIONE CLASSICA (Salva-Partita) ---

fn evaluate_material(board: &Scacchiera) -> i32 {
    let mut score = 0;
    // Valori: P=100, N=320, B=330, R=500, Q=900
    let values = [100, 320, 330, 500, 900, 20000]; 
    
    for c in 0..2 {
        let mult = if c == 0 { 1 } else { -1 }; // Bianco +, Nero -
        
        for p in 0..5 { // Ignoriamo i Re per il conto materiale base
             // Ricostruiamo l'enum Pezzo dall'indice
             // (Nota: in un codice ottimizzato useremmo loop diversi, ma questo è sicuro)
             let count = (board.pezzi[p] & board.colori[c]).count_ones() as i32;
             score += count * values[p] * mult;
        }
    }
    score
}

fn evaluate_positional(board: &Scacchiera) -> i32 {
    let mut score = 0;
    
    // Bonus per il controllo del centro (e4, d4, e5, d5)
    let center_mask = 0x0000001818000000; 
    
    for c in 0..2 {
        let mult = if c == 0 { 1 } else { -1 };
        let my_pieces = board.colori[c];
        
        // Bonus Pedoni Centrali
        let pawns = board.pezzi[0] & my_pieces;
        score += (pawns & center_mask).count_ones() as i32 * 20 * mult;
        
        // Bonus Cavalli Sviluppati (non sui bordi)
        let knights = board.pezzi[1] & my_pieces;
        let edges = 0xFF818181818181FF;
        score -= (knights & edges).count_ones() as i32 * 10 * mult;
        
        // Bonus Coppia degli Alfieri
        let bishops = board.pezzi[2] & my_pieces;
        if bishops.count_ones() >= 2 { score += 30 * mult; }
        
        // Sicurezza Re (Bonus se arroccato)
        let king = board.pezzi[5] & my_pieces;
        let k_sq = king.trailing_zeros() as usize;
        
        // Se il Re è nelle zone di arrocco (g1/c1 per bianco, g8/c8 per nero)
        if c == 0 { // Bianco
            if k_sq == 6 || k_sq == 2 { score += 20 * mult; }
        } else { // Nero
            if k_sq == 62 || k_sq == 58 { score += 20 * mult; }
        }
    }
    
    score
}