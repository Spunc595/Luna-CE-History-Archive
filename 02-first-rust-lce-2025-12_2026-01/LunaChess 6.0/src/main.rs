mod board;
mod movegen;
mod search;
mod uci;
mod evaluation;
mod transposition;
mod zobrist;
mod attacks;
mod constants;

use uci::UciEngine;

fn main() {
    // INIZIALIZZA LE TABELLE MAGIC
    attacks::init_slider_attacks();
    
    let mut engine = UciEngine::nuova();
    engine.run();
}