use crate::board::{Scacchiera, Colore};
use crate::movegen::{genera_e_filtra_mosse, esegui_mossa_completa, parse_mossa_da_stringa};
use crate::search::Motore;
use std::io::{self, Write};
use std::time::Duration;

pub struct Game {
    pub board: Scacchiera,
    pub history: Vec<Scacchiera>,  // Salva le scacchiere precedenti
    pub engine_color: Option<Colore>,
    pub motore: Motore,
    pub ply_count: u32,
    pub game_over: bool,
}

impl Game {
    pub fn nuovo() -> Self {
        Game {
            board: Scacchiera::nuova(),
            history: Vec::new(),
            engine_color: None,
            motore: Motore::nuovo(),
            ply_count: 0,
            game_over: false,
        }
    }
    
    pub fn from_fen(fen: &str) -> Result<Self, String> {
        match Scacchiera::da_fen(fen) {
            Ok(board) => {
                let mut game = Game::nuovo();
                game.board = board;
                Ok(game)
            }
            Err(e) => Err(e),
        }
    }
    
    pub fn umano_vs_engine(&mut self, umano_bianco: bool) {
        self.engine_color = if umano_bianco {
            Some(Colore::Nero)  // Engine gioca col nero
        } else {
            Some(Colore::Bianco) // Engine gioca col bianco
        };
        
        println!("PARTITA UMANO vs ENGINE");
        println!("Umano: {:?}", if umano_bianco { Colore::Bianco } else { Colore::Nero });
        println!("Engine: {:?}", self.engine_color.unwrap());
        println!("{}", "=".repeat(50));
        
        self.gioca_interattivo();
    }
    
    pub fn engine_vs_engine(&mut self, tempo_per_mossa: Duration) {
        println!("PARTITA ENGINE vs ENGINE");
        println!("Tempo per mossa: {:?}", tempo_per_mossa);
        println!("{}", "=".repeat(50));
        
        let mut mossa_numero = 1;
        
        while !self.game_over && mossa_numero <= 100 { // Limite di 100 mosse per sicurezza
            println!("\n=== MOSSA {} ===", mossa_numero);
            self.stampa_stato();
            
            // Controlla se la partita è finita
            if self.e_finito() {
                self.game_over = true;
                break;
            }
            
            // L'engine cerca la migliore mossa
            println!("Engine ({:?}) sta pensando...", self.board.colore_attivo());
            
            let tempo_mossa = if mossa_numero < 10 {
                Duration::from_secs(2) // Più tempo per le prime mosse
            } else {
                tempo_per_mossa
            };
            
            let mossa_engine = self.motore.trova_mossa_migliore(&self.board, tempo_mossa);
            
            if let Some(mossa) = mossa_engine {
                println!("Engine gioca: {}", mossa);
                
                // Salva lo stato prima della mossa
                self.history.push(self.board.clone());
                
                // Esegui la mossa
                if !self.esegui_mossa(&mossa) {
                    println!("ERRORE: Engine ha suggerito una mossa non valida!");
                    break;
                }
                
                mossa_numero += 1;
                self.ply_count += 1;
                
            } else {
                println!("Engine non ha trovato mosse valide!");
                self.game_over = true;
                break;
            }
            
            // Pausa per leggibilità
            if mossa_numero < 10 {
                std::thread::sleep(Duration::from_millis(500));
            }
        }
        
        println!("\n=== PARTITA CONCLUSA ===");
        self.annuncia_risultato();
    }
    
    fn gioca_interattivo(&mut self) {
        let mut mossa_numero = 1;
        
        while !self.game_over {
            println!("\n=== MOSSA {} ===", mossa_numero);
            self.stampa_stato();
            
            // Controlla se la partita è finita
            if self.e_finito() {
                self.game_over = true;
                break;
            }
            
            let turno_corrente = self.board.colore_attivo();
            
            // Controlla se è il turno dell'engine
            if Some(turno_corrente) == self.engine_color {
                println!("Turno dell'ENGINE ({:?})...", turno_corrente);
                
                // Engine cerca la migliore mossa
                let tempo = Duration::from_secs(if mossa_numero < 10 { 3 } else { 5 });
                let mossa_engine = self.motore.trova_mossa_migliore(&self.board, tempo);
                
                if let Some(mossa) = mossa_engine {
                    println!("Engine gioca: {}", mossa);
                    
                    // Salva lo stato prima della mossa
                    self.history.push(self.board.clone());
                    
                    // Esegui la mossa
                    if !self.esegui_mossa(&mossa) {
                        println!("ERRORE: Engine ha suggerito una mossa non valida!");
                        break;
                    }
                    
                    mossa_numero += 1;
                    self.ply_count += 1;
                } else {
                    println!("Engine non ha trovato mosse!");
                    self.game_over = true;
                    break;
                }
            } else {
                // Turno umano
                println!("Turno dell'UMANO ({:?})", turno_corrente);
                self.turno_umano();
                
                // Dopo la mossa umana, passa al turno successivo
                mossa_numero += 1;
                self.ply_count += 1;
            }
        }
        
        println!("\n=== PARTITA CONCLUSA ===");
        self.annuncia_risultato();
    }
    
    fn turno_umano(&mut self) {
        loop {
            // Mostra mosse disponibili
            let mosse = genera_e_filtra_mosse(&self.board);
            
            if mosse.is_empty() {
                println!("Nessuna mossa disponibile!");
                return;
            }
            
            print!("Inserisci mossa (es: 'e2e4', 'undo', 'quit', 'help'): ");
            io::stdout().flush().unwrap();
            
            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();
            let input = input.trim();
            
            match input {
                "quit" | "exit" => {
                    println!("Partita interrotta.");
                    self.game_over = true;
                    return;
                }
                "undo" => {
                    if self.undo_mossa() {
                        println!("Mossa annullata.");
                        return;
                    } else {
                        println!("Nessuna mossa da annullare.");
                        continue;
                    }
                }
                "help" => {
                    self.mostra_aiuto(&mosse);
                    continue;
                }
                _ => {
                    // Prova a parsare la mossa
                    if let Some(mossa) = parse_mossa_da_stringa(&self.board, input) {
                        // Verifica se la mossa è legale
                        if mosse.contains(&mossa) {
                            // Salva lo stato prima della mossa
                            self.history.push(self.board.clone());
                            
                            // Esegui la mossa
                            if self.esegui_mossa(&mossa) {
                                println!("Mossa eseguita: {}", mossa);
                                return;
                            } else {
                                println!("Mossa non valida!");
                            }
                        } else {
                            // Controlla se è una promozione mancante
                            let mosse_str: Vec<String> = mosse.iter()
                                .map(|m| m.to_string())
                                .collect();
                            
                            // Cerca mosse con stesso from-to ma promozione diversa
                            let from_to = format!("{}{}", mossa.da, mossa.a);
                            let mosse_candidate: Vec<String> = mosse_str.iter()
                                .filter(|s| s.starts_with(&from_to))
                                .cloned()
                                .collect();
                            
                            if !mosse_candidate.is_empty() {
                                println!("Possibili mosse:");
                                for m in mosse_candidate {
                                    println!("  {}", m);
                                }
                                println!("Specifica il pezzo di promozione (q, r, b, n)");
                            } else {
                                println!("Mossa non valida!");
                            }
                        }
                    } else {
                        println!("Formato mossa non valido. Usa formato 'e2e4' o 'e7e8q' per promozioni.");
                    }
                }
            }
        }
    }
    
    fn esegui_mossa(&mut self, mossa: &crate::board::Mossa) -> bool {
        let successo = esegui_mossa_completa(&mut self.board, mossa);
        
        if successo {
            // Controlla se la partita è finita dopo questa mossa
            if self.e_finito() {
                self.game_over = true;
            }
        }
        
        successo
    }
    
    fn undo_mossa(&mut self) -> bool {
        if let Some(scacchiera_precedente) = self.history.pop() {
            self.board = scacchiera_precedente;
            self.ply_count -= 1;
            if self.ply_count > 0 {
                self.ply_count -= 1;
            }
            true
        } else {
            false
        }
    }
    
    fn mostra_aiuto(&self, mosse: &[crate::board::Mossa]) {
        println!("\n=== AIUTO ===");
        println!("Comandi disponibili:");
        println!("  <mossa>  - Esempio: e2e4, g1f3, e7e8q (per promozione)");
        println!("  undo     - Annulla l'ultima mossa");
        println!("  quit     - Termina la partita");
        println!("  help     - Mostra questo aiuto");
        
        println!("\nMosse disponibili ({}):", mosse.len());
        for (i, mossa) in mosse.iter().enumerate() {
            print!("{:10}", mossa);
            if (i + 1) % 6 == 0 {
                println!();
            }
        }
        println!();
    }
    
    fn e_finito(&self) -> bool {
        let mosse = genera_e_filtra_mosse(&self.board);
        
        if mosse.is_empty() {
            let colore = self.board.colore_attivo();
            if self.board.re_in_scacco(colore) {
                true // Scaccomatto
            } else {
                true // Stallo
            }
        } else {
            false
        }
    }
    
    fn annuncia_risultato(&self) {
        let mosse = genera_e_filtra_mosse(&self.board);
        
        if mosse.is_empty() {
            let colore = self.board.colore_attivo();
            
            if self.board.re_in_scacco(colore) {
                let vincitore = colore.opposto();
                println!("SCACCOMATTO! Vince il {:?}", vincitore);
            } else {
                println!("STALLO! Partita patta.");
            }
        } else {
            println!("Partita interrotta dopo {} plies.", self.ply_count);
        }
        
        println!("FEN finale: {}", self.board.a_fen());
    }
    
    pub fn stampa_stato(&self) {
        println!("{}", self.board);
        
        let colore = self.board.colore_attivo();
        println!("Turno: {:?}", colore);
        
        // Controlla se il re è sotto scacco
        if self.board.re_in_scacco(colore) {
            println!("RE IN SCACCO!");
        }
        
        // Mostra contatori
        println!("Numero mossa: {}", self.board.numero_mossa());
        println!("Contatore 50 mosse: {}", self.board.contatore_semimosse());
        
        // Mostra en passant se presente
        if let Some(ep) = self.board.en_passant() {
            println!("En passant possibile su: {}", ep);
        }
        
        // Mostra diritti arrocco
        let diritti = self.board.diritti_arrocco();
        println!("Diritti arrocco: {} {} {} {}", 
            if diritti.bianco_lato_re { "K" } else { "-" },
            if diritti.bianco_lato_regina { "Q" } else { "-" },
            if diritti.nero_lato_re { "k" } else { "-" },
            if diritti.nero_lato_regina { "q" } else { "-" });
    }
    
    pub fn gioca_puzzle(&mut self, fen: &str, soluzione: &[&str]) -> bool {
        println!("=== PUZZLE ===");
        
        if let Ok(board) = Scacchiera::da_fen(fen) {
            self.board = board;
            self.history.clear();
            
            let mut mossa_numero = 1;
            let mosse_soluzione: Vec<crate::board::Mossa> = soluzione.iter()
                .filter_map(|s| parse_mossa_da_stringa(&self.board, s))
                .collect();
            
            println!("Posizione:");
            self.stampa_stato();
            println!("\nRisolvi il puzzle! {} mosse.", mosse_soluzione.len());
            
            while mossa_numero <= mosse_soluzione.len() {
                println!("\nMossa {} di {}", mossa_numero, mosse_soluzione.len());
                
                let mosse_disponibili = genera_e_filtra_mosse(&self.board);
                if mosse_disponibili.is_empty() {
                    println!("Nessuna mossa disponibile!");
                    return false;
                }
                
                print!("Tua mossa: ");
                io::stdout().flush().unwrap();
                
                let mut input = String::new();
                io::stdin().read_line(&mut input).unwrap();
                let input = input.trim();
                
                if input == "quit" {
                    return false;
                }
                
                if let Some(mossa_giocatore) = parse_mossa_da_stringa(&self.board, input) {
                    let mossa_corretta = &mosse_soluzione[mossa_numero - 1];
                    
                    if mossa_giocatore == *mossa_corretta {
                        println!("Corretto!");
                        self.esegui_mossa(&mossa_giocatore);
                        mossa_numero += 1;
                    } else {
                        println!("Sbagliato! La mossa corretta era: {}", mossa_corretta);
                        println!("Puzzle fallito.");
                        return false;
                    }
                } else {
                    println!("Mossa non valida!");
                }
            }
            
            println!("\nComplimenti! Puzzle risolto correttamente!");
            true
        } else {
            println!("FEN non valido!");
            false
        }
    }
    
    pub fn stampa_storia(&self) {
        println!("=== STORIA MOSSE ===");
        
        // Ricostruisci la partita dalla storia
        let mut mosse = Vec::new();
        
        // Simula la partita dall'inizio
        let mut scacchiera = Scacchiera::nuova();
        
        for board in &self.history {
            // Troviamo la mossa che porta da scacchiera a board
            let mosse_candidate = genera_e_filtra_mosse(&scacchiera);
            
            for mossa in mosse_candidate {
                let mut test_board = scacchiera.clone();
                if esegui_mossa_completa(&mut test_board, &mossa) && test_board.hash() == board.hash() {
                    mosse.push(mossa);
                    scacchiera = test_board;
                    break;
                }
            }
        }
        
        // Stampa le mosse in formato PG
        for (i, chunk) in mosse.chunks(2).enumerate() {
            print!("{}. ", i + 1);
            for (j, mossa) in chunk.iter().enumerate() {
                if j == 0 {
                    print!("{} ", mossa);
                } else {
                    print!("{} ", mossa);
                }
            }
            println!();
        }
    }
}

impl Default for Game {
    fn default() -> Self {
        Self::nuovo()
    }
}