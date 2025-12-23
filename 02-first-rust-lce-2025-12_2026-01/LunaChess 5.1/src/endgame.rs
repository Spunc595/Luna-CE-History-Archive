use crate::board::Scacchiera;
use crate::movegen::{self, Mossa};
use crate::transposition::TranspositionTable;
use crate::openingbook::OpeningBook;
use crate::evaluation::Valutatore; // AGGIUNTO: import per Valutatore

// Usa nomi diversi per evitare conflitti
const INFINITO_SEARCH: i32 = 100_000; // CAMBIA NOME
const VITTORIA: i32 = 50_000;
const MAX_NODI: u64 = 1_000_000;

pub struct Motore {
    pub transposition_table: TranspositionTable,
    pub nodi_visitati: u64,
    pub tempo_limite: std::time::Instant,
    pub mossa_migliore: Option<Mossa>,
    pub profondita_massima: u8,
    pub stop_requested: bool,
    pub opening_book: OpeningBook,
    pub usa_opening_book: bool,
    pub valutatore: Valutatore,
}

impl Motore {
    pub fn nuovo() -> Self {
        Motore {
            transposition_table: TranspositionTable::nuova(128),
            nodi_visitati: 0,
            tempo_limite: std::time::Instant::now(),
            mossa_migliore: None,
            profondita_massima: 3,
            stop_requested: false,
            opening_book: OpeningBook::nuova(),
            usa_opening_book: true,
            valutatore: Valutatore::nuovo(),
        }
    }
    
    /// Carica le tabelle Syzygy
    pub fn carica_tabelle_syzygy(&mut self, percorso: &str, pezzi_massimi: u8) -> bool {
        self.valutatore.carica_tabelle_syzygy(percorso, pezzi_massimi)
    }
    
    pub fn imposta_profondita_massima(&mut self, profondita: u8) {
        self.profondita_massima = profondita.min(6);
    }
    
    pub fn attiva_opening_book(&mut self, attiva: bool) {
        self.usa_opening_book = attiva;
    }
    
    pub fn stop(&mut self) {
        self.stop_requested = true;
    }
    
    pub fn trova_mossa_migliore(&mut self, scacchiera: &Scacchiera, tempo: std::time::Duration) -> Option<Mossa> {
        println!("=== IL COMPUTER STA PENSANDO... ===");
        
        // Reset dello stato
        self.nodi_visitati = 0;
        self.tempo_limite = std::time::Instant::now() + tempo;
        self.mossa_migliore = None;
        self.stop_requested = false;
        
        // 1. Controlla l'opening book
        if self.usa_opening_book {
            if let Some(book_move) = self.opening_book.cerca_mossa(scacchiera) {
                println!("=== MOSSA DAL LIBRO DI APERTURA ===");
                println!("Mossa dal libro: {}", book_move);
                return Some(book_move);
            } else {
                println!("Nessuna mossa trovata nel libro di aperture, procedo con la ricerca...");
            }
        }
        
        // 2. Se non c'è mossa nel libro, procedi con la ricerca normale
        let mosse = movegen::genera_e_filtra_mosse(scacchiera);
        
        if mosse.is_empty() {
            println!("Nessuna mossa disponibile!");
            return None;
        }
        
        if mosse.len() == 1 {
            println!("Mossa obbligata: {}", mosse[0]);
            return Some(mosse[0]);
        }
        
        // Ricerca con timeout
        let start_time = std::time::Instant::now();
        let mut best_move = mosse[0];
        let mut best_value = -INFINITO_SEARCH; // USA IL NUOVO NOME
        
        // Ricerca rapida a profondità 1
        for &mossa in &mosse {
            if self.stop_requested || std::time::Instant::now() > self.tempo_limite {
                break;
            }
            
            let mut nuova_scacchiera = scacchiera.clone();
            if movegen::esegui_mossa_completa(&mut nuova_scacchiera, &mossa) {
                let valore = self.valutatore.valuta(&nuova_scacchiera);
                if valore > best_value {
                    best_value = valore;
                    best_move = mossa;
                }
            }
        }
        
        // Ricerca approfondita se c'è tempo
        if start_time.elapsed() < tempo / 2 {
            let mut depth = 2;
            while depth <= self.profondita_massima && !self.stop_requested {
                if std::time::Instant::now() > self.tempo_limite {
                    break;
                }
                
                println!("Ricerca a profondità {}...", depth);
                
                let mut current_best = best_move;
                let mut current_value = -INFINITO_SEARCH; // USA IL NUOVO NOME
                
                // Ordina le mosse: mossa migliore precedente per prima
                let mut mosse_ordinate = mosse.clone();
                mosse_ordinate.sort_by(|a, b| {
                    if *a == best_move { std::cmp::Ordering::Less }
                    else if *b == best_move { std::cmp::Ordering::Greater }
                    else { std::cmp::Ordering::Equal }
                });
                
                for &mossa in &mosse_ordinate {
                    if self.stop_requested || std::time::Instant::now() > self.tempo_limite {
                        break;
                    }
                    
                    let mut nuova_scacchiera = scacchiera.clone();
                    if movegen::esegui_mossa_completa(&mut nuova_scacchiera, &mossa) {
                        let valore = -self.negamax_safe(&nuova_scacchiera, depth - 1, -INFINITO_SEARCH, INFINITO_SEARCH);
                        
                        if valore > current_value {
                            current_value = valore;
                            current_best = mossa;
                        }
                    }
                    
                    // Controllo sicurezza
                    if self.nodi_visitati > MAX_NODI {
                        println!("Limite nodi raggiunto!");
                        self.stop_requested = true;
                        break;
                    }
                }
                
                if !self.stop_requested {
                    best_move = current_best;
                    best_value = current_value;
                    println!("Profondità {}: mossa {} (valore {})", depth, best_move, best_value);
                }
                
                depth += 1;
            }
        }
        
        let elapsed = start_time.elapsed();
        println!("Tempo di ricerca: {:?}", elapsed);
        println!("Nodi visitati: {}", self.nodi_visitati);
        println!("Mossa scelta: {}", best_move);
        
        Some(best_move)
    }
    
    fn negamax_safe(&mut self, scacchiera: &Scacchiera, profondita: u8, mut alfa: i32, beta: i32) -> i32 {
        self.nodi_visitati += 1;
        
        if self.stop_requested || std::time::Instant::now() > self.tempo_limite {
            return self.valutatore.valuta(scacchiera);
        }
        
        if self.nodi_visitati % 10000 == 0 {
            if self.stop_requested || std::time::Instant::now() > self.tempo_limite {
                return self.valutatore.valuta(scacchiera);
            }
        }
        
        if profondita == 0 {
            return self.quiescence_safe(scacchiera, alfa, beta);
        }
        
        let mosse = movegen::genera_e_filtra_mosse(scacchiera);
        
        if mosse.is_empty() {
            // Scacco matto o stallo
            if scacchiera.re_in_scacco(scacchiera.colore_attivo()) {
                return -VITTORIA + (self.profondita_massima as i32 - profondita as i32);
            }
            return 0; // Stallo
        }
        
        let mut best_value = -INFINITO_SEARCH; // USA IL NUOVO NOME
        
        for &mossa in &mosse {
            if self.stop_requested {
                break;
            }
            
            let mut nuova_scacchiera = scacchiera.clone();
            if !movegen::esegui_mossa_completa(&mut nuova_scacchiera, &mossa) {
                continue;
            }
            
            let valore = -self.negamax_safe(&nuova_scacchiera, profondita - 1, -beta, -alfa);
            
            if valore > best_value {
                best_value = valore;
            }
            
            alfa = alfa.max(valore);
            if alfa >= beta {
                break;
            }
        }
        
        best_value
    }
    
    fn quiescence_safe(&mut self, scacchiera: &Scacchiera, mut alfa: i32, beta: i32) -> i32 {
        let stand_pat = self.valutatore.valuta(scacchiera);
        
        if stand_pat >= beta {
            return beta;
        }
        
        if alfa < stand_pat {
            alfa = stand_pat;
        }
        
        // Solo mosse di cattura
        let tutte_mosse = movegen::genera_e_filtra_mosse(scacchiera);
        let mosse_cattura: Vec<Mossa> = tutte_mosse.into_iter()
            .filter(|m| scacchiera.ottieni_pezzo(m.a).is_some())
            .collect();
        
        if mosse_cattura.is_empty() {
            return stand_pat;
        }
        
        let mut best_value = stand_pat;
        
        for &mossa in &mosse_cattura {
            if self.stop_requested {
                break;
            }
            
            let mut nuova_scacchiera = scacchiera.clone();
            if !movegen::esegui_mossa_completa(&mut nuova_scacchiera, &mossa) {
                continue;
            }
            
            let valore = -self.quiescence_safe(&nuova_scacchiera, -beta, -alfa);
            
            if valore > best_value {
                best_value = valore;
            }
            
            if best_value >= beta {
                return beta;
            }
            
            if best_value > alfa {
                alfa = best_value;
            }
        }
        
        best_value
    }
}