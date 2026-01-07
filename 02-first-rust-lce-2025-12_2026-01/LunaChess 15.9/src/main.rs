mod board;
mod move_ordering;
mod nnue;
mod search;
mod transposition;
mod uci;
mod attacks;
mod movegen;

fn main() {
    // Inizia il loop UCI
    uci::lancia_loop_uci();
}