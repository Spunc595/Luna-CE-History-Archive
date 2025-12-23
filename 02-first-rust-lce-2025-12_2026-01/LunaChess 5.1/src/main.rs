mod board;
mod evaluation;
mod endgame;
mod game;
mod movegen;
mod openingbook;
mod search;
mod tables;
mod test_engine;
mod transposition;
mod UCI;
mod zobrist;

use crate::UCI::UCIHandler;

fn main() {
    let mut uci = UCIHandler::new();
    uci.run();
}