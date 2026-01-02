use crate::board::{Scacchiera, Pezzo, Colore};
pub mod params;
use params::*;

pub fn valuta_posizione(s: &Scacchiera) -> i16 {
    let mut punteggio: i16 = 0;
    let turno = s.turno();

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

            if colore_pezzo == Colore::Bianco { punteggio += val; } 
            else { punteggio -= val; }
        }
    }

    if turno == Colore::Bianco { punteggio } else { -punteggio }
}

fn valutazione_sicurezza_re(s: &Scacchiera, colore: Colore, re_idx: usize) -> i16 {
    let mut safety = 0;
    
    // Controllo scudo pedoni (solo se il re è nelle case d'arrocco tipiche)
    // Bianco: g1 (62), c1 (58) | Nero: g8 (6), c8 (2)
    let scudo_pedoni = match (colore, re_idx) {
        (Colore::Bianco, 62) => vec![53, 54, 55], // Pedoni f2, g2, h2
        (Colore::Bianco, 58) => vec![48, 49, 50], // Pedoni a2, b2, c2
        (Colore::Nero, 6)    => vec![13, 14, 15], // Pedoni f7, g7, h7
        (Colore::Nero, 2)    => vec![8, 9, 10],   // Pedoni a7, b7, c7
        _ => return -20, // Penalità se il Re è altrove
    };

    for &idx in &scudo_pedoni {
        if let Some(Pezzo::Pedone) = s.get_piece_at(idx) {
            if s.get_color_at(idx) == Some(colore) {
                safety += 15; // Bonus per ogni pedone a protezione
            }
        }
    }
    
    safety
}