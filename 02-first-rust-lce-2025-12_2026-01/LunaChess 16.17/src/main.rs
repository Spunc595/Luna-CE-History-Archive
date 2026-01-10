mod board;
mod attacks;
mod movegen;
mod evaluation;
mod search;
mod tt;
mod zobrist;
mod uci;

fn main() {
    // Luna Chess Engine 16.17
    uci::uci_loop();
}