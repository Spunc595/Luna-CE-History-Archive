mod board;
mod attacks;
mod movegen;
mod evaluation;
mod search;
mod uci;
mod tt;
mod zobrist;
mod nnue;
mod book;

use nnue::LunaNNUE;
use std::path::Path;

fn main() {
    // 1. Tenta il caricamento della rete neurale all'avvio
    let nnue_path = "luna.nnue";
    let mut nnue_instance = None;

    if Path::new(nnue_path).exists() {
        // Modificato: usiamo Some(net) invece di Ok(net) perché load restituisce Option
        match LunaNNUE::load(nnue_path) {
            Some(net) => {
                println!("info string Luna NNUE caricata con successo da {}", nnue_path);
                nnue_instance = Some(net);
            }
            None => {
                println!("info string Errore: Il file NNUE esiste ma non è stato possibile caricarlo correttamente.");
            }
        }
    } else {
        println!("info string Attenzione: file {} non trovato. Luna userà la valutazione classica.", nnue_path);
    }

    // 2. Passa l'istanza al loop UCI
    uci::uci_loop(nnue_instance);
}