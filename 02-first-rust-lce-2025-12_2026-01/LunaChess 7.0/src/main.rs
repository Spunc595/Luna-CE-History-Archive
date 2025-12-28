// src/main.rs

mod board;
mod attacks;     // Assicurati che questo modulo esista
mod movegen;
mod evaluation;
mod transposition;
mod search;
mod uci;

use uci::UciEngine;

fn main() {
    let mut engine = UciEngine::nuova();
    engine.run();
}