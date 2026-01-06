mod board;
mod attacks;
mod movegen;
mod move_ordering;
mod nnue;
mod evaluation;
mod search;
mod transposition;
mod uci;

use uci::UciEngine;

fn main() {
    // Inizializziamo il gestore UCI
    let mut engine = UciEngine::nuova();
    
    // Avviamo il motore. Da questo momento il motore ascolta i comandi
    // (come "uci", "isready", "position", "go") inviati da Arena.
    engine.run();
}