use crate::board::{Scacchiera, Pezzo, Colore};

pub fn valuta(scacchiera: &Scacchiera) -> i32 {
    let mut valore = 0;
    
    // Valore materiale
    for indice in 0..64 {
        if let Some(casella) = crate::board::Casella::da_indice(indice) {
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
                
                // Valore posizionale (esempio per pedoni)
                if pezzo == Pezzo::Pedone {
                    valore += valuta_posizione_pedone(casella, colore) * segno;
                }
            }
        }
    }
    
    // Valore per il colore attivo
    match scacchiera.colore_attivo() {
        Colore::Bianco => valore,
        Colore::Nero => -valore,
    }
}

fn valuta_posizione_pedone(casella: crate::board::Casella, colore: Colore) -> i32 {
    let numero = casella.numero() as i32;
    let lettera = casella.lettera() as i32;
    
    let valore_base = match colore {
        Colore::Bianco => numero * 5, // I pedoni bianchi valgono di più più avanzano
        Colore::Nero => (7 - numero) * 5, // I pedoni neri valgono di più più avanzano
    };
    
    // Bonus per pedoni centrali
    let bonus_centrale = if lettera >= 3 && lettera <= 4 {
        10
    } else {
        0
    };
    
    valore_base + bonus_centrale
}

// Tabella dei pezzi per valutazione posizionale (esempio per pedoni bianchi)
const TABELLA_PEDONI_BIANCHI: [[i32; 8]; 8] = [
    [0,  0,  0,  0,  0,  0,  0,  0],
    [50, 50, 50, 50, 50, 50, 50, 50],
    [10, 10, 20, 30, 30, 20, 10, 10],
    [5,  5, 10, 25, 25, 10,  5,  5],
    [0,  0,  0, 20, 20,  0,  0,  0],
    [5, -5,-10,  0,  0,-10, -5,  5],
    [5, 10, 10,-20,-20, 10, 10,  5],
    [0,  0,  0,  0,  0,  0,  0,  0],
];