pub mod params; // Assicurati che il file params.rs esista
pub mod pst;    // Assicurati che il file pst.rs esista

use crate::board::Scacchiera;
use crate::nnue::Network;

// La funzione deve chiamarsi 'evaluate' per essere vista da search.rs
// Il tipo di ritorno deve essere i32 per compatibilità con Alpha-Beta
pub fn evaluate(s: &Scacchiera, net: &Network) -> i32 {
    
    // 1. Punteggio PST (Materiale + Posizione)
    // Convertiamo subito a i32 per sicurezza
    let pst_score = pst::valuta_pst(s) as i32;

    // 2. Punteggio NNUE (Rete Neurale)
    // Chiamiamo la funzione che definiremo nel modulo nnue (evaluate_nnue o valuta)
    // Qui uso crate::nnue::evaluate_nnue come stabilito prima.
    let nnue_score = crate::nnue::evaluate_nnue(s, net) as i32;

    // 3. Logica Ibrida
    // Se la NNUE è esattamente 0 (possibile indicatore di rete vuota o patta perfetta),
    // e il PST dice che c'è squilibrio, usiamo il PST per sicurezza.
    if nnue_score == 0 && pst_score.abs() > 50 {
        return pst_score;
    }

    // Mix: 80% Rete Neurale, 20% Tabelle classiche.
    let final_score = (nnue_score * 8 + pst_score * 2) / 10;

    final_score
}