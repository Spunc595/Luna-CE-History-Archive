pub mod params; // Dichiariamo il modulo params
pub mod pst;    // Dichiariamo il modulo pst

use crate::board::Scacchiera;
use crate::nnue::Network;

pub fn valuta(s: &Scacchiera, net: &Network) -> i16 {
    // 1. Punteggio PST (Materiale + Posizione) calcolato usando i params
    let pst_score = pst::valuta_pst(s);

    // 2. Punteggio NNUE (Rete Neurale)
    let nnue_score = net.valuta(s);

    // 3. Logica Ibrida
    // Se la NNUE restituisce 0 (file mancante o errore), ci affidiamo totalmente alle tabelle PST.
    if nnue_score == 0 {
        return pst_score;
    }

    // Mix: 80% Rete Neurale, 20% Tabelle classiche.
    // Questo stabilizza il gioco posizionale.
    (nnue_score * 8 + pst_score * 2) / 10
}