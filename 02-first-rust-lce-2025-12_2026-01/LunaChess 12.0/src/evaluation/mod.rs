use crate::board::{Scacchiera, Pezzo, Colore};
pub mod params;
use params::*;

pub fn valuta_posizione(s: &Scacchiera) -> i16 {
    let mut punteggio: i16 = 0;
    let turno = s.turno();

    for i in 0..64 {
        // Usiamo i nomi dei metodi suggeriti dal compilatore
        if let Some(pezzo) = s.get_piece_at(i) {
            let colore_pezzo = s.get_color_at(i).expect("Pezzo presente senza colore");
            
            // 1. Valore Materiale Base
            let mut val = match pezzo {
                Pezzo::Pedone  => VALORE_PEDONE,
                Pezzo::Cavallo => VALORE_CAVALLO,
                Pezzo::Alfiere => VALORE_ALFIERE,
                Pezzo::Torre   => VALORE_TORRE,
                Pezzo::Regina  => VALORE_DONNA,
                Pezzo::Re      => 0,
            };

            // 2. Bonus Posizionale (PST)
            // Ribaltiamo l'indice per il Nero per leggere correttamente le tabelle
            let index_pst = if colore_pezzo == Colore::Bianco { i } else { i ^ 56 };
            
            val += match pezzo {
                Pezzo::Pedone  => PST_PEDONE[index_pst],
                Pezzo::Cavallo => PST_CAVALLO[index_pst],
                Pezzo::Alfiere => PST_ALFIERE[index_pst],
                Pezzo::Torre   => PST_TORRE[index_pst],
                Pezzo::Re      => PST_RE[index_pst],
                Pezzo::Regina  => 0, 
            };

            if colore_pezzo == Colore::Bianco {
                punteggio += val;
            } else {
                punteggio -= val;
            }
        }
    }

    // Restituiamo il punteggio relativo al giocatore di turno
    if turno == Colore::Bianco { punteggio } else { -punteggio }
}