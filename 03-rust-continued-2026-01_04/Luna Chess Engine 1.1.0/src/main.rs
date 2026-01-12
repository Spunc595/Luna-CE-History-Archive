mod board;
mod attacks;
mod movegen;
mod evaluation;
mod search;
mod uci;
mod tt;
mod zobrist;

fn main() {
    // Avvia il loop UCI
    uci::uci_loop();
}