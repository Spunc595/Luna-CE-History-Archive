// src/evaluation/mod.rs

pub mod params;
pub mod pst;
use crate::board::{Scacchiera, Colore, Pezzo};
use crate::nnue::Network;

pub fn evaluate_classical(s: &Scacchiera) -> i32 {
    let phase = pst::calcola_fase(s) as i32;
    let score = pst::valuta_pst(s) as i32 + evaluate_dynamic_bonus(s, phase);
    
    if s.turno == Colore::Bianco { score } else { -score }
}

pub fn evaluate(s: &Scacchiera, _net: &Network) -> i32 {
    evaluate_classical(s)
}

fn evaluate_dynamic_bonus(s: &Scacchiera, phase: i32) -> i32 {
    let mut bonus = 0;
    
    for color in [Colore::Bianco, Colore::Nero] {
        let mut side_score = 0;
        let is_white = color == Colore::Bianco;
        
        // --- 1. SVILUPPO PEZZI MINORI ---
        // Penalità se Cavalli e Alfieri sono ancora in casa in apertura
        if phase < 100 {
            let backrank_mask: u64 = if is_white { 0x00000000000000FF } else { 0xFF00000000000000 };
            let minor_pieces = s.pezzi[Pezzo::Cavallo.indice()] | s.pezzi[Pezzo::Alfiere.indice()];
            let undeveloped = (minor_pieces & s.colori[color.indice()] & backrank_mask).count_ones();
            side_score += undeveloped as i32 * params::DEVELOPMENT_PENALTY as i32;
        }

        // --- 2. PEDONI PASSATI ---
        let mut pawns = s.pezzi[0] & s.colori[color.indice()];
        while pawns != 0 {
            let sq = pawns.trailing_zeros() as usize;
            if s.is_passed_pawn(sq, color) {
                let rank = if is_white { sq / 8 } else { 7 - (sq / 8) };
                let p_bonus = (params::PASSED_PAWN_BONUS[rank] as i32 * (256 - phase) + 
                               params::PASSED_PAWN_ENDGAME_BONUS[rank] as i32 * phase) / 256;
                side_score += p_bonus;
            }
            pawns &= pawns - 1;
        }

        // --- 3. AVAMPOSTI CAVALLI ---
        let mut knights = s.pezzi[1] & s.colori[color.indice()];
        while knights != 0 {
            let sq = knights.trailing_zeros() as usize;
            if s.is_outpost_square(sq, color) {
                side_score += params::KNIGHT_OUTPOST_BONUS as i32;
            }
            knights &= knights - 1;
        }

        if is_white { bonus += side_score; } else { bonus -= side_score; }
    }
    bonus
}