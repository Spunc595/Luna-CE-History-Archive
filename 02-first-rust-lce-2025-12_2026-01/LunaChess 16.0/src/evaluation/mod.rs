// src/evaluation/mod.rs
pub mod params;
pub mod pst;
use crate::board::{Scacchiera, Colore, Pezzo};
use crate::nnue::Network;

pub fn evaluate(s: &Scacchiera, net: &Network) -> i32 {
    let score = evaluate_internal(s);
    if s.turno == Colore::Bianco { score } else { -score }
}

pub fn evaluate_classical(s: &Scacchiera) -> i32 {
    let score = evaluate_internal(s);
    if s.turno == Colore::Bianco { score } else { -score }
}

fn evaluate_internal(s: &Scacchiera) -> i32 {
    let phase = pst::calcola_fase(s) as i32;
    let pst_score = pst::valuta_pst(s) as i32;
    pst_score + evaluate_dynamic_bonus(s, phase)
}

fn evaluate_dynamic_bonus(s: &Scacchiera, phase: i32) -> i32 {
    let mut bonus = 0;
    bonus += evaluate_side_heuristics(s, Colore::Bianco, phase);
    bonus -= evaluate_side_heuristics(s, Colore::Nero, phase);
    bonus
}

fn evaluate_side_heuristics(s: &Scacchiera, color: Colore, phase: i32) -> i32 {
    let mut side_score = 0;
    let enemy = color.opposto();

    // 1. Pedoni Passati e Prossimità Re
    let mut pawns = s.pezzi[Pezzo::Pedone.indice()] & s.colori[color.indice()];
    let king_pos = (s.pezzi[Pezzo::Re.indice()] & s.colori[color.indice()]).trailing_zeros() as usize;
    let enemy_king_pos = (s.pezzi[Pezzo::Re.indice()] & s.colori[enemy.indice()]).trailing_zeros() as usize;

    while pawns != 0 {
        let sq = pawns.trailing_zeros() as usize;
        if s.is_passed_pawn(sq, color) {
            let rank = if color == Colore::Bianco { sq / 8 } else { 7 - (sq / 8) };
            let mut p_bonus = (params::PASSED_PAWN_BONUS[rank] as i32 * (256 - phase) + 
                               params::PASSED_PAWN_ENDGAME_BONUS[rank] as i32 * phase) / 256;

            if s.is_square_protected_by_pawn(sq, color) { p_bonus += params::PROTECTED_PASSED_PAWN_BONUS as i32; }

            let dist_own = s.manhattan_distance(sq, king_pos);
            let dist_enemy = s.manhattan_distance(sq, enemy_king_pos);
            if dist_own <= 2 { p_bonus += params::KING_PROXIMITY_TO_OWN_PASSED_PAWN as i32; }
            if dist_enemy > 3 { p_bonus += (dist_enemy as i32 - 3) * 5; }

            side_score += p_bonus;
        }
        pawns &= pawns - 1;
    }

    // 2. Avamposti Cavalli
    let mut knights = s.pezzi[Pezzo::Cavallo.indice()] & s.colori[color.indice()];
    while knights != 0 {
        let sq = knights.trailing_zeros() as usize;
        if s.is_outpost_square(sq, color) {
            side_score += params::KNIGHT_OUTPOST_BONUS as i32;
        }
        knights &= knights - 1;
    }

    // 3. Regina
    let mut queens = s.pezzi[Pezzo::Regina.indice()] & s.colori[color.indice()];
    while queens != 0 {
        let sq = queens.trailing_zeros() as usize;
        side_score += (s.count_safe_queen_moves(sq, color) as i16 * params::QUEEN_MOBILITY_WEIGHT) as i32;
        if s.is_queen_exposed(sq, color) { side_score += params::QUEEN_EXPOSED_PENALTY as i32; }
        queens &= queens - 1;
    }

    side_score
}