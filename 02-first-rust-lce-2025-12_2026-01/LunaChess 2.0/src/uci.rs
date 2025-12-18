use std::io::{self, BufRead};
use std::time::{Instant, Duration};
use crate::board::Scacchiera;
use crate::search::ricerca_migliore_mossa;

pub struct UciHandler {
    scacchiera: Scacchiera,
    tempo_rimanente_bianco: Duration,
    tempo_rimanente_nero: Duration,
    incremento_bianco: Duration,
    incremento_nero: Duration,
    tempo_massimo_per_mossa: Option<Duration>,
}

impl UciHandler {
    pub fn new() -> Self {
        UciHandler {
            scacchiera: Scacchiera::nuova(),
            tempo_rimanente_bianco: Duration::from_secs(300), // 5 minuti
            tempo_rimanente_nero: Duration::from_secs(300),
            incremento_bianco: Duration::from_secs(0),
            incremento_nero: Duration::from_secs(0),
            tempo_massimo_per_mossa: None,
        }
    }
    
    pub fn run(&mut self) {
        let stdin = io::stdin();
        
        println!("id name MotoreScacchiRust v1.0");
        println!("id author Daniele");
        println!("uciok");
        
        for linea in stdin.lock().lines() {
            match linea {
                Ok(comando) => {
                    if comando.trim() == "quit" {
                        break;
                    }
                    self.gestisci_comando(&comando);
                }
                Err(_) => break,
            }
        }
    }
    
    fn gestisci_comando(&mut self, comando: &str) {
        let parti: Vec<&str> = comando.split_whitespace().collect();
        
        if parti.is_empty() {
            return;
        }
        
        match parti[0] {
            "uci" => {
                println!("id name MotoreScacchiRust");
                println!("id author Daniele");
                println!("uciok");
            }
            "isready" => {
                println!("readyok");
            }
            "ucinewgame" => {
                self.scacchiera = Scacchiera::nuova();
            }
            "position" => self.comando_position(&parti),
            "go" => self.comando_go(&parti),
            "stop" => (), // Per ora ignoriamo il comando stop
            "quit" => std::process::exit(0),
            _ => (), // Ignora comandi sconosciuti
        }
    }
    
    fn comando_position(&mut self, parti: &[&str]) {
        if parti.len() < 2 {
            return;
        }
        
        match parti[1] {
            "startpos" => {
                self.scacchiera = Scacchiera::nuova();
                if parti.len() > 2 && parti[2] == "moves" {
                    self.applica_mosse(&parti[3..]);
                }
            }
            "fen" => {
                if parti.len() < 8 {
                    return;
                }
                let fen = parti[2..8].join(" ");
                if let Ok(scacchiera) = Scacchiera::da_fen(&fen) {
                    self.scacchiera = scacchiera;
                }
                if parti.len() > 8 && parti[8] == "moves" {
                    self.applica_mosse(&parti[9..]);
                }
            }
            _ => (),
        }
    }
    
    fn comando_go(&mut self, parti: &[&str]) {
        let mut tempo_mossa = Duration::from_secs(5); // Default 5 secondi
        
        // Parsing dei parametri temporali UCI
        for i in 0..parti.len() {
            match parti[i] {
                "movetime" if i + 1 < parti.len() => {
                    if let Ok(ms) = parti[i + 1].parse::<u64>() {
                        tempo_mossa = Duration::from_millis(ms);
                    }
                }
                "wtime" if i + 1 < parti.len() => {
                    if let Ok(ms) = parti[i + 1].parse::<u64>() {
                        self.tempo_rimanente_bianco = Duration::from_millis(ms);
                    }
                }
                "btime" if i + 1 < parti.len() => {
                    if let Ok(ms) = parti[i + 1].parse::<u64>() {
                        self.tempo_rimanente_nero = Duration::from_millis(ms);
                    }
                }
                "winc" if i + 1 < parti.len() => {
                    if let Ok(ms) = parti[i + 1].parse::<u64>() {
                        self.incremento_bianco = Duration::from_millis(ms);
                    }
                }
                "binc" if i + 1 < parti.len() => {
                    if let Ok(ms) = parti[i + 1].parse::<u64>() {
                        self.incremento_nero = Duration::from_millis(ms);
                    }
                }
                _ => (),
            }
        }
        
        // Calcola tempo ottimale per la mossa
        let tempo_disponibile = match self.scacchiera.colore_attivo() {
            crate::board::Colore::Bianco => self.tempo_rimanente_bianco / 40, // Divisore tipico
            crate::board::Colore::Nero => self.tempo_rimanente_nero / 40,
        };
        
        tempo_mossa = tempo_mossa.min(tempo_disponibile);
        
        // Usa la ricerca iterativa con tempo limitato
        use crate::search::ricerca_iterativa;
        let risultato = ricerca_iterativa(&self.scacchiera, tempo_mossa.as_millis() as u64);
        
        if let Some(mossa) = risultato.mossa {
            println!("bestmove {}", mossa);
        } else {
            // Nessuna mossa legale - scaccomatto o stallo
            println!("bestmove 0000");
        }
    }
    
    fn applica_mosse(&mut self, mosse: &[&str]) {
        use crate::movegen::genera_mosse;
        
        for &str_mossa in mosse {
            let tutte_mosse = genera_mosse(&self.scacchiera);
            
            if let Some(mossa) = tutte_mosse.iter().find(|m| m.to_string() == str_mossa) {
                self.scacchiera.esegui_mossa(mossa);
            }
        }
    }
}