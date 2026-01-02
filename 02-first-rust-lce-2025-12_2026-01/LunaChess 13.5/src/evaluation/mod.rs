use crate::board::Scacchiera;
use crate::nnue::Network;

/// Valuta la posizione corrente usando la rete neurale caricata.
/// Restituisce il punteggio dal punto di vista del giocatore di turno.
pub fn valuta_posizione(s: &Scacchiera, net: &Network) -> i16 {
    // La funzione valuta() della rete restituisce già il punteggio 
    // normalizzato in centipedoni (i16).
    net.valuta(s)
}

// Nota: Le funzioni come sicurezza_re o mobilità non servono più qui 
// perché la NNUE le calcola implicitamente attraverso i suoi pesi sinaptici.