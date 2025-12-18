mod board;
mod movegen;
mod search;
mod evaluation;
mod uci;

use uci::UciHandler;

fn main() {
    println!("RustChess Engine v1.0");
    let mut uci_handler = UciHandler::new();
    uci_handler.run();
}