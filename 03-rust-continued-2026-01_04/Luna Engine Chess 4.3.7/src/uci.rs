use std::io::{self, BufRead};
use crate::board::{Scacchiera, Colore};
use crate::search::{SearchInfo, iterative_deepening};
use crate::tt::TranspositionTable;
use crate::zobrist::ZobristKeys;
use crate::nnue::LunaNNUE;
use crate::book::OpeningBook;
use std::thread;
use std::sync::{Arc, Mutex};

pub struct UCI {
    board: Scacchiera,
    tt: Arc<Mutex<TranspositionTable>>,
    book: Option<OpeningBook>,
    nnue: Option<Arc<LunaNNUE>>,
    zobrist: ZobristKeys, // CRITICO: Le chiavi devono essere persistenti
}

impl UCI {
    pub fn new(nnue_option: Option<LunaNNUE>) -> Self {
        // Inizializziamo le chiavi una sola volta qui
        let zobrist = ZobristKeys::default();
        let book = OpeningBook::load("book.txt");
        
        if book.is_some() {
            println!("info string Book loaded successfully");
        }

        let nnue = nnue_option.map(Arc::new);

        UCI {
            board: Scacchiera::new_iniziale(&zobrist),
            tt: Arc::new(Mutex::new(TranspositionTable::new(64))), // 64MB di default
            book,
            nnue,
            zobrist,
        }
    }

    pub fn run_loop(&mut self) {
        let stdin = io::stdin();
        let mut handle = stdin.lock();
        let mut buffer = String::new();

        loop {
            buffer.clear();
            if handle.read_line(&mut buffer).unwrap_or(0) == 0 { break; }
            let command = buffer.trim();
            if command == "quit" { break; }
            self.process_command(command);
        }
    }

    fn process_command(&mut self, cmd: &str) {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.is_empty() { return; }

        match parts[0] {
            "uci" => {
                println!("id name Luna 0.6 NNUE HalfKA");
                println!("id author Daniele & Alessandro");
                println!("option name Hash type spin default 64 min 1 max 65536");
                println!("uciok");
            },
            "isready" => println!("readyok"),
            "ucinewgame" => {
                let mut tt = self.tt.lock().unwrap();
                tt.clear();
            },
            "position" => self.parse_position(&parts),
            "go" => self.parse_go(&parts),
            _ => {}
        }
    }

    fn parse_position(&mut self, parts: &[&str]) {
        let mut fen_idx = 0;
        
        // Reset della scacchiera alla posizione base o FEN richiesta
        if parts.len() > 1 && parts[1] == "startpos" {
            self.board = Scacchiera::new_iniziale(&self.zobrist);
            fen_idx = 2;
        } else if parts.len() > 1 && parts[1] == "fen" {
            let mut fen = String::new();
            let mut i = 2;
            while i < parts.len() && parts[i] != "moves" {
                fen.push_str(parts[i]);
                fen.push(' ');
                i += 1;
            }
            self.board = Scacchiera::from_fen(fen.trim(), &self.zobrist);
            fen_idx = i;
        }

        // Inizializzazione Accumulatore NNUE per la posizione di partenza
        if let Some(net) = &self.nnue {
            self.board.inizializza_nnue(net);
        }

        // Applichiamo la lista delle mosse ricevuta dalla GUI
        if fen_idx < parts.len() && parts[fen_idx] == "moves" {
            for m_str in &parts[fen_idx+1..] {
                // Generiamo mosse legali per trovare l'oggetto Mossa corrispondente alla stringa UCI
                let moves = self.board.genera_mosse_legali(&self.zobrist);
                if let Some(m) = moves.iter().find(|m| m.to_uci().to_lowercase() == m_str.to_lowercase()) {
                    // Manteniamo sincronizzato l'accumulatore passandolo a esegui_mossa
                    self.board.esegui_mossa(m, &self.zobrist, self.nnue.as_deref());
                } else {
                    println!("info string ERROR: move {} non trovata!", m_str);
                }
            }
            
            // === REFRESH DI SICUREZZA ===
            // Dopo aver applicato tutte le mosse, facciamo un hard-reset dell'accumulatore.
            // Questo cancella ogni possibile microscopico errore di floating point/intero 
            // accumulato durante l'esecuzione della sequenza.
            if let Some(net) = &self.nnue {
                self.board.inizializza_nnue(net);
            }
        }
    }

    fn parse_go(&mut self, parts: &[&str]) {
        // 1. Controllo Opening Book
        if let Some(book) = &self.book {
            if let Some(book_move) = book.get_move(&self.board) {
                println!("bestmove {}", book_move.to_uci());
                return;
            }
        }

        let mut depth = 64;
        let mut wtime = 0;
        let mut btime = 0;
        let mut winc = 0;
        let mut binc = 0;
        let mut movestogo = 30; 
        let mut movetime = 0;

        for i in 0..parts.len() {
            match parts[i] {
                "depth" => if i + 1 < parts.len() { depth = parts[i+1].parse().unwrap_or(64); },
                "wtime" => if i + 1 < parts.len() { wtime = parts[i+1].parse().unwrap_or(0); },
                "btime" => if i + 1 < parts.len() { btime = parts[i+1].parse().unwrap_or(0); },
                "winc"  => if i + 1 < parts.len() { winc = parts[i+1].parse().unwrap_or(0); },
                "binc"  => if i + 1 < parts.len() { binc = parts[i+1].parse().unwrap_or(0); },
                "movestogo" => if i + 1 < parts.len() { movestogo = parts[i+1].parse().unwrap_or(30); },
                "movetime"  => if i + 1 < parts.len() { movetime = parts[i+1].parse().unwrap_or(0); },
                _ => {}
            }
        }

        // 2. Calcolo limite di tempo
        let time_limit = if movetime > 0 {
            movetime
        } else {
            let (my_time, my_inc) = if self.board.turno == Colore::Bianco { (wtime, winc) } else { (btime, binc) };
            let base_time = my_time / (movestogo as u128 + 2);
            let time_to_use = base_time + (my_inc * 8 / 10);
            time_to_use.saturating_sub(50).max(20) 
        };

        // 3. Lancio thread di ricerca
        let mut board_copy = self.board.clone(); // La copia include l'accumulatore sincronizzato
        let tt_arc = self.tt.clone();
        let nnue_arc = self.nnue.clone(); 
        let zobrist_copy = self.zobrist.clone();
        
        thread::spawn(move || {
            let mut info = SearchInfo::new(time_limit, depth);
            let mut tt = tt_arc.lock().unwrap();
            
            tt.new_search();
            
            let nnue_ref = nnue_arc.as_deref();
            let (best, _) = iterative_deepening(&mut board_copy, &mut info, &mut tt, &zobrist_copy, nnue_ref);
            println!("bestmove {}", best.to_uci());
        });
    }
}