use std::io::{self, BufRead};
use std::sync::{Arc, Mutex};
use crate::board::Scacchiera;
use crate::search::Motore;
use crate::transposition::TranspositionTable;

pub struct UciEngine {
    motore: Motore,
    board: Scacchiera,
    tt: Arc<Mutex<TranspositionTable>>, // Teniamo un riferimento per il resize
}

impl UciEngine {
    pub fn nuova() -> Self {
        // 1. Creiamo la Transposition Table (Default 64MB)
        // 2. La avvolgiamo in un Mutex (per sicurezza thread)
        // 3. La avvolgiamo in un Arc (per condividerla)
        let tt_inner = TranspositionTable::new(64);
        let tt = Arc::new(Mutex::new(tt_inner));
        
        // Passiamo la TT condivisa al motore
        let mut motore = Motore::nuovo(tt.clone());
        
        // Carichiamo la configurazione NNUE di default (HCE)
        motore.set_nnue_config("luna_net.nnue".to_string(), true);

        UciEngine {
            motore,
            board: Scacchiera::nuova(),
            tt,
        }
    }

    pub fn run(&mut self) {
        let stdin = io::stdin();
        let mut handle = stdin.lock();
        let mut buffer = String::new();

        loop {
            buffer.clear();
            if handle.read_line(&mut buffer).unwrap() == 0 { break; }
            let command = buffer.trim();
            if command.is_empty() { continue; }

            let parts: Vec<&str> = command.split_whitespace().collect();
            let cmd = parts[0];

            match cmd {
                "uci" => {
                    println!("id name Luna Chess Engine 15.3");
                    println!("id author Danie & Gemini");
                    println!("option name Hash type spin default 64 min 1 max 1024");
                    println!("uciok");
                },
                "isready" => println!("readyok"),
                "ucinewgame" => {
                    // Pulisce la Hash Table per la nuova partita
                    if let Ok(mut tt) = self.tt.lock() {
                        tt.clear();
                    }
                },
                "position" => self.gestisci_position(&parts),
                "go" => self.gestisci_go(&parts),
                "setoption" => self.gestisci_setoption(&parts),
                "quit" => break,
                _ => {}
            }
        }
    }

    fn gestisci_position(&mut self, parts: &[&str]) {
        // Formato: position startpos moves e2e4 ...
        // Oppure: position fen ... moves ...
        
        let mut move_index = 1;
        if parts.len() > 1 && parts[1] == "startpos" {
            self.board = Scacchiera::nuova();
            move_index = 2;
        } else if parts.len() > 1 && parts[1] == "fen" {
            // Uniamo le parti della FEN finché non troviamo "moves"
            let mut fen = String::new();
            while move_index < parts.len() && parts[move_index] != "moves" {
                fen.push_str(parts[move_index]);
                fen.push(' ');
                move_index += 1;
            }
            if let Ok(b) = Scacchiera::da_fen(&fen) {
                self.board = b;
            }
        }

        if move_index < parts.len() && parts[move_index] == "moves" {
            move_index += 1;
            for m_str in &parts[move_index..] {
                let mosse = self.board.genera_mosse();
                if let Some(mv) = mosse.iter().find(|m| m.to_string() == *m_str) {
                    // Passiamo None come rete perché stiamo solo aggiornando la board, 
                    // la valutazione la farà il motore dopo
                    self.board.esegui_mossa_raw(mv);
                }
            }
        }
    }

    fn gestisci_go(&mut self, parts: &[&str]) {
        let mut wtime = 0;
        let mut btime = 0;
        let mut movetime = 0;
        let mut depth = 64; // Default infinito (gestito dal tempo)

        for i in 1..parts.len() {
            match parts[i] {
                "wtime" => if let Ok(val) = parts.get(i+1).unwrap_or(&"0").parse() { wtime = val; },
                "btime" => if let Ok(val) = parts.get(i+1).unwrap_or(&"0").parse() { btime = val; },
                "movetime" => if let Ok(val) = parts.get(i+1).unwrap_or(&"0").parse() { movetime = val; },
                "depth" => if let Ok(val) = parts.get(i+1).unwrap_or(&"64").parse() { depth = val; },
                _ => {}
            }
        }

        let time_limit = if movetime > 0 {
            movetime
        } else {
            // Gestione tempo semplice: usa 1/30 del tempo rimanente
            let my_time = if self.board.turno == crate::board::Colore::Bianco { wtime } else { btime };
            if my_time > 0 { my_time / 30 } else { 1000 }
        };

        // Chiama il motore
        let best = self.motore.trova_mossa_migliore(&mut self.board, time_limit, true);
        
        if let Some(m) = best {
            println!("bestmove {}", m);
        } else {
            // Fallback se non trova nulla (non dovrebbe accadere)
            println!("bestmove 0000");
        }
    }
    
    fn gestisci_setoption(&mut self, parts: &[&str]) {
        // Esempio: setoption name Hash value 128
        if parts.len() >= 5 && parts[2] == "Hash" && parts[3] == "value" {
            if let Ok(mb) = parts[4].parse::<usize>() {
                // Resize della Transposition Table
                // Dato che è in un Mutex e Arc, dobbiamo aggiornarla.
                if let Ok(mut tt) = self.tt.lock() {
                    // Sostituiamo la tabella interna con una nuova
                    *tt = TranspositionTable::new(mb);
                    println!("info string Hash set to {} MB", mb);
                }
            }
        }
    }
}