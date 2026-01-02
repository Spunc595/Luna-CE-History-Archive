use std::io::{self, BufRead};
use std::sync::{Arc, RwLock};
use crate::board::Scacchiera;
use crate::search::Motore;
use crate::nnue::Network;
use crate::transposition::TranspositionTable;

mod board; mod movegen; mod attacks; mod evaluation; mod search; mod transposition; mod nnue;

fn main() {
    let nnue_net = Arc::new(Network::carica("luna_net.nnue"));
    let tt = Arc::new(RwLock::new(TranspositionTable::new(64)));
    let mut scacchiera = Scacchiera::nuova();
    scacchiera.inizializza_acc_nnue(&nnue_net);
    let mut motore = Motore { nodi: 0, start_time: std::time::Instant::now(), limit_ms: 0, stopped: false, tt: tt.clone(), nnue: nnue_net.clone() };

    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let msg = line.unwrap();
        let parts: Vec<&str> = msg.split_whitespace().collect();
        if parts.is_empty() { continue; }
        match parts[0] {
            "uci" => { println!("id name Luna Chess NNUE\nid author Gemini\nuciok"); }
            "isready" => println!("readyok"),
            "ucinewgame" => { scacchiera = Scacchiera::nuova(); scacchiera.inizializza_acc_nnue(&nnue_net); }
            "position" => {
                if parts[1] == "startpos" { scacchiera = Scacchiera::nuova(); }
                scacchiera.inizializza_acc_nnue(&nnue_net);
                if parts.len() > 2 && parts[2] == "moves" {
                    for m_str in &parts[3..] {
                        let mut moves = Vec::new(); crate::movegen::genera_tutte_mosse_pseudo_legali(&scacchiera, &mut moves);
                        if let Some(m) = moves.into_iter().find(|x| x.to_string() == *m_str) { scacchiera.esegui_mossa(&m, &nnue_net); }
                    }
                }
            }
            "go" => { if let Some(m) = motore.trova_mossa_migliore(&mut scacchiera, 2000) { println!("bestmove {}", m); } }
            "quit" => break,
            _ => {}
        }
    }
}