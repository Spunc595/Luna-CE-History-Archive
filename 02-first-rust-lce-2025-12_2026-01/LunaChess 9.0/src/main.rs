mod board;
mod attacks;
mod movegen;
mod evaluation;
mod search;
mod transposition;
mod uci;

use uci::UciEngine;

fn main() {
    // Ottimizzazione per le performance: impostiamo la gestione dei panic in modo silenzioso
    // o gestito per evitare crash durante i tornei.
    std::panic::set_hook(Box::new(|panic_info| {
        eprintln!("Il motore ha riscontrato un errore critico: {:?}", panic_info);
    }));

    let mut engine = UciEngine::nuova();
    engine.run();
}