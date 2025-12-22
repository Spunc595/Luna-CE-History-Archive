use crate::board::Scacchiera;
use crate::movegen;
use crate::search::Motore;
use std::io::{self, BufRead, Write};
use std::time::Duration;

pub struct UciEngine {
    motore: Motore,
    scacchiera: Scacchiera,
}

impl UciEngine {
    pub fn nuova() -> Self {
        UciEngine {
            motore: Motore::nuovo(),
            scacchiera: Scacchiera::nuova(),
        }
    }
    
    pub fn esegui(&mut self) {
        let stdin = io::stdin();
        
        println!("id name MotoreScacchiRust");
        println!("id author Daniele");
        println!("option name UseOpeningBook type check default true");
        println!("uciok");
        
        for line in stdin.lock().lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => continue,
            };
            let parts: Vec<&str> = line.split_whitespace().collect();
            
            if parts.is_empty() {
                continue;
            }
            
            match parts[0] {
                "isready" => {
                    println!("readyok");
                }
                "setoption" => {
                    self.gestisci_setoption(&parts);
                }
                "position" => {
                    self.interpreta_posizione(&parts);
                }
                "go" => {
                    self.cerca_mossa(&parts);
                }
                "stop" => {
                    self.motore.stop();
                }
                "quit" => {
                    break;
                }
                "uci" => {
                    println!("id name MotoreScacchiRust");
                    println!("id author Daniele");
                    println!("option name UseOpeningBook type check default true");
                    println!("uciok");
                }
                _ => {
                    // Ignora comandi sconosciuti
                }
            }
            
            io::stdout().flush().unwrap();
        }
    }
    
    fn gestisci_setoption(&mut self, parts: &[&str]) {
        // Formato: setoption name <nome> value <valore>
        if parts.len() >= 4 && parts[1] == "name" {
            let option_name = parts[2];
            
            if parts.len() >= 6 && parts[3] == "value" {
                let option_value = parts[4];
                
                match option_name {
                    "UseOpeningBook" => {
                        if let Ok(use_book) = option_value.parse::<bool>() {
                            self.motore.attiva_opening_book(use_book);
                            println!("info string Opening book {}", 
                                     if use_book { "attivato" } else { "disattivato" });
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    
    fn interpreta_posizione(&mut self, parts: &[&str]) {
        if parts.len() < 2 {
            return;
        }
        
        match parts[1] {
            "startpos" => {
                self.scacchiera = Scacchiera::nuova();
                // Applica mosse successive se presenti
                if parts.len() > 2 && parts[2] == "moves" {
                    for mossa_str in &parts[3..] {
                        if let Some(mossa) = movegen::parse_mossa_da_stringa(&self.scacchiera, mossa_str) {
                            let mut temp_scacchiera = self.scacchiera.clone();
                            if movegen::esegui_mossa_completa(&mut temp_scacchiera, &mossa) {
                                self.scacchiera = temp_scacchiera;
                            }
                        }
                    }
                }
            }
            "fen" => {
                if parts.len() >= 8 {
                    let fen = parts[2..8].join(" ");
                    if let Ok(scacchiera) = Scacchiera::da_fen(&fen) {
                        self.scacchiera = scacchiera;
                        // Applica mosse successive se presenti
                        if parts.len() > 8 && parts[8] == "moves" {
                            for mossa_str in &parts[9..] {
                                if let Some(mossa) = movegen::parse_mossa_da_stringa(&self.scacchiera, mossa_str) {
                                    let mut temp_scacchiera = self.scacchiera.clone();
                                    if movegen::esegui_mossa_completa(&mut temp_scacchiera, &mossa) {
                                        self.scacchiera = temp_scacchiera;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
    
    fn cerca_mossa(&mut self, parts: &[&str]) {
        let mut tempo_massimo = Duration::from_secs(10); // Default 10 secondi
        
        // Analizza i parametri UCI
        for i in 0..parts.len() {
            match parts[i] {
                "movetime" if i + 1 < parts.len() => {
                    if let Ok(millis) = parts[i + 1].parse::<u64>() {
                        tempo_massimo = Duration::from_millis(millis);
                    }
                }
                "wtime" | "btime" | "winc" | "binc" => {
                    // Gestione tempo per partita a tempo (da implementare)
                }
                "depth" if i + 1 < parts.len() => {
                    if let Ok(profondita) = parts[i + 1].parse::<u8>() {
                        self.motore.imposta_profondita_massima(profondita);
                    }
                }
                "infinite" => {
                    tempo_massimo = Duration::from_secs(3600); // 1 ora per "infinite"
                }
                _ => {}
            }
        }
        
        // Cerca la mossa migliore
        if let Some(mossa) = self.motore.trova_mossa_migliore(&self.scacchiera, tempo_massimo) {
            println!("bestmove {}", mossa);
        } else {
            println!("bestmove 0000"); // Mossa nulla se nessuna mossa disponibile
        }
    }
}