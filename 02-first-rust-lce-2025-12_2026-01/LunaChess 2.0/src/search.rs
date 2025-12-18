use crate::board::Scacchiera;
use crate::movegen::{genera_mosse, Mossa};
use crate::evaluation::valuta;
use std::time::Instant;

const PROFONDITA_MASSIMA: i32 = 6; // Aumentata per partite reali

pub struct RisultatoRicerca {
    pub mossa: Option<Mossa>,
    pub valore: i32,
}

pub fn ricerca_migliore_mossa(scacchiera: &Scacchiera) -> RisultatoRicerca {
    negamax(scacchiera, 3, -100000, 100000) // Profondità fissa per testing
}

pub fn ricerca_iterativa(scacchiera: &Scacchiera, tempo_massimo_ms: u64) -> RisultatoRicerca {
    let inizio = Instant::now();
    let mut migliore_mossa = None;
    let mut miglior_valore = 0;
    
    for profondita in 1..=PROFONDITA_MASSIMA {
        // Controlla se il tempo è scaduto
        if inizio.elapsed().as_millis() as u64 > tempo_massimo_ms {
            break;
        }
        
        let risultato = negamax(scacchiera, profondita, -100000, 100000);
        migliore_mossa = risultato.mossa;
        miglior_valore = risultato.valore;
        
        // Debug output (opzionale)
        if profondita >= 3 {
            println!("info depth {} score cp {} nodes 0 time {}",
                profondita, 
                miglior_valore,
                inizio.elapsed().as_millis());
        }
        
        // Se troviamo scaccomatto, possiamo fermarci prima
        if miglior_valore.abs() > 9000 {
            break;
        }
    }
    
    RisultatoRicerca {
        mossa: migliore_mossa,
        valore: miglior_valore,
    }
}

fn negamax(scacchiera: &Scacchiera, profondita: i32, alfa: i32, beta: i32) -> RisultatoRicerca {
    if profondita == 0 {
        return RisultatoRicerca {
            mossa: None,
            valore: valuta(scacchiera),
        };
    }

    let mosse = genera_mosse(scacchiera);
    
    if mosse.is_empty() {
        // Scaccomatto o stallo
        if scacchiera.in_scacco(scacchiera.colore_attivo()) {
            // Scaccomatto - valore molto negativo
            return RisultatoRicerca {
                mossa: None,
                valore: -100000 + (PROFONDITA_MASSIMA - profondita),
            };
        } else {
            // Stallo
            return RisultatoRicerca {
                mossa: None,
                valore: 0,
            };
        }
    }

    let mut migliore_mossa = None;
    let mut miglior_valore = -100000;
    let mut alfa_locale = alfa;

    for mossa in mosse {
        let mut nuova_scacchiera = scacchiera.clone();
        nuova_scacchiera.esegui_mossa(&mossa);
        
        let risultato = negamax(&nuova_scacchiera, profondita - 1, -beta, -alfa_locale);
        let valore = -risultato.valore;

        if valore > miglior_valore {
            miglior_valore = valore;
            migliore_mossa = Some(mossa);

            if valore > alfa_locale {
                alfa_locale = valore;
            }

            if alfa_locale >= beta {
                break; // Taglio beta
            }
        }
    }

    RisultatoRicerca {
        mossa: migliore_mossa,
        valore: miglior_valore,
    }
}