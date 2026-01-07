// src/evaluation/mod.rs
pub mod params;
pub mod pst;
use crate::board::{Scacchiera, Colore, Pezzo};
use crate::nnue::Network;

pub fn evaluate(s: &Scacchiera, _net: &Network) -> i32 {
    evaluate_classical(s)
}

pub fn evaluate_classical(s: &Scacchiera) -> i32 {
    let phase = pst::calcola_fase(s) as i32;
    let mut score = pst::valuta_pst(s) as i32;
    
    // FORZA SVILUPPO: Se Cavalli/Alfieri sono fermi in apertura (phase < 150)
    if phase < 150 {
        for color in [Colore::Bianco, Colore::Nero] {
            let is_white = color == Colore::Bianco;
            let backrank_mask: u64 = if is_white { 0x00000000000000FF } else { 0xFF00000000000000 };
            
            let minors = s.pezzi[Pezzo::Cavallo.indice()] | s.pezzi[Pezzo::Alfiere.indice()];
            let undeveloped = (minors & s.colori[color.indice()] & backrank_mask).count_ones();
            
            let penalty = undeveloped as i32 * params::DEVELOPMENT_PENALTY as i32;
            if is_white { score += penalty; } else { score -= penalty; }
        }
    }

    score + evaluate_dynamic_bonus(s, phase)
}

fn evaluate_dynamic_bonus(s: &Scacchiera, phase: i32) -> i32 {
    let mut bonus = 0;
    for color in [Colore::Bianco, Colore::Nero] {
        let mut side_score = 0;
        let is_white = color == Colore::Bianco;
        let king_pos = (s.pezzi[Pezzo::Re.indice()] & s.colori[color.indice()]).trailing_zeros() as usize;

        // Pedoni Passati
        let mut pawns = s.pezzi[0] & s.colori[color.indice()];
        while pawns != 0 {
            let sq = pawns.trailing_zeros() as usize;
            if s.is_passed_pawn(sq, color) {
                let rank = if is_white { sq / 8 } else { 7 - (sq / 8) };
                let p_bonus = (params::PASSED_PAWN_BONUS[rank] as i32 * (256 - phase) + 
                               params::PASSED_PAWN_ENDGAME_BONUS[rank] as i32 * phase) / 256;
                side_score += p_bonus;
                if s.manhattan_distance(sq, king_pos) <= 2 { side_score += params::KING_PROXIMITY_TO_OWN_PASSED_PAWN as i32; }
            }
            pawns &= pawns - 1;
        }

        // Avamposti Cavalli
        let mut knights = s.pezzi[1] & s.colori[color.indice()];
        while knights != 0 {
            let sq = knights.trailing_zeros() as usize;
            if s.is_outpost_square(sq, color) { side_score += params::KNIGHT_OUTPOST_BONUS as i32; }
            knights &= knights - 1;
        }
        
        // Mobilità Regina
        let mut queens = s.pezzi[4] & s.colori[color.indice()];
        while queens != 0 {
            let sq = queens.trailing_zeros() as usize;
            side_score += s.count_safe_queen_moves(sq, color) as i32 * params::QUEEN_MOBILITY_WEIGHT as i32;
            if s.is_queen_exposed(sq, color) { side_score += params::QUEEN_EXPOSED_PENALTY as i32; }
            queens &= queens - 1;
        }

        if is_white { bonus += side_score; } else { bonus -= side_score; }
    }
    bonus
}