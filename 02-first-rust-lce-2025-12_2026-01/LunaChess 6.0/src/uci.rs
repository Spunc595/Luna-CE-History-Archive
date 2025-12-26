use crate::board::{Scacchiera, Mossa, Colore};
use crate::movegen::{genera_mosse_legali, parse_mossa_da_stringa};
use crate::search::Motore;  // Questo è ora corretto
use std::io::{self, BufRead, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

pub struct UciEngine {
    board: Scacchiera,
    engine: Motore,
    is_running: Arc<AtomicBool>,
    is_ready: bool,
    stop_search: Arc<AtomicBool>,
    last_search_handle: Option<thread::JoinHandle<()>>,
    hash_size: usize,
    threads: usize,
    multipv: u32,
    debug_mode: bool,
}

impl UciEngine {
    pub fn nuova() -> Self {
        UciEngine {
            board: Scacchiera::nuova(),
            engine: Motore::nuovo(),
            is_running: Arc::new(AtomicBool::new(true)),
            is_ready: true,
            stop_search: Arc::new(AtomicBool::new(false)),
            last_search_handle: None,
            hash_size: 32,
            threads: 1,
            multipv: 1,
            debug_mode: false,
        }
    }

    pub fn run(&mut self) {
        let stdin = io::stdin();
        let mut stdout = io::stdout();
        
        println!("RustChess Engine 1.0 by RustChess Team");
        println!("Type 'uci' to start UCI mode");
        
        // Flush output
        let _ = stdout.flush();
        
        for line in stdin.lock().lines() {
            let line = match line {
                Ok(l) => l.trim().to_string(),
                Err(_) => break,
            };
            
            if line.is_empty() {
                continue;
            }
            
            self.process_command(&line);
            
            if !self.is_running.load(Ordering::Relaxed) {
                break;
            }
            
            // Flush output dopo ogni comando
            let _ = stdout.flush();
        }
    }

    fn process_command(&mut self, command: &str) {
        if self.debug_mode {
            println!("info string Received command: {}", command);
        }
        
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return;
        }

        match parts[0] {
            "uci" => self.handle_uci(),
            "isready" => self.handle_isready(),
            "ucinewgame" => self.handle_newgame(),
            "position" => self.handle_position(&parts),
            "go" => self.handle_go(&parts),
            "stop" => self.handle_stop(),
            "quit" => self.handle_quit(),
            "setoption" => self.handle_setoption(&parts),
            "debug" => self.handle_debug(&parts),
            "ponderhit" => self.handle_ponderhit(),
            "print" => self.handle_print(&parts),
            "perft" => self.handle_perft(&parts),
            "d" => self.board.stampa(),
            "eval" => self.handle_eval(),
            "test" => self.test_arena_compatibility(),  // Rimosso Self::
            "help" => self.handle_help(),
            _ => {
                println!("info string Unknown command: '{}'", command);
                println!("info string Type 'help' for available commands");
            }
        }
    }

    fn handle_uci(&self) {
        println!("id name RustChess Engine 1.0");
        println!("id author RustChess Team");
        println!("option name Hash type spin default 32 min 1 max 1024");
        println!("option name Threads type spin default 1 min 1 max 8");
        println!("option name MultiPV type spin default 1 min 1 max 10");
        println!("option name Move Overhead type spin default 10 min 0 max 500");
        println!("option name Contempt type spin default 0 min -100 max 100");
        println!("option name Ponder type check default false");
        println!("uciok");
    }

    fn handle_isready(&self) {
        // Attendiamo brevemente se necessario
        thread::sleep(Duration::from_millis(10));
        println!("readyok");
    }

    fn handle_newgame(&mut self) {
        // Ferma eventuali ricerche in corso
        self.stop_search.store(true, Ordering::Relaxed);
        
        if let Some(handle) = self.last_search_handle.take() {
            let _ = handle.join();
        }
        
        self.board = Scacchiera::nuova();
        self.engine = Motore::nuovo(); // Ricrea il motore invece di resettare
        self.stop_search.store(false, Ordering::Relaxed);
        
        if self.debug_mode {
            println!("info string New game started");
        }
    }

    fn handle_position(&mut self, parts: &[&str]) {
        if parts.len() < 2 {
            println!("info string Error: position command requires arguments");
            return;
        }

        let mut new_board = if parts[1] == "startpos" {
            Scacchiera::nuova()
        } else if parts[1] == "fen" {
            if parts.len() < 3 {
                println!("info string Error: fen requires a FEN string");
                return;
            }
            
            let fen_start = Self::command_after(&parts, 2);  // Aggiunto Self::
            match Scacchiera::da_fen(&fen_start) {
                Ok(board) => board,
                Err(e) => {
                    println!("info string Error parsing FEN: {}", e);
                    println!("info string FEN was: '{}'", fen_start);
                    // Ripristina la scacchiera precedente
                    return;
                }
            }
        } else {
            // Se inizia con una mossa, assumi startpos
            Scacchiera::nuova()
        };

        // Cerca e applica le mosse
        let mut moves_applied = 0;
        for i in 2..parts.len() {
            if parts[i] == "moves" {
                for j in (i + 1)..parts.len() {
                    let move_str = parts[j];
                    match parse_mossa_da_stringa(&new_board, move_str) {
                        Some(mossa) => {
                            if !new_board.esegui_mossa(&mossa) {
                                println!("info string Illegal move: {} (move {})", move_str, moves_applied + 1);
                                // Ripristina la scacchiera precedente
                                return;
                            }
                            moves_applied += 1;
                        }
                        None => {
                            println!("info string Invalid move format: '{}'", move_str);
                            // Ripristina la scacchiera precedente
                            return;
                        }
                    }
                }
                break;
            }
        }
        
        // Se siamo qui, tutte le mosse erano valide
        self.board = new_board;
        
        if self.debug_mode {
            println!("info string Position set successfully ({} moves applied)", moves_applied);
            if moves_applied > 0 {
                println!("info string Current position: {}", self.board.a_fen());
            }
        }
    }

    fn handle_go(&mut self, parts: &[&str]) {
        // Ferma eventuali ricerche precedenti
        self.stop_search.store(true, Ordering::Relaxed);
        if let Some(handle) = self.last_search_handle.take() {
            let _ = handle.join();
        }
        self.stop_search.store(false, Ordering::Relaxed);
        
        // Parse dei parametri di ricerca
        let mut search_params = SearchParams::default();
        let mut i = 1;
        
        while i < parts.len() {
            match parts[i] {
                "movetime" if i + 1 < parts.len() => {
                    if let Ok(ms) = parts[i + 1].parse::<u64>() {
                        search_params.movetime = Some(Duration::from_millis(ms));
                    }
                    i += 2;
                }
                "depth" if i + 1 < parts.len() => {
                    if let Ok(d) = parts[i + 1].parse::<u8>() {
                        search_params.depth = Some(d);
                    }
                    i += 2;
                }
                "wtime" if i + 1 < parts.len() => {
                    if let Ok(ms) = parts[i + 1].parse::<u64>() {
                        search_params.wtime = Some(Duration::from_millis(ms));
                    }
                    i += 2;
                }
                "btime" if i + 1 < parts.len() => {
                    if let Ok(ms) = parts[i + 1].parse::<u64>() {
                        search_params.btime = Some(Duration::from_millis(ms));
                    }
                    i += 2;
                }
                "winc" if i + 1 < parts.len() => {
                    if let Ok(ms) = parts[i + 1].parse::<u64>() {
                        search_params.winc = Some(Duration::from_millis(ms));
                    }
                    i += 2;
                }
                "binc" if i + 1 < parts.len() => {
                    if let Ok(ms) = parts[i + 1].parse::<u64>() {
                        search_params.binc = Some(Duration::from_millis(ms));
                    }
                    i += 2;
                }
                "nodes" if i + 1 < parts.len() => {
                    if let Ok(n) = parts[i + 1].parse::<u64>() {
                        search_params.nodes = Some(n);
                    }
                    i += 2;
                }
                "mate" if i + 1 < parts.len() => {
                    if let Ok(m) = parts[i + 1].parse::<u8>() {
                        search_params.mate = Some(m);
                    }
                    i += 2;
                }
                "infinite" => {
                    search_params.infinite = true;
                    i += 1;
                }
                "ponder" => {
                    search_params.ponder = true;
                    i += 1;
                }
                "searchmoves" => {
                    i += 1;
                    while i < parts.len() && !parts[i].starts_with('-') {
                        search_params.searchmoves.push(parts[i].to_string());
                        i += 1;
                    }
                }
                _ => i += 1,
            }
        }
        
        // Imposta i parametri nel motore
        if let Some(depth) = search_params.depth {
            self.engine.imposta_profondita_massima(depth);
        }
        
        self.engine.set_multipv(self.multipv);
        
        // Calcola il tempo per la ricerca
        let search_time = self.calculate_search_time(&search_params);
        
        if self.debug_mode {
            println!("info string Search parameters:");
            println!("info string   Depth: {:?}", search_params.depth);
            println!("info string   Movetime: {:?}", search_params.movetime);
            println!("info string   Search time: {:?} ms", search_time.as_millis());
            println!("info string   Infinite: {}", search_params.infinite);
            println!("info string   Ponder: {}", search_params.ponder);
        }
        
        // Crea copie per il thread
        let board_clone = self.board.clone();
        let mut engine_clone = Motore::nuovo(); // Usa nuovo motore
        let stop_search = Arc::clone(&self.stop_search);
        
        // Avvia la ricerca in un thread separato
        let handle = thread::spawn(move || {
            let start_time = Instant::now();
            
            // Invia info iniziali
            let result = engine_clone.trova_mossa_migliore(
                &board_clone, 
                search_time,
            );
            
            // Invia la mossa migliore
            match result {
                Some(mossa) => {
                    let elapsed = start_time.elapsed();
                    let nodes_searched = engine_clone.nodi_visitati;
                    let nps = if elapsed.as_secs() > 0 {
                        nodes_searched / elapsed.as_secs() as u64
                    } else {
                        0
                    };
                    
                    println!("info depth {} score cp {} nodes {} time {} nps {}",
                        1, 0, nodes_searched, 
                        elapsed.as_millis(), nps);
                    
                    println!("bestmove {}", mossa);
                }
                None => {
                    // Nessuna mossa trovata (scacco matto o stallo)
                    let mosse = genera_mosse_legali(&board_clone);
                    if let Some(first_move) = mosse.first() {
                        println!("bestmove {}", first_move);
                    } else {
                        println!("bestmove 0000"); // Mossa nulla per matto/stallo
                    }
                }
            }
        });
        
        self.last_search_handle = Some(handle);
    }
    
    fn calculate_search_time(&self, params: &SearchParams) -> Duration {
        if params.infinite {
            return Duration::from_secs(31536000); // 1 anno
        }
        
        if let Some(movetime) = params.movetime {
            return movetime;
        }
        
        // Calcola tempo basato sul tempo rimanente
        let (time_left, inc) = match self.board.colore_attivo() {
            Colore::Bianco => (params.wtime, params.winc),
            Colore::Nero => (params.btime, params.binc),
        };
        
        if let Some(time) = time_left {
            // Formula base: usa ~1/40 del tempo rimanente + incremento
            let moves_to_go = 40; // Stimiamo 40 mosse rimanenti
            let base_time = time.as_millis() as u64 / moves_to_go;
            let increment = inc.map_or(0, |inc| inc.as_millis() as u64);
            
            // Assicurati di non usare più del 90% del tempo rimanente
            let max_time = (time.as_millis() as f64 * 0.9) as u64;
            let calculated = base_time + increment;
            
            Duration::from_millis(calculated.min(max_time).max(50)) // Minimo 50ms
        } else {
            // Default: 1 secondo
            Duration::from_secs(1)
        }
    }

    fn handle_stop(&mut self) {
        self.stop_search.store(true, Ordering::Relaxed);
        
        // Attendi che la ricerca si fermi
        if let Some(handle) = self.last_search_handle.take() {
            let _ = handle.join();
            println!("bestmove 0000"); // Mossa nulla quando fermato
        }
        
        self.stop_search.store(false, Ordering::Relaxed);
    }

    fn handle_quit(&mut self) {
        self.handle_stop(); // Ferma eventuali ricerche
        self.is_running.store(false, Ordering::Relaxed);
        
        if self.debug_mode {
            println!("info string Engine quitting");
        }
    }

    fn handle_setoption(&mut self, parts: &[&str]) {
        if parts.len() < 4 || parts[1] != "name" {
            println!("info string Invalid setoption command");
            return;
        }
        
        let name_start = 2;
        let mut name_end = name_start;
        while name_end < parts.len() && parts[name_end] != "value" {
            name_end += 1;
        }
        
        let name = parts[name_start..name_end].join(" ");
        
        if name_end >= parts.len() || parts[name_end] != "value" {
            // Opzione senza valore (probabilmente checkbox)
            match name.as_str() {
                "Ponder" => {
                    // Gestisci ponder qui se necessario
                }
                _ => {}
            }
            return;
        }
        
        let value_start = name_end + 1;
        if value_start >= parts.len() {
            println!("info string Missing value for option {}", name);
            return;
        }
        
        let value = parts[value_start..].join(" ");
        
        match name.as_str() {
            "Hash" => {
                if let Ok(size_mb) = value.parse::<usize>() {
                    self.hash_size = size_mb.clamp(1, 1024);
                    self.engine.reset_transposition_table(self.hash_size);
                    if self.debug_mode {
                        println!("info string Hash size set to {} MB", self.hash_size);
                    }
                }
            }
            "Threads" => {
                if let Ok(threads) = value.parse::<usize>() {
                    self.threads = threads.clamp(1, 8);
                    if self.debug_mode {
                        println!("info string Threads set to {}", self.threads);
                    }
                }
            }
            "MultiPV" => {
                if let Ok(multipv) = value.parse::<u32>() {
                    self.multipv = multipv.clamp(1, 10);
                    self.engine.set_multipv(self.multipv);
                    if self.debug_mode {
                        println!("info string MultiPV set to {}", self.multipv);
                    }
                }
            }
            "Move Overhead" => {
                // Gestisci overhead qui se necessario
            }
            "Contempt" => {
                if let Ok(contempt) = value.parse::<i32>() {
                    self.engine.set_contempt(contempt.clamp(-100, 100));
                }
            }
            _ => {
                if self.debug_mode {
                    println!("info string Unknown option: {} = {}", name, value);
                }
            }
        }
    }

    fn handle_debug(&mut self, parts: &[&str]) {
        if parts.len() > 1 {
            match parts[1] {
                "on" => {
                    self.debug_mode = true;
                    println!("info string Debug mode ON");
                }
                "off" => {
                    self.debug_mode = false;
                    println!("info string Debug mode OFF");
                }
                _ => {}
            }
        } else {
            self.debug_mode = !self.debug_mode;
            println!("info string Debug mode {}", if self.debug_mode { "ON" } else { "OFF" });
        }
    }

    fn handle_ponderhit(&self) {
        // Per ora gestione base
        if self.debug_mode {
            println!("info string Ponder hit received");
        }
    }

    fn handle_print(&self, parts: &[&str]) {
        if parts.len() > 1 {
            match parts[1] {
                "board" => {
                    println!("{}", self.board);
                }
                "fen" => {
                    println!("Position: {}", self.board.a_fen());
                }
                "moves" => {
                    let mosse = genera_mosse_legali(&self.board);
                    println!("Legal moves ({}):", mosse.len());
                    for (i, mossa) in mosse.iter().enumerate() {
                        println!("  {}. {}", i + 1, mossa);
                    }
                }
                "hash" => {
                    println!("Hash: 0x{:016x}", self.board.hash());
                }
                _ => {
                    println!("info string Unknown print command: {}", parts[1]);
                }
            }
        } else {
            println!("{}", self.board);
        }
    }

    fn handle_perft(&self, parts: &[&str]) {
        let depth = if parts.len() > 1 {
            parts[1].parse::<u32>().unwrap_or(1)
        } else {
            1
        };
        
        println!("info string Running perft depth {}", depth);
        let start = Instant::now();
        let nodes = self.perft(&self.board, depth);
        let elapsed = start.elapsed();
        
        println!("info depth {} nodes {} time {} nps {}",
            depth, nodes, elapsed.as_millis(),
            (nodes as f64 / elapsed.as_secs_f64()) as u64);
    }
    
    fn perft(&self, board: &Scacchiera, depth: u32) -> u64 {
        if depth == 0 {
            return 1;
        }
        
        let mosse = genera_mosse_legali(board);
        if depth == 1 {
            return mosse.len() as u64;
        }
        
        let mut nodes = 0;
        for mossa in mosse {
            let mut new_board = board.clone();
            if new_board.esegui_mossa(&mossa) {
                nodes += self.perft(&new_board, depth - 1);
            }
        }
        
        nodes
    }

    fn handle_eval(&self) {
        use crate::evaluation::valuta_posizione;
        
        let score = valuta_posizione(&self.board);
        let material = self.board.valore_materiale_totale();
        
        println!("info string Static evaluation: {}", score);
        println!("info string Material balance: {}", material);
        println!("info string Position: {}", 
            if score > 0 { "White better" } 
            else if score < 0 { "Black better" } 
            else { "Equal" }
        );
        
        // Controlla scacchi
        if self.board.re_in_scacco(self.board.colore_attivo()) {
            println!("info string {} king is in check", 
                match self.board.colore_attivo() {
                    Colore::Bianco => "White",
                    Colore::Nero => "Black",
                }
            );
        }
    }

    fn test_arena_compatibility(&self) {
        println!("=== TEST COMPATIBILITÀ ARENA ===");
        
        // Test 1: Posizione iniziale
        println!("\n1. Test posizione iniziale:");
        let board = Scacchiera::nuova();
        println!("Turno: {:?}", board.colore_attivo());
        let mosse = genera_mosse_legali(&board);
        println!("Mosse disponibili: {}", mosse.len());
        
        // Test 2: Esegui una mossa
        println!("\n2. Test esecuzione mossa e2e4:");
        let mut board2 = board.clone();
        let mossa = Mossa::from_uci("e2e4").unwrap();
        if board2.esegui_mossa(&mossa) {
            println!("Mossa eseguita, nuovo turno: {:?}", board2.colore_attivo());
        }
        
        // Test 3: Test FEN parsing
        println!("\n3. Test FEN parsing:");
        let fen = "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1";
        match Scacchiera::da_fen(fen) {
            Ok(b) => println!("FEN parsed successfully: {}", b.a_fen()),
            Err(e) => println!("FEN error: {}", e),
        }
        
        // Test 4: Simula flusso UCI
        println!("\n4. Simulazione flusso UCI:");
        println!("uci");
        println!("id name RustChess Engine 1.0");
        println!("id author RustChess Team");
        println!("option name Hash type spin default 32 min 1 max 1024");
        println!("option name Threads type spin default 1 min 1 max 8");
        println!("option name MultiPV type spin default 1 min 1 max 10");
        println!("option name Move Overhead type spin default 10 min 0 max 500");
        println!("option name Contempt type spin default 0 min -100 max 100");
        println!("option name Ponder type check default false");
        println!("uciok");
        println!("isready");
        println!("readyok");
        println!("ucinewgame");
        println!("readyok");
        println!("position startpos moves e2e4 e7e5");
        println!("go movetime 1000");
        println!("info depth 1 score cp 20");
        println!("bestmove g1f3");
        
        println!("\n=== TEST COMPLETATO ===");
    }
    
    fn handle_help(&self) {
        println!("Available commands:");
        println!("  uci              - Enter UCI mode");
        println!("  isready          - Check if engine is ready");
        println!("  ucinewgame       - Start a new game");
        println!("  position [args]  - Set position (startpos or fen)");
        println!("  go [args]        - Start searching (depth, movetime, etc.)");
        println!("  stop             - Stop current search");
        println!("  quit             - Quit engine");
        println!("  setoption [args] - Set engine options");
        println!("  debug [on|off]   - Toggle debug mode");
        println!("  print [board|fen|moves|hash] - Print info");
        println!("  perft [depth]    - Run perft test");
        println!("  eval             - Show static evaluation");
        println!("  d                - Display board");
        println!("  test             - Run compatibility tests");
        println!("  help             - Show this help");
    }
    
    // Aggiunta: funzione helper per command_after
    fn command_after(parts: &[&str], start: usize) -> String {
        parts[start..].join(" ")
    }
}

#[derive(Debug, Default)]
struct SearchParams {
    movetime: Option<Duration>,
    depth: Option<u8>,
    wtime: Option<Duration>,
    btime: Option<Duration>,
    winc: Option<Duration>,
    binc: Option<Duration>,
    nodes: Option<u64>,
    mate: Option<u8>,
    infinite: bool,
    ponder: bool,
    searchmoves: Vec<String>,
}