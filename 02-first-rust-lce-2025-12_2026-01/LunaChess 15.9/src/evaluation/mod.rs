// src/evaluation/mod.rs

pub mod params;
pub mod pst;

use crate::board::{Scacchiera, Colore, Pezzo};
use crate::nnue::Network;

pub fn evaluate(s: &Scacchiera, net: &Network) -> i32 {
    // Al momento NNUE è disattivato o usato in mix come da tua richiesta
    let pst_score = pst::valuta_pst(s) as i32;
    let phase = pst::calcola_fase(s) as i32;
    let dynamic_bonus = evaluate_dynamic_bonus(s, phase);

    let score = pst_score + dynamic_bonus;
    
    // Restituiamo il punteggio relativo al giocatore di turno (Negamax)
    if s.turno == Colore::Bianco { score } else { -score }
}

pub fn evaluate_classical(s: &Scacchiera) -> i32 {
    let pst_score = pst::valuta_pst(s) as i32;
    let phase = pst::calcola_fase(s) as i32;
    let dynamic_bonus = evaluate_dynamic_bonus(s, phase);
    
    let score = pst_score + dynamic_bonus;
    if s.turno == Colore::Bianco { score } else { -score }
}

fn evaluate_dynamic_bonus(s: &Scacchiera, phase: i32) -> i32 {
    let mut bonus = 0;
    bonus += evaluate_passed_pawns(s, phase);
    bonus += evaluate_queen_mobility(s);
    bonus
}

fn evaluate_passed_pawns(s: &Scacchiera, phase: i32) -> i32 {
    let mut score = 0;
    
    for color in [Colore::Bianco, Colore::Nero] {
        let mut pawns = s.pezzi[Pezzo::Pedone.indice()] & s.colori[color.indice()];
        
        while pawns != 0 {
            let sq = pawns.trailing_zeros() as usize;
            
            if s.is_passed_pawn(sq, color) {
                let rank = if color == Colore::Bianco { sq / 8 } else { 7 - (sq / 8) };
                
                // Bonus differenziato tra Middlegame ed Endgame
                let mg_bonus = params::PASSED_PAWN_BONUS[rank];
                let eg_bonus = params::PASSED_PAWN_ENDGAME_BONUS[rank];
                
                // Interpolazione del bonus in base alla fase
                let mut p_bonus = ((mg_bonus as i32 * (256 - phase) + eg_bonus as i32 * phase) / 256) as i32;
                
                if s.is_square_protected_by_pawn(sq, color) {
                    p_bonus += params::PROTECTED_PASSED_PAWN_BONUS as i32;
                }
                
                if color == Colore::Bianco { score += p_bonus; } else { score -= p_bonus; }
            }
            pawns &= pawns - 1;
        }
    }
    score
}

fn evaluate_queen_mobility(s: &Scacchiera) -> i32 {
    let mut score = 0;

    for color in [Colore::Bianco, Colore::Nero] {
        let mut queens = s.pezzi[Pezzo::Regina.indice()] & s.colori[color.indice()];
        
        while queens != 0 {
            let sq = queens.trailing_zeros() as usize;
            
            let mobility_count = s.count_safe_queen_moves(sq, color);
            let mut q_score = (mobility_count as i16 * params::QUEEN_MOBILITY_WEIGHT) as i32;
            
            if s.is_queen_exposed(sq, color) {
                q_score += params::QUEEN_EXPOSED_PENALTY as i32;
            }

            if color == Colore::Bianco { score += q_score; } else { score -= q_score; }
            queens &= queens - 1;
        }
    }
    score
}