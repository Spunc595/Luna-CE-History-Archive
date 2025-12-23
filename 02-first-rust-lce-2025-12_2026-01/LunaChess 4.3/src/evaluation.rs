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

// NUOVA TABELLA: Re nel finale (più attivo)
const RE_POS_FINALE: [i32; 64] = [
    -50, -30, -30, -30, -30, -30, -30, -50,
    -30, -20, -10, -10, -10, -10, -20, -30,
    -30, -10,  20,  30,  30,  20, -10, -30,
    -30, -10,  30,  40,  40,  30, -10, -30,
    -30, -10,  30,  40,  40,  30, -10, -30,
    -30, -10,  20,  30,  30,  20, -10, -30,
    -30, -20, -10,   0,   0, -10, -20, -30,
    -50, -40, -30, -20, -20, -30, -40, -50,
];

// Definizioni temporanee per compatibilità (queste dovrebbero essere in endgame.rs)
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum WDL {
    Perdita,
    PerditaBenedetta,
    Patta,
    VittoriaMaledetta,
    Vittoria,
}

// Struttura temporanea (dovrebbe essere in endgame.rs)
pub struct EndgameHandler {
    // Campi per gestire finali Syzygy
}

impl EndgameHandler {
    pub fn new() -> Self {
        EndgameHandler {
            // Inizializza campi
        }
    }
    
    pub fn e_finale(&self, _scacchiera: &Scacchiera) -> bool {
        false // Implementazione temporanea
    }
    
    pub fn carica_syzygy(&mut self, _percorso: &str, _pezzi_massimi: u8) -> bool {
        false // Implementazione temporanea
    }
    
    pub fn prova_syzygy(&self, _scacchiera: &Scacchiera) -> Option<WDL> {
        None // Implementazione temporanea
    }
    
    pub fn valuta_finale(&self, _scacchiera: &Scacchiera, valore: i32) -> i32 {
        valore // Per ora restituisce il valore invariato
    }
}

/// Gestore per la valutazione delle posizioni
pub struct Valutatore {
    endgame_handler: EndgameHandler, // AGGIUNTA: gestore per i finali
}

impl Valutatore {
    pub fn nuovo() -> Self {
        Self {
            endgame_handler: EndgameHandler::new(), // Inizializza il gestore dei finali
        }
    }
    
    /// Carica le tabelle Syzygy
    pub fn carica_tabelle_syzygy(&mut self, percorso: &str, pezzi_massimi: u8) -> bool {
        self.endgame_handler.carica_syzygy(percorso, pezzi_massimi)
    }
    
    /// Valuta la posizione principale
    pub fn valuta(&self, scacchiera: &Scacchiera) -> i32 {
        let mut valore = 0;
        
        // Prima controlla se siamo in un finale e abbiamo tabelle Syzygy
        if self.endgame_handler.e_finale(scacchiera) {
            if let Some(wdl) = self.endgame_handler.prova_syzygy(scacchiera) {
                return self.wdl_a_punteggio(wdl);
            }
        }
        
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
                    
                    // Valore posizionale (usando la tabella corretta per il finale)
                    let pos_valore = self.valore_posizionale(*pezzo, *colore, square, scacchiera);
                    let totale_valore = pezzo_valore + pos_valore;
                    
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
        
        // Aggiungi valutazioni specifiche per i finali
        if self.endgame_handler.e_finale(scacchiera) {
            valore = self.endgame_handler.valuta_finale(scacchiera, valore);
        }
        
        // Inverti per il nero
        if scacchiera.colore_attivo() == Colore::Nero {
            valore = -valore;
        }
        
        valore
    }
    
    /// Valore posizionale (adattato per i finali)
    fn valore_posizionale(&self, pezzo: Pezzo, colore: Colore, square: usize, scacchiera: &Scacchiera) -> i32 {
        // Scegli la tabella corretta in base alla fase della partita
        let tabella = match pezzo {
            Pezzo::Pedone => {
                if colore == Colore::Bianco {
                    &PEDONE_BIANCO_POS
                } else {
                    &PEDONE_NERO_POS
                }
            }
            Pezzo::Cavallo => &CAVALLO_POS,
            Pezzo::Alfiere => &ALFIERE_POS,
            Pezzo::Torre => &TORRE_POS,
            Pezzo::Regina => &REGINA_POS,
            Pezzo::Re => {
                // Nel finale, usa una tabella diversa per il re
                if self.endgame_handler.e_finale(scacchiera) {
                    &RE_POS_FINALE
                } else {
                    &RE_POS_APERTURA
                }
            }
        };
        
        // Adatta l'indice in base al colore (il nero vede la scacchiera invertita)
        let idx = if colore == Colore::Bianco {
            square
        } else {
            63 - square // Inverti per il nero
        };
        
        tabella[idx]
    }
    
    /// Converti WDL in punteggio
    fn wdl_a_punteggio(&self, wdl: WDL) -> i32 {
        // Converti WDL in punteggio
        match wdl {
            WDL::Perdita => -9000,
            WDL::PerditaBenedetta => -8000,
            WDL::Patta => 0,
            WDL::VittoriaMaledetta => 8000,
            WDL::Vittoria => 9000,
        }
    }
}

// Funzione principale per compatibilità (mantiene l'interfaccia esistente)
pub fn valuta_posizione(scacchiera: &Scacchiera) -> i32 {
    let valutatore = Valutatore::nuovo();
    valutatore.valuta(scacchiera)
}

fn valore_posizionale(pezzo: Pezzo, colore: Colore, square: usize) -> i32 {
    // Questa funzione è mantenuta per compatibilità
    let tabella = match pezzo {
        Pezzo::Pedone => {
            if colore == Colore::Bianco {
                &PEDONE_BIANCO_POS
            } else {
                &PEDONE_NERO_POS
            }
        }
        Pezzo::Cavallo => &CAVALLO_POS,
        Pezzo::Alfiere => &ALFIERE_POS,
        Pezzo::Torre => &TORRE_POS,
        Pezzo::Regina => &REGINA_POS,
        Pezzo::Re => &RE_POS_APERTURA, // Usa sempre la tabella di apertura per compatibilità
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
    let valutatore = Valutatore::nuovo();
    valutatore.valuta(scacchiera) // Usa il nuovo valutatore
}

// NUOVE FUNZIONI PER IL MATTO E VALUTAZIONI SPECIALI
pub const INFINITO: i32 = 100_000;
pub const MATTO: i32 = 50_000;

/// Punteggio di matto
pub fn punteggio_matto(ply: u32) -> i32 {
    MATTO - ply as i32
}

/// Punteggio di matto perso
pub fn punteggio_matto_perso(ply: u32) -> i32 {
    -MATTO + ply as i32
}

/// Controlla se il punteggio è un matto
pub fn e_punteggio_matto(punteggio: i32) -> bool {
    punteggio.abs() >= MATTO - 1000
}

/// Funzione helper per ottenere il punteggio di matto in ply
pub fn matto_in(ply: u32) -> i32 {
    MATTO - ply as i32
}

/// Funzione helper per ottenere il punteggio di matto subito
pub fn matto_subito(ply: u32) -> i32 {
    -MATTO + ply as i32
}