// Import necessari dal modulo board
use crate::board::{Scacchiera, Colore, Pezzo};

// Tabelle di valutazione posizionale (semplificate)
const PEDONE_BIANCO_POS: [i32; 64] = [
    0,   0,   0,   0,   0,   0,   0,   0,
    50,  50,  50,  50,  50,  50,  50,  50,
    10,  10,  20,  30,  30,  20,  10,  10,
    5,   5,  10,  25,  25,  10,   5,   5,
    0,   0,   0,  20,  20,   0,   0,   0,
    5,  -5, -10,   0,   0, -10,  -5,   5,
    5,  10,  10, -20, -20,  10,  10,   5,
    0,   0,   0,   0,   0,   0,   0,   0,
];

// Tabella pedone nero (invertita rispetto al bianco) - Spostata qui per essere definita prima dell'uso
const PEDONE_NERO_POS: [i32; 64] = [
    0,   0,   0,   0,   0,   0,   0,   0,
    5,  10,  10, -20, -20,  10,  10,   5,
    5,  -5, -10,   0,   0, -10,  -5,   5,
    0,   0,   0,  20,  20,   0,   0,   0,
    5,   5,  10,  25,  25,  10,   5,   5,
    10,  10,  20,  30,  30,  20,  10,  10,
    50,  50,  50,  50,  50,  50,  50,  50,
    0,   0,   0,   0,   0,   0,   0,   0,
];

const CAVALLO_POS: [i32; 64] = [
    -50, -40, -30, -30, -30, -30, -40, -50,
    -40, -20,   0,   0,   0,   0, -20, -40,
    -30,   0,  10,  15,  15,  10,   0, -30,
    -30,   5,  15,  20,  20,  15,   5, -30,
    -30,   0,  15,  20,  20,  15,   0, -30,
    -30,   5,  10,  15,  15,  10,   5, -30,
    -40, -20,   0,   5,   5,   0, -20, -40,
    -50, -40, -30, -30, -30, -30, -40, -50,
];

const ALFIERE_POS: [i32; 64] = [
    -20, -10, -10, -10, -10, -10, -10, -20,
    -10,   0,   0,   0,   0,   0,   0, -10,
    -10,   0,   5,  10,  10,   5,   0, -10,
    -10,   5,   5,  10,  10,   5,   5, -10,
    -10,   0,  10,  10,  10,  10,   0, -10,
    -10,  10,  10,  10,  10,  10,  10, -10,
    -10,   5,   0,   0,   0,   0,   5, -10,
    -20, -10, -10, -10, -10, -10, -10, -20,
];

const TORRE_POS: [i32; 64] = [
    0,   0,   0,   0,   0,   0,   0,   0,
    5,  10,  10,  10,  10,  10,  10,   5,
   -5,   0,   0,   0,   0,   0,   0,  -5,
   -5,   0,   0,   0,   0,   0,   0,  -5,
   -5,   0,   0,   0,   0,   0,   0,  -5,
   -5,   0,   0,   0,   0,   0,   0,  -5,
   -5,   0,   0,   0,   0,   0,   0,  -5,
    0,   0,   0,   5,   5,   0,   0,   0,
];

const REGINA_POS: [i32; 64] = [
    -20, -10, -10, -5, -5, -10, -10, -20,
    -10,   0,   0,   0,   0,   0,   0, -10,
    -10,   0,   5,   5,   5,   5,   0, -10,
     -5,   0,   5,   5,   5,   5,   0,  -5,
      0,   0,   5,   5,   5,   5,   0,  -5,
    -10,   5,   5,   5,   5,   5,   0, -10,
    -10,   0,   5,   0,   0,   0,   0, -10,
    -20, -10, -10, -5, -5, -10, -10, -20,
];

const RE_POS_APERTURA: [i32; 64] = [
    -30, -40, -40, -50, -50, -40, -40, -30,
    -30, -40, -40, -50, -50, -40, -40, -30,
    -30, -40, -40, -50, -50, -40, -40, -30,
    -30, -40, -40, -50, -50, -40, -40, -30,
    -20, -30, -30, -40, -40, -30, -30, -20,
    -10, -20, -20, -20, -20, -20, -20, -10,
     20,  20,   0,   0,   0,   0,  20,  20,
     20,  30,  10,   0,   0,  10,  30,  20,
];

pub fn valuta_posizione(scacchiera: &Scacchiera) -> i32 {
    let mut valore = 0;
    
    // Valore materiale e posizionale per ogni pezzo
    for colore in &[Colore::Bianco, Colore::Nero] {
        for pezzo in &[
            Pezzo::Pedone, Pezzo::Cavallo, Pezzo::Alfiere,
            Pezzo::Torre, Pezzo::Regina, Pezzo::Re
        ] {
            let bitboard = scacchiera.bitboard_pezzo(*pezzo, *colore);
            let mut bb = bitboard;
            
            while bb != 0 {
                let square = bb.trailing_zeros() as usize;
                bb &= bb - 1;
                
                // Valore materiale
                let pezzo_valore = match pezzo {
                    Pezzo::Pedone => 100,
                    Pezzo::Cavallo => 320,
                    Pezzo::Alfiere => 330,
                    Pezzo::Torre => 500,
                    Pezzo::Regina => 900,
                    Pezzo::Re => 20000,
                };
                
                // Valore posizionale
                let pos_valore = valore_posizionale(*pezzo, *colore, square);
                let mut totale_valore = pezzo_valore + pos_valore;
                
                // Bilancia il valore in base al colore
                if *colore == scacchiera.colore_attivo() {
                    valore += totale_valore;
                } else {
                    valore -= totale_valore;
                }
            }
        }
    }
    
    // Bonus per sviluppo (semplicistico)
    valore += bonus_sviluppo(scacchiera);
    
    // Penalità per re sotto scacco
    if scacchiera.re_in_scacco(scacchiera.colore_attivo()) {
        valore -= 50;
    }
    
    valore
}

fn valore_posizionale(pezzo: Pezzo, colore: Colore, square: usize) -> i32 {
    let tabella = match pezzo {
        Pezzo::Pedone => {
            if colore == Colore::Bianco {
                &PEDONE_BIANCO_POS
            } else {
                // Inverti la tabella per il nero
                &PEDONE_NERO_POS
            }
        }
        Pezzo::Cavallo => &CAVALLO_POS,
        Pezzo::Alfiere => &ALFIERE_POS,
        Pezzo::Torre => &TORRE_POS,
        Pezzo::Regina => &REGINA_POS,
        Pezzo::Re => &RE_POS_APERTURA,
    };
    
    // Adatta l'indice in base al colore (il nero vede la scacchiera invertita)
    let idx = if colore == Colore::Bianco {
        square
    } else {
        63 - square // Inverti per il nero
    };
    
    tabella[idx]
}

fn bonus_sviluppo(scacchiera: &Scacchiera) -> i32 {
    let mut bonus = 0;
    
    // Bonus per pezzi sviluppati (fuori dalla prima/ultima riga)
    let pezzi_sviluppati_bianco = scacchiera.bitboard_colore(Colore::Bianco) & !0xFF; // Non sulla prima riga
    let pezzi_sviluppati_nero = scacchiera.bitboard_colore(Colore::Nero) & !0xFF00000000000000; // Non sull'ultima riga
    
    bonus += (pezzi_sviluppati_bianco.count_ones() as i32) * 10;
    bonus -= (pezzi_sviluppati_nero.count_ones() as i32) * 10;
    
    // Bonus per arrocco fatto
    let diritti = scacchiera.diritti_arrocco();
    if !diritti.bianco_lato_re && !diritti.bianco_lato_regina {
        bonus += 30; // Bianco ha arroccato
    }
    if !diritti.nero_lato_re && !diritti.nero_lato_regina {
        bonus -= 30; // Nero ha arroccato
    }
    
    if scacchiera.colore_attivo() == Colore::Bianco {
        bonus
    } else {
        -bonus
    }
}

// Funzione di valutazione più semplice (fallback)
pub fn valuta_posizione_semplice(scacchiera: &Scacchiera) -> i32 {
    let mut valore = 0;
    
    for rank in 0..8 {
        for file in 0..8 {
            if let Some((pezzo, colore)) = scacchiera.pezzi[rank][file] {
                let pezzo_valore = match pezzo {
                    Pezzo::Pedone => 100,
                    Pezzo::Cavallo => 320,
                    Pezzo::Alfiere => 330,
                    Pezzo::Torre => 500,
                    Pezzo::Regina => 900,
                    Pezzo::Re => 20000,
                };
                if colore == scacchiera.colore_attivo() {
                    valore += pezzo_valore;
                } else {
                    valore -= pezzo_valore;
                }
            }
        }
    }
    
    valore
}