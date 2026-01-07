// src/main.rs
mod board;
mod evaluation;
mod search;
mod uci;
mod nnue;
mod book;
mod zobrist;
mod tt;

#[macro_use]
extern crate lazy_static;

fn main() {
    // Caricamento "safe" della rete: se fallisce, usa una struttura vuota
    // Assicurati che nnue::Network abbia un metodo per generarsi vuota o gestisca l'errore
    let net = crate::nnue::Network::carica("luna_net.nnue");
    
    eprintln!("Luna 16.4 - HCE Engine Active");
    
    // Avvio del loop UCI
    uci::uci_loop(&net);
}