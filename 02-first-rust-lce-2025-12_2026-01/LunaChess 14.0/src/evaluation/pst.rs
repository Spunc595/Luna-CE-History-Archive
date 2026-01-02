use crate::board::{Scacchiera, Colore, Pezzo};
use crate::evaluation::params::*; // Importa tutto da params

pub fn valuta_pst(s: &Scacchiera) -> i16 {
    let mut score = 0;

    for p_idx in 0..6 {
        let pezzo = unsafe { std::mem::transmute::<u8, Pezzo>(p_idx as u8) };
        let table = get_table(pezzo);
        let val = MATERIAL_VALUES[p_idx];

        // Pezzi Bianchi
        let mut bb_w = s.pezzi[p_idx] & s.colori[Colore::Bianco.indice()];
        while bb_w != 0 {
            let sq = bb_w.trailing_zeros() as usize;
            // Ribaltamento indice per il Bianco (A1 è 56 in queste tabelle visuali)
            let table_idx = sq ^ 56; 
            score += val + table[table_idx];
            bb_w &= bb_w - 1;
        }

        // Pezzi Neri
        let mut bb_b = s.pezzi[p_idx] & s.colori[Colore::Nero.indice()];
        while bb_b != 0 {
            let sq = bb_b.trailing_zeros() as usize;
            // Indice diretto per il Nero
            let table_idx = sq; 
            score -= val + table[table_idx];
            bb_b &= bb_b - 1;
        }
    }

    if s.turno == Colore::Bianco { score } else { -score }
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