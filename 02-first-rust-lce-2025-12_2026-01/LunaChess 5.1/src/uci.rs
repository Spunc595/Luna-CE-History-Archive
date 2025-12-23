use crate::board::Scacchiera;
use crate::movegen;
use crate::search::Motore;
use crate::evaluation::Valutatore;
use std::io::{self, BufRead, Write};
use std::time::Duration;

pub struct UciEngine {
    motore: Motore,
    scacchiera: Scacchiera,
    valutatore: Valutatore,
}

impl UciEngine {
    pub fn nuova() -> Self {
        UciEngine {
            motore: Motore::nuovo(),
            scacchiera: Scacchiera::nuova(),
            valutatore: Valutatore::nuovo(),
        }
    }
    
    pub fn esegui(&mut self) {
        let stdin = io::stdin();
        
        println!("id name MotoreScacchiRust");
        println!("id author Daniele");
        println!("option name UseOpeningBook type check default true");
        println!("option name SyzygyPath type string default ''");
        println!("option name SyzygyMaxPieces type spin default 5 min 3 max 7");
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
                    println!("option name SyzygyPath type string default ''");
                    println!("option name SyzygyMaxPieces type spin default 5 min 3 max 7");
                    println!("uciok");
                }
                "syzygy" => {
                    self.gestisci_syzygy(&parts);
                }
                "eval" => {
                    self.mostra_valutazione();
                }
                "endgame" => {
                    self.mostra_info_finale();
                }
                "d" => {
                    self.mostra_scacchiera();
                }
                _ => {
                    // Ignora comandi sconosciuti
                }
            }
            
            io::stdout().flush().unwrap();
        }
    }
    
    fn gestisci_setoption(&mut self, parts: &[&str]) {
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
                    "SyzygyPath" => {
                        if !option_value.is_empty() {
                            let max_pieces = 5;
                            if self.motore.carica_tabelle_syzygy(option_value, max_pieces) {
                                println!("info string Tabelle Syzygy caricate da: {}", option_value);
                            } else {
                                println!("info string Errore nel caricamento tabelle Syzygy");
                            }
                        }
                    }
                    "SyzygyMaxPieces" => {
                        if let Ok(max_pieces) = option_value.parse::<u8>() {
                            println!("info string Syzygy max pieces impostato a: {}", max_pieces);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    
    fn gestisci_syzygy(&mut self, parts: &[&str]) {
        if parts.len() < 2 {
            println!("info string Uso: syzygy <percorso> [max_pezzi]");
            println!("info string Esempio: syzygy C:/syzygy 5");
            return;
        }
        
        let percorso = parts[1];
        let max_pezzi = if parts.len() >= 3 {
            parts[2].parse().unwrap_or(5)
        } else {
            5
        };
        
        if self.motore.carica_tabelle_syzygy(percorso, max_pezzi) {
            println!("info string Tabelle Syzygy caricate da: {} (max {} pezzi)", percorso, max_pezzi);
        } else {
            println!("info string ERRORE: impossibile caricare tabelle Syzygy da: {}", percorso);
        }
    }
    
    fn mostra_valutazione(&self) {
        let valutazione = self.valutatore.valuta(&self.scacchiera);
        
        // Usa metodi pubblici invece di accedere direttamente ai campi privati
        let e_finale = self.is_finale_position(&self.scacchiera);
        
        println!("info string Valutazione: {}", valutazione);
        println!("info string È finale: {}", e_finale);
        
        let pezzi_totali = self.scacchiera.pezzi_totali();
        println!("info string Pezzi totali: {}", pezzi_totali);
    }
    
    fn mostra_info_finale(&self) {
        let e_finale = self.is_finale_position(&self.scacchiera);
        
        println!("=== INFORMAZIONI FINALE ===");
        println!("Posizione finale: {}", e_finale);
        
        if e_finale {
            // Conta i pezzi per tipo usando i nuovi metodi
            let pedoni = (self.scacchiera.pedoni() as u64).count_ones();
            let cavalli = (self.scacchiera.cavalli() as u64).count_ones();
            let alfieri = (self.scacchiera.alfieri() as u64).count_ones();
            let torri = (self.scacchiera.torri() as u64).count_ones();
            let regine = (self.scacchiera.regine() as u64).count_ones();
            
            println!("Composizione:");
            println!("  Pedoni: {}", pedoni);
            println!("  Cavalli: {}", cavalli);
            println!("  Alfieri: {}", alfieri);
            println!("  Torri: {}", torri);
            println!("  Regine: {}", regine);
            
            // Valutazione specifica per finali
            let valutazione_base = self.valutatore.valuta(&self.scacchiera);
            println!("Valutazione: {}", valutazione_base);
        } else {
            println!("Non è una posizione di finale");
        }
        println!("==========================");
    }
    
    fn is_finale_position(&self, scacchiera: &Scacchiera) -> bool {
        // Logica semplificata: consideriamo finale se ci sono pochi pezzi
        let pezzi_totali = scacchiera.pezzi_totali();
        pezzi_totali <= 10
    }
    
    fn mostra_scacchiera(&self) {
        self.scacchiera.stampa();
        println!("Tratto: {:?}", self.scacchiera.colore_attivo());
        println!("Ply: {}", self.scacchiera.ply());
        
        if self.scacchiera.re_in_scacco(self.scacchiera.colore_attivo()) {
            println!("Re in scacco!");
        }
    }
    
    fn interpreta_posizione(&mut self, parts: &[&str]) {
        if parts.len() < 2 {
            return;
        }
        
        match parts[1] {
            "startpos" => {
                self.scacchiera = Scacchiera::nuova();
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
        
        if self.is_finale_position(&self.scacchiera) {
            println!("info string Posizione riconosciuta come finale");
        }
    }
    
    fn cerca_mossa(&mut self, parts: &[&str]) {
        let mut tempo_massimo = Duration::from_secs(10);
        
        for i in 0..parts.len() {
            match parts[i] {
                "movetime" if i + 1 < parts.len() => {
                    if let Ok(millis) = parts[i + 1].parse::<u64>() {
                        tempo_massimo = Duration::from_millis(millis);
                    }
                }
                "depth" if i + 1 < parts.len() => {
                    if let Ok(profondita) = parts[i + 1].parse::<u8>() {
                        self.motore.imposta_profondita_massima(profondita);
                    }
                }
                "infinite" => {
                    tempo_massimo = Duration::from_secs(3600);
                }
                _ => {}
            }
        }
        
        if let Some(mossa) = self.motore.trova_mossa_migliore(&self.scacchiera, tempo_massimo) {
            println!("bestmove {}", mossa);
        } else {
            println!("bestmove 0000");
        }
    }
}

// Aggiungi questo per compatibilità con main.rs
pub struct UCIHandler {
    engine: UciEngine,
}

impl UCIHandler {
    pub fn new() -> Self {
        Self {
            engine: UciEngine::nuova(),
        }
    }
    
    pub fn run(&mut self) {
        self.engine.esegui();
    }
}