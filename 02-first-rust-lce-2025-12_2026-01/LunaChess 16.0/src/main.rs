// src/main.rs

mod board;
mod attacks;
mod movegen;
mod evaluation; // Punta alla cartella evaluation/mod.rs
mod search;
mod uci;
mod nnue;

#[macro_use]
extern crate lazy_static;

fn main() {
    // 1. Inizializzazione della rete neurale
    // Usiamo Network::carica come suggerito dal compilatore
    let net = crate::nnue::Network::carica("luna_net.nnue");

    // 2. Messaggio di avvio nel terminale
    eprintln!("Luna Chess Engine 16 - Valutazione Tapered HCE");

    // 3. Avvio del loop UCI
    uci::uci_loop(&net);
}