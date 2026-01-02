use crate::board::{Scacchiera, Pezzo, Colore};
mod params;
use params::PARAMS;

pub fn valuta_posizione(s: &Scacchiera) -> i32 {
    let mut mg = [0, 0];
    let mut eg = [0, 0];
    let mut phase = 0;
    let occ = s.occupazione();

    for sq in 0..64 {
        if let Some(p) = s.get_piece_at(sq) {
            let c = s.get_color_at(sq).unwrap();
            let pi = p.indice();
            let ci = c.indice();
            
            phase += match p { Pezzo::Cavallo | Pezzo::Alfiere => 1, Pezzo::Torre => 2, Pezzo::Regina => 4, _ => 0 };

            // Inserisci qui i valori dei pezzi (MG_VAL/EG_VAL) e PST
            // mg[ci] += PARAMS.piece_values_mg[pi] + PST_MG[pi][sq];
            // eg[ci] += PARAMS.piece_values_eg[pi] + PST_EG[pi][sq];

            if p != Pezzo::Pedone && p != Pezzo::Re {
                let moves = s.get_attacks_for_piece(p, sq, occ);
                let count = moves.count_ones() as i32;
                mg[ci] += count * PARAMS.mobility_mg[pi];
                eg[ci] += count * PARAMS.mobility_eg[pi];
            }

            if p == Pezzo::Re {
                mg[ci] += valuta_sicurezza_re(s, c, sq);
            }
        }
    }

    let p_clamped = phase.min(24);
    let mg_diff = mg[0] - mg[1];
    let eg_diff = eg[0] - eg[1];
    let score = (mg_diff * p_clamped + eg_diff * (24 - p_clamped)) / 24;
    if s.turno() == Colore::Bianco { score } else { -score }
}

fn valuta_sicurezza_re(s: &Scacchiera, c: Colore, sq: usize) -> i32 {
    let mut score = 0;
    let rank = sq / 8;
    let file = sq % 8;
    if (c == Colore::Bianco && rank == 0) || (c == Colore::Nero && rank == 7) {
        let shield_row = if c == Colore::Bianco { 1 } else { 6 };
        for f_offset in -1..=1 {
            let f = file as i32 + f_offset;
            if f >= 0 && f <= 7 {
                let shield_sq = shield_row * 8 + f as usize;
                if let Some(Pezzo::Pedone) = s.get_piece_at(shield_sq) {
                    if s.get_color_at(shield_sq).unwrap() == c { score += PARAMS.king_shield_bonus; }
                }
            }
        }
    }
    score
}