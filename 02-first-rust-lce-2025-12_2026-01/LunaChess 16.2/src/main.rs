// src/main.rs
mod board;
mod attacks;
mod movegen;
mod evaluation;
mod search;
mod uci;
mod nnue;

fn main() {
    let net = crate::nnue::Network::carica("luna_net.nnue");
    eprintln!("Luna 16 - Avvio modulo di valutazione integrato");
    uci::uci_loop(&net);
}