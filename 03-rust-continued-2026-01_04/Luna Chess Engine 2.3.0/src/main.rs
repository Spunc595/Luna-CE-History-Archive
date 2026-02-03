mod attacks;
mod board;
mod book;
mod evaluation;
mod movegen;
mod nnue;
mod search;
mod tt;
mod uci;
mod zobrist;

use std::env;
use crate::uci::UCI;
use crate::zobrist::ZobristKeys;
use crate::board::Scacchiera;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    // Inizializza Zobrist Keys globali
    let _ = ZobristKeys::default();

    if args.len() > 1 && args[1] == "bench" {
        run_bench();
    } else {
        let mut uci = UCI::new();
        uci.run_loop();
    }
}

fn run_bench() {
    println!("Running benchmark...");
    let positions = [
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
    ];

    let start = std::time::Instant::now();
    let mut nodes = 0;

    for (i, pos) in positions.iter().enumerate() {
        println!("Position {}", i + 1);
        let mut board = Scacchiera::from_fen(pos, &ZobristKeys::default());
        let count = perft(&mut board, 5);
        nodes += count;
        println!("Nodes: {}", count);
    }

    let elapsed = start.elapsed().as_millis();
    println!("Total nodes: {}", nodes);
    println!("Time: {} ms", elapsed);
    println!("NPS: {}", nodes as u128 * 1000 / elapsed.max(1));
}

fn perft(board: &mut Scacchiera, depth: i32) -> u64 {
    if depth == 0 { return 1; }

    let moves = board.genera_mosse_legali(&ZobristKeys::default());
    let mut nodes = 0;

    for m in moves {
        let mut new_board = board.clone();
        if new_board.esegui_mossa(&m, &ZobristKeys::default()) {
            nodes += perft(&mut new_board, depth - 1);
        }
    }
    nodes
}