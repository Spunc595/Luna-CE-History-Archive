use crate::board::{Scacchiera, Pezzo, Colore};

pub fn valuta_posizione(scacchiera: &Scacchiera) -> i32 {
    let mut valore = 0;
    
    // Valore materiale usando il metodo pubblico invece di accedere direttamente al campo
    for riga in 0..8 {
        for colonna in 0..8 {
            if let Some(casella) = crate::board::Casella::nuova(colonna as u8, riga as u8) {
                if let Some((pezzo, colore)) = scacchiera.ottieni_pezzo(casella) {
                    let valore_pezzo = match pezzo {
                        Pezzo::Pedone => 100,
                        Pezzo::Cavallo => 320,
                        Pezzo::Alfiere => 330,
                        Pezzo::Torre => 500,
                        Pezzo::Regina => 900,
                        Pezzo::Re => 20000,
                    };
                    
                    let segno = match colore {
                        Colore::Bianco => 1,
                        Colore::Nero => -1,
                    };
                    
                    valore += valore_pezzo * segno;
                    
                    // Valori posizionali (semplificati)
                    valore += valuta_posizione_pezzo(pezzo, colore, riga, colonna) * segno;
                }
            }
        }
    }
    
    // Bonus per il colore attivo
    match scacchiera.colore_attivo() {
        Colore::Bianco => valore,
        Colore::Nero => -valore,
    }
}

fn valuta_posizione_pezzo(pezzo: Pezzo, colore: Colore, riga: usize, colonna: usize) -> i32 {
    match pezzo {
        Pezzo::Pedone => {
            // I pedoni valgono di più man mano che avanzano
            let progresso = match colore {
                Colore::Bianco => riga as i32,
                Colore::Nero => (7 - riga) as i32,
            };
            progresso * 10
        }
        Pezzo::Cavallo => {
            // I cavalli sono migliori al centro
            let centro = (3 - (riga as i32 - 3).abs()) + (3 - (colonna as i32 - 3).abs());
            centro * 5
        }
        Pezzo::Alfiere => {
            // Gli alfieri sono migliori su diagonali lunghe
            if riga == colonna || riga + colonna == 7 {
                10
            } else {
                0
            }
        }
        Pezzo::Torre => {
            // Le torre sono migliori su colonne semi-aperte o aperte
            // (semplificato: bonus se sulla 7a traversa)
            if (colore == Colore::Bianco && riga == 6) || (colore == Colore::Nero && riga == 1) {
                20
            } else {
                0
            }
        }
        Pezzo::Regina => {
            // La regina è più forte al centro
            let distanza_dal_centro = (riga as i32 - 3).abs() + (colonna as i32 - 3).abs();
            -distanza_dal_centro * 3
        }
        Pezzo::Re => {
            // Il re è più sicuro negli angoli all'inizio, al centro in finale
            // (semplificato: penalità se esposto)
            let esposizione = ((riga as i32 - 3).abs() + (colonna as i32 - 3).abs()) / 2;
            -esposizione * 5
        }
    }
}