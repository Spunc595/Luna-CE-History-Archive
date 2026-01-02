pub mod params;
pub mod pst;

use crate::board::Scacchiera;
use crate::nnue::Network;

// --- FUNZIONE PRINCIPALE (con NNUE) ---
// Usata quando la rete è caricata e l'opzione "Use NNUE" è true.
pub fn evaluate(s: &Scacchiera, net: &Network) -> i32 {
    
    // 1. Punteggio PST (Materiale + Posizione)
    let pst_score = pst::valuta_pst(s) as i32;

    // 2. Punteggio NNUE (Rete Neurale)
    // Nota: Assumo che crate::nnue::evaluate_nnue legga l'accumulatore dalla scacchiera
    let nnue_score = crate::nnue::evaluate_nnue(s, net) as i32;

    // 3. Logica Ibrida (Opzionale, ma mantengo la tua logica)
    // Se la NNUE da 0 (o errore), usiamo il PST.
    if nnue_score == 0 && pst_score.abs() > 50 {
        return pst_score;
    }

    // Mix: 80% Rete Neurale, 20% Tabelle classiche.
    // Questo aiuta a coprire "buchi" di conoscenza della rete in posizioni strane,
    // anche se i motori top moderni usano spesso 100% NNUE.
    let final_score = (nnue_score * 8 + pst_score * 2) / 10;

    final_score
}

// --- FUNZIONE CLASSICA (Fallback) ---
// QUESTA è la funzione che mancava per search.rs!
// Viene chiamata se:
// 1. Il file .nnue non viene trovato.
// 2. L'utente disattiva "Use NNUE" da Arena.
pub fn evaluate_classical(s: &Scacchiera) -> i32 {
    pst::valuta_pst(s) as i32
}