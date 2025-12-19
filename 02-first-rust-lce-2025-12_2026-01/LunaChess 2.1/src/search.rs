use crate::board::{Scacchiera, Colore};
use crate::movegen::{genera_mosse, Mossa};
use crate::evaluation::valuta_posizione;

#[derive(Debug, Clone, Copy)]
pub struct RisultatoRicerca {
    pub valore: i32,
    pub mossa: Option<Mossa>,
    pub nodi_visitati: u64,
}

pub fn ricerca_miglior_mossa(scacchiera: &Scacchiera, profondita: i32) -> RisultatoRicerca {
    negamax(scacchiera, profondita, -100000, 100000)
}

pub fn negamax(scacchiera: &Scacchiera, profondita: i32, alfa: i32, beta: i32) -> RisultatoRicerca {
    if profondita == 0 {
        return RisultatoRicerca {
            valore: valuta_posizione(scacchiera),
            mossa: None,
            nodi_visitati: 1,
        };
    }
    
    let mosse = genera_mosse(scacchiera);
    if mosse.is_empty() {
        // Scacco matto o stallo
        if scacchiera.re_in_scacco(scacchiera.colore_attivo()) {
            // Scacco matto
            return RisultatoRicerca {
                valore: -100000 + (profondita as i32),
                mossa: None,
                nodi_visitati: 1,
            };
        } else {
            // Stallo
            return RisultatoRicerca {
                valore: 0,
                mossa: None,
                nodi_visitati: 1,
            };
        }
    }
    
    let mut miglior_valore = -100000;
    let mut miglior_mossa = None;
    let mut nodi_totali = 0;
    
    for mossa in mosse {
        let mut nuova_scacchiera = scacchiera.clone();
        crate::movegen::esegui_mossa_completa(&mut nuova_scacchiera, &mossa);
        
        let risultato = negamax(&nuova_scacchiera, profondita - 1, -beta, -alfa);
        let valore = -risultato.valore;
        
        nodi_totali += risultato.nodi_visitati;
        
        if valore > miglior_valore {
            miglior_valore = valore;
            miglior_mossa = Some(mossa);
        }
        
        if miglior_valore > alfa {
            if miglior_valore >= beta {
                // Taglio beta
                break;
            }
        }
    }
    
    RisultatoRicerca {
        valore: miglior_valore,
        mossa: miglior_mossa,
        nodi_visitati: nodi_totali,
    }
}