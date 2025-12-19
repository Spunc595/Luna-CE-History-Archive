mod board;
mod movegen;
mod search;
mod evaluation;
mod uci;
mod transposition;
mod zobrist;

use uci::UciHandler;

fn main() {
    println!("Motore di Scacchi Rust v1.0");
    let mut uci_handler = UciHandler::new();
    uci_handler.run();
}