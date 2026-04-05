mod board;
mod movegen;
mod attacks;
mod magic;
mod zobrist;
mod nnue;
mod evaluation;
mod search;
mod tt;
mod book;
mod uci;

use crate::nnue::LunaNNUE;
use crate::uci::UCI;

fn main() {
    // Inizializzazione tabelle precalcolate
    crate::magic::init_magic();
    
    // Caricamento rete neurale
    let nnue = LunaNNUE::load("luna.nnue");
    if nnue.is_some() {
        println!("info string NNUE loaded successfully");
    } else {
        println!("info string NNUE not found, using static evaluation");
    }

    // Avvio interfaccia UCI
    let mut uci_interface = UCI::new(nnue);
    println!("Luna Engine v7.2 - Ready");
    uci_interface.run_loop();
}