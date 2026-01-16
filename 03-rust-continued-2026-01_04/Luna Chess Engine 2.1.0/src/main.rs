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

fn main() {
    uci::uci_loop();
}