// src/evaluation/pst.rs
use crate::board::{Scacchiera, Colore, Pezzo};
use crate::evaluation::params::*;

pub fn valuta_pst(s: &Scacchiera) -> i16 {
    let mut mg_score: i32 = 0;
    let mut eg_score: i32 = 0;
    let phase = calcola_fase(s) as i32;

    for p_idx in 0..6 {
        let pezzo = match p_idx {
            0 => Pezzo::Pedone, 1 => Pezzo::Cavallo, 2 => Pezzo::Alfiere,
            3 => Pezzo::Torre, 4 => Pezzo::Regina, _ => Pezzo::Re
        };
        
        let mg_table = get_table(pezzo);
        let eg_table = if pezzo == Pezzo::Re { &KING_ENDGAME_TABLE } else { mg_table };
        let val = MATERIAL_VALUES[p_idx] as i32;

        // --- Bianchi ---
        let mut bb_w = s.pezzi[p_idx] & s.colori[Colore::Bianco.indice()];
        while bb_w != 0 {
            let sq = bb_w.trailing_zeros() as usize;
            let idx = sq ^ 56; // Specchiamento: la tabella in params è orientata per il Nero (Rank 8 top)
            mg_score += val + mg_table[idx] as i32;
            eg_score += val + eg_table[idx] as i32;
            bb_w &= bb_w - 1;
        }

        // --- Neri ---
        let mut bb_b = s.pezzi[p_idx] & s.colori[Colore::Nero.indice()];
        while bb_b != 0 {
            let sq = bb_b.trailing_zeros() as usize;
            mg_score -= val + mg_table[sq] as i32;
            eg_score -= val + eg_table[sq] as i32;
            bb_b &= bb_b - 1;
        }
    }

    // Interpolazione lineare (Tapered Eval)
    ((mg_score * (256 - phase) + eg_score * phase) / 256) as i16
}

pub fn calcola_fase(s: &Scacchiera) -> i16 {
    let n_knights = s.pezzi[1].count_ones();
    let n_bishops = s.pezzi[2].count_ones();
    let n_rooks = s.pezzi[3].count_ones();
    let n_queens = s.pezzi[4].count_ones();
    let current = (n_knights + n_bishops + n_rooks * 2 + n_queens * 4) as i32;
    ((24 - current).max(0) * 256 / 24) as i16
}

// Funzione interna ed esportata per l'ordinamento delle mosse
pub fn get_table(p: Pezzo) -> &'static [i16; 64] {
    match p {
        Pezzo::Pedone => &PAWN_TABLE, Pezzo::Cavallo => &KNIGHT_TABLE,
        Pezzo::Alfiere => &BISHOP_TABLE, Pezzo::Torre => &ROOK_TABLE,
        Pezzo::Regina => &QUEEN_TABLE, Pezzo::Re => &KING_TABLE,
    }
}