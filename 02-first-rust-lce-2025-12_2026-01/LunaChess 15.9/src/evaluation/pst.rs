// src/evaluation/pst.rs

use crate::board::{Scacchiera, Colore, Pezzo};
use crate::evaluation::params::*;

pub fn valuta_pst(s: &Scacchiera) -> i16 {
    let mut mg_score = 0;
    let mut eg_score = 0;
    
    // Calcoliamo la fase (0 = Middlegame puro, 256 = Endgame puro)
    let phase = calcola_fase(s);

    for p_idx in 0..6 {
        let pezzo = match p_idx {
            0 => Pezzo::Pedone, 1 => Pezzo::Cavallo, 2 => Pezzo::Alfiere,
            3 => Pezzo::Torre, 4 => Pezzo::Regina, _ => Pezzo::Re
        };
        
        let mg_table = get_table(pezzo);
        // Se è il Re, usiamo la tabella specifica per il finale, altrimenti usiamo la stessa
        let eg_table = if pezzo == Pezzo::Re { &KING_ENDGAME_TABLE } else { mg_table };
        let val = MATERIAL_VALUES[p_idx];

        // Pezzi Bianchi
        let mut bb_w = s.pezzi[p_idx] & s.colori[Colore::Bianco.indice()];
        while bb_w != 0 {
            let sq = bb_w.trailing_zeros() as usize;
            let idx = sq ^ 56; // Ribaltamento per il Bianco
            mg_score += val + mg_table[idx];
            eg_score += val + eg_table[idx];
            bb_w &= bb_w - 1;
        }

        // Pezzi Neri
        let mut bb_b = s.pezzi[p_idx] & s.colori[Colore::Nero.indice()];
        while bb_b != 0 {
            let sq = bb_b.trailing_zeros() as usize;
            let idx = sq; // Indice diretto per il Nero
            mg_score -= val + mg_table[idx];
            eg_score -= val + eg_table[idx];
            bb_b &= bb_b - 1;
        }
    }

    // Interpolazione lineare: (MG * (256 - phase) + EG * phase) / 256
    let final_score = ((mg_score as i32 * (256 - phase as i32) + eg_score as i32 * phase as i32) / 256) as i16;
    
    final_score
}

pub fn calcola_fase(s: &Scacchiera) -> i16 {
    // Pesi per determinare la fase (Donna=4, Torre=2, Alfiere/Cavallo=1)
    let n_knights = (s.pezzi[Pezzo::Cavallo.indice()]).count_ones();
    let n_bishops = (s.pezzi[Pezzo::Alfiere.indice()]).count_ones();
    let n_rooks = (s.pezzi[Pezzo::Torre.indice()]).count_ones();
    let n_queens = (s.pezzi[Pezzo::Regina.indice()]).count_ones();

    let total_phase = 24; // Massimo materiale (esclusi pedoni e re)
    let current_material = (n_knights * 1 + n_bishops * 1 + n_rooks * 2 + n_queens * 4) as i32;
    
    // Calcoliamo quanto materiale manca rispetto all'inizio
    let mut phase = total_phase - current_material;
    phase = (phase * 256) / total_phase;
    
    phase.clamp(0, 256) as i16
}

fn get_table(p: Pezzo) -> &'static [i16; 64] {
    match p {
        Pezzo::Pedone => &PAWN_TABLE,
        Pezzo::Cavallo => &KNIGHT_TABLE,
        Pezzo::Alfiere => &BISHOP_TABLE,
        Pezzo::Torre => &ROOK_TABLE,
        Pezzo::Regina => &QUEEN_TABLE,
        Pezzo::Re => &KING_TABLE,
    }
}