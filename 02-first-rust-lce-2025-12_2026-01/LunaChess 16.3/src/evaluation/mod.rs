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
    
    score += evaluate_side_heuristics(s, Colore::Bianco, phase);
    score -= evaluate_side_heuristics(s, Colore::Nero, phase);

    score
}

fn evaluate_side_heuristics(s: &Scacchiera, color: Colore, phase: i32) -> i32 {
    let mut side_score = 0;
    let is_white = color == Colore::Bianco;
    
    // Maschera per la prima traversa (Backrank)
    // Bianco: Rank 1 (0..8), Nero: Rank 8 (56..64)
    let backrank_mask: u64 = if is_white { 0x00000000000000FF } else { 0xFF00000000000000 };
    
    // --- 1. FORZATURA SVILUPPO (Anti-Apatia) ---
    // Applichiamo la penalità solo se siamo ancora in apertura/middlegame
    if phase < 160 {
        let minors = s.pezzi[Pezzo::Cavallo.indice()] | s.pezzi[Pezzo::Alfiere.indice()];
        let undeveloped = (minors & s.colori[color.indice()] & backrank_mask).count_ones();
        
        // Penalità severa per ogni pezzo minore ancora sulla casa di partenza
        side_score += undeveloped as i32 * params::DEVELOPMENT_PENALTY as i32;

        // Bonus attività: Premiamo i pezzi minori che sono usciti dalla backrank
        let active_minors = (minors & s.colori[color.indice()] & !backrank_mask).count_ones();
        side_score += active_minors as i32 * 20; 
    }

    // --- 2. PEDONI PASSATI ---
    let mut pawns = s.pezzi[Pezzo::Pedone.indice()] & s.colori[color.indice()];
    while pawns != 0 {
        let sq = pawns.trailing_zeros() as usize;
        if s.is_passed_pawn(sq, color) {
            let rank = if is_white { sq / 8 } else { 7 - (sq / 8) };
            let p_bonus = (params::PASSED_PAWN_BONUS[rank] as i32 * (256 - phase) + 
                           params::PASSED_PAWN_ENDGAME_BONUS[rank] as i32 * phase) / 256;
            side_score += p_bonus;
            
            // Prossimità del Re ai propri pedoni passati
            let king_pos = (s.pezzi[Pezzo::Re.indice()] & s.colori[color.indice()]).trailing_zeros() as usize;
            if s.manhattan_distance(sq, king_pos) <= 2 {
                side_score += params::KING_PROXIMITY_TO_OWN_PASSED_PAWN as i32;
            }
        }
        pawns &= pawns - 1;
    }

    // --- 3. AVAMPOSTI CAVALLI ---
    let mut knights = s.pezzi[Pezzo::Cavallo.indice()] & s.colori[color.indice()];
    while knights != 0 {
        let sq = knights.trailing_zeros() as usize;
        if s.is_outpost_square(sq, color) {
            side_score += params::KNIGHT_OUTPOST_BONUS as i32;
        }
        knights &= knights - 1;
    }

    // --- 4. SICUREZZA E MOBILITÀ REGINA ---
    let mut queens = s.pezzi[Pezzo::Regina.indice()] & s.colori[color.indice()];
    while queens != 0 {
        let sq = queens.trailing_zeros() as usize;
        side_score += s.count_safe_queen_moves(sq, color) as i32 * params::QUEEN_MOBILITY_WEIGHT as i32;
        if s.is_queen_exposed(sq, color) {
            side_score += params::QUEEN_EXPOSED_PENALTY as i32;
        }
        queens &= queens - 1;
    }

    side_score
}