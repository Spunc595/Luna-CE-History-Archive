use crate::board::{Scacchiera, Pezzo, Colore, Bitboard};
pub mod params;
use params::*;

pub fn valuta_posizione(s: &Scacchiera) -> i16 {
    let mut punteggio: i16 = 0;
    let turno = s.turno();
    let occ = s.occupazione();

    for i in 0..64 {
        if let Some(pezzo) = s.get_piece_at(i) {
            let colore_pezzo = s.get_color_at(i).expect("Pezzo senza colore");
            let index_pst = if colore_pezzo == Colore::Bianco { i } else { i ^ 56 };
            
            let mut val = match pezzo {
                Pezzo::Pedone  => VALORE_PEDONE + PST_PEDONE[index_pst],
                Pezzo::Cavallo => VALORE_CAVALLO + PST_CAVALLO[index_pst],
                Pezzo::Alfiere => VALORE_ALFIERE + PST_ALFIERE[index_pst],
                Pezzo::Torre   => VALORE_TORRE + PST_TORRE[index_pst],
                Pezzo::Regina  => VALORE_REGINA,
                Pezzo::Re      => PST_RE[index_pst] + valutazione_sicurezza_re(s, colore_pezzo, i),
            };

            // AGGIUNTA: Valutazione Mobilità
            if pezzo != Pezzo::Pedone && pezzo != Pezzo::Re {
                let attacchi = s.get_attacks_for_piece(pezzo, i, occ);
                let count = attacchi.count_ones() as i16;
                val += count * 2; // 2 centipedoni per ogni casa controllata
            }

            if colore_pezzo == Colore::Bianco { punteggio += val; } 
            else { punteggio -= val; }
        }
    }

    if turno == Colore::Bianco { punteggio } else { -punteggio }
}

fn valutazione_sicurezza_re(s: &Scacchiera, colore: Colore, re_idx: usize) -> i16 {
    let mut safety = 0;
    let scudo_pedoni = match (colore, re_idx) {
        (Colore::Bianco, 62) => vec![53, 54, 55], 
        (Colore::Bianco, 58) => vec![48, 49, 50], 
        (Colore::Nero, 6)    => vec![13, 14, 15], 
        (Colore::Nero, 2)    => vec![8, 9, 10],   
        _ => return -30, // Penalità aumentata per Re esposto
    };

    for &idx in &scudo_pedoni {
        if let Some(Pezzo::Pedone) = s.get_piece_at(idx) {
            if s.get_color_at(idx) == Some(colore) {
                safety += 20; // Bonus aumentato
            }
        }
    }
    safety
}