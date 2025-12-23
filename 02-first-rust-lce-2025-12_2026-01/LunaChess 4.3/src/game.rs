use crate::board::{Scacchiera, Colore};
use crate::movegen::{self, Mossa};
use crate::search::Motore;
use std::io::{self, Write};

pub struct Game {
    board: Scacchiera,
    history: Vec<(Mossa, Scacchiera)>,
    engine_color: Option<Colore>,
    engine: Motore,
}

impl Game {
    pub fn new() -> Self {
        Game {
            board: Scacchiera::nuova(),
            history: Vec::new(),
            engine_color: None,
            engine: Motore::nuovo(),
        }
    }
    
    pub fn human_vs_engine(&mut self, human_is_white: bool) {
        self.engine_color = if human_is_white {
            Some(Colore::Nero)
        } else {
            Some(Colore::Bianco)
        };
        
        self.play_interactive();
    }
    
    pub fn engine_vs_engine(&mut self) {
        println!("PARTITA ENGINE vs ENGINE");
        println!("{}", "=".repeat(50));
        
        let mut move_count = 0;
        let mut engine1 = Motore::nuovo();
        let mut engine2 = Motore::nuovo();
        
        while move_count < 50 { // Limite mosse per sicurezza
            println!("\nMossa {}:", move_count + 1);
            println!("{}", self.board);
            
            let moves = movegen::genera_e_filtra_mosse(&self.board);
            if moves.is_empty() {
                println!("Nessuna mossa legale!");
                break;
            }
            
            // L'engine cerca la migliore mossa
            let tempo = std::time::Duration::from_secs(1);
            let current_engine = if self.board.colore_attivo() == Colore::Bianco {
                &mut engine1
            } else {
                &mut engine2
            };
            
            let best_move = current_engine.trova_mossa_migliore(&self.board, tempo);
            
            if let Some(mv) = best_move {
                println!("Engine gioca: {}", mv);
                
                let board_before = self.board.clone();
                if movegen::esegui_mossa(&mut self.board, &mv) {
                    self.history.push((mv, board_before));
                    move_count += 1;
                } else {
                    println!("Mossa non valida!");
                    break;
                }
            } else {
                println!("Engine non ha trovato mosse!");
                break;
            }
            
            // Pausa per leggibilità
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
        
        println!("\nPartita conclusa dopo {} mosse", move_count);
    }
    
    fn play_interactive(&mut self) {
        println!("TU vs ENGINE");
        println!("{}", "=".repeat(50));
        
        loop {
            println!("\n{}", self.board);
            
            let moves = movegen::genera_e_filtra_mosse(&self.board);
            
            // Controlla se è la mossa dell'engine
            if Some(self.board.colore_attivo()) == self.engine_color {
                println!("Engine sta pensando...");
                
                let tempo = std::time::Duration::from_secs(2);
                let best_move = self.engine.trova_mossa_migliore(&self.board, tempo);
                
                if let Some(mv) = best_move {
                    println!("Engine gioca: {}", mv);
                    movegen::esegui_mossa(&mut self.board, &mv);
                } else {
                    println!("Engine non ha trovato mosse!");
                }
            } else {
                // Mossa umana
                println!("Tocca a te! Mosse disponibili: {}", moves.len());
                print!("Inserisci mossa (es: 'e2e4' o 'quit'): ");
                io::stdout().flush().unwrap();
                
                let mut input = String::new();
                io::stdin().read_line(&mut input).unwrap();
                let input = input.trim();
                
                if input == "quit" {
                    break;
                }
                
                // Cerca la mossa corrispondente
                if let Some(mv) = movegen::parse_mossa_da_stringa(&self.board, input) {
                    if movegen::esegui_mossa(&mut self.board, &mv) {
                        println!("Mossa eseguita: {}", mv);
                    } else {
                        println!("Mossa non valida! Prova ancora.");
                    }
                } else {
                    println!("Formato mossa non valido! Usa formato come 'e2e4'.");
                }
            }
            
            // Controlla scacco matto o stallo
            if moves.is_empty() {
                if self.board.re_in_scacco(self.board.colore_attivo()) {
                    println!("SCACCO MATTO!");
                } else {
                    println!("STALLO!");
                }
                break;
            }
        }
    }
}