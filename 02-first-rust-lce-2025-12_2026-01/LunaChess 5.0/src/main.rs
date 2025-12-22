mod board;
mod movegen;
mod evaluation;
mod search;
mod openingbook;
mod transposition;
mod uci;

fn main() {
    let mut engine = uci::UciEngine::nuova();
    engine.esegui();
}