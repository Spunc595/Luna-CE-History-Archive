use crate::board::{Scacchiera, Casella, Pezzo};  // Rimuovi Colore
use crate::board::Mossa;
use std::collections::HashMap;
use rand::Rng;

#[derive(Clone)]
pub struct OpeningBook {
    book: HashMap<String, Vec<BookEntry>>,
}

#[derive(Clone, Debug)]
struct BookEntry {
    mossa: String, // in formato UCI
    peso: u32,
}

impl OpeningBook {
    pub fn nuova() -> Self {
        let mut book = HashMap::new();
        
        // Aggiungi tutte le aperture
        Self::aggiungi_aperture(&mut book);
        
        OpeningBook { book }
    }
    
    fn aggiungi_aperture(book: &mut HashMap<String, Vec<BookEntry>>) {
        // === POSIZIONE INIZIALE (Bianco muove) ===
        book.entry("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq -".to_string())
            .or_insert_with(Vec::new)
            .extend(vec![
                BookEntry { mossa: "e2e4".to_string(), peso: 100 },  // e4 (Aperture aperte)
                BookEntry { mossa: "d2d4".to_string(), peso: 90 },   // d4 (Aperture chiuse)
                BookEntry { mossa: "c2c4".to_string(), peso: 70 },   // c4 (Inglese)
                BookEntry { mossa: "g1f3".to_string(), peso: 60 },   // Nf3 (Reti)
            ]);
        
        // === DOPO 1.e4 (Nero muove) ===
        book.entry("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq -".to_string())
            .or_insert_with(Vec::new)
            .extend(vec![
                BookEntry { mossa: "e7e5".to_string(), peso: 100 },  // e5 (Apertura Italiana/Spagnola)
                BookEntry { mossa: "c7c5".to_string(), peso: 90 },   // c5 (Siciliana)
                BookEntry { mossa: "e7e6".to_string(), peso: 80 },   // e6 (Francese)
                BookEntry { mossa: "c7c6".to_string(), peso: 75 },   // c6 (Caro-Kann)
                BookEntry { mossa: "g8f6".to_string(), peso: 70 },   // Nf6 (Alekhine/Petrov)
            ]);
        
        // === DOPO 1.d4 (Nero muove) ===
        book.entry("rnbqkbnr/pppppppp/8/8/3P4/8/PPP1PPPP/RNBQKBNR b KQkq -".to_string())
            .or_insert_with(Vec::new)
            .extend(vec![
                BookEntry { mossa: "d7d5".to_string(), peso: 100 },  // d5 (Gambetto di Donna)
                BookEntry { mossa: "g8f6".to_string(), peso: 90 },   // Nf6 (Difesa Indiana)
                BookEntry { mossa: "e7e6".to_string(), peso: 80 },   // e6 (Catalana)
                BookEntry { mossa: "c7c5".to_string(), peso: 75 },   // c5 (Benoni)
            ]);
        
        // === DOPO 1.c4 (Nero muove) ===
        book.entry("rnbqkbnr/pppppppp/8/8/2P5/8/PP1PPPPP/RNBQKBNR b KQkq -".to_string())
            .or_insert_with(Vec::new)
            .extend(vec![
                BookEntry { mossa: "e7e5".to_string(), peso: 85 },   // e5
                BookEntry { mossa: "c7c5".to_string(), peso: 80 },   // c5
                BookEntry { mossa: "g8f6".to_string(), peso: 75 },   // Nf6
            ]);
        
        // === APERTURA ITALIANA ===
        // Dopo 1.e4 e5
        book.entry("rnbqkbnr/pppp1ppp/8/4p3/4P3/8/PPPP1PPP/RNBQKBNR w KQkq -".to_string())
            .or_insert_with(Vec::new)
            .push(BookEntry { mossa: "g1f3".to_string(), peso: 100 }); // Nf3
        
        // Dopo 1.e4 e5 2.Nf3
        book.entry("rnbqkbnr/pppp1ppp/8/4p3/4P3/5N2/PPPP1PPP/RNBQKBNR b KQkq -".to_string())
            .or_insert_with(Vec::new)
            .push(BookEntry { mossa: "b8c6".to_string(), peso: 100 }); // Nc6
        
        // Dopo 1.e4 e5 2.Nf3 Nc6
        book.entry("r1bqkbnr/pppp1ppp/2n5/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq -".to_string())
            .or_insert_with(Vec::new)
            .extend(vec![
                BookEntry { mossa: "f1c4".to_string(), peso: 80 },  // Bc4 (Italiana)
                BookEntry { mossa: "f1b5".to_string(), peso: 70 },  // Bb5 (Spagnola)
            ]);
        
        // === SICILIANA ===
        // Dopo 1.e4 c5 2.Nf3
        book.entry("rnbqkbnr/pp1ppppp/8/2p5/4P3/5N2/PPPP1PPP/RNBQKB1R b KQkq -".to_string())
            .or_insert_with(Vec::new)
            .push(BookEntry { mossa: "d7d6".to_string(), peso: 85 }); // d6
        
        // === FRANCESE ===
        // Dopo 1.e4 e6 2.d4
        book.entry("rnbqkbnr/pppp1ppp/4p3/8/3PP3/8/PPP2PPP/RNBQKBNR b KQkq -".to_string())
            .or_insert_with(Vec::new)
            .push(BookEntry { mossa: "d7d5".to_string(), peso: 100 }); // d5
        
        // === SVILUPPO ALFIERI E ARROCCO ===
        // Posizioni tipiche per sviluppo
        let posizioni_sviluppo = vec![
            // Bianco sviluppa alfiere
            ("r1bqkbnr/pppp1ppp/2n5/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq -", vec![
                BookEntry { mossa: "f1c4".to_string(), peso: 90 },
                BookEntry { mossa: "f1b5".to_string(), peso: 80 },
            ]),
            // Nero sviluppa alfiere
            ("r1bqkbnr/pppp1ppp/2n5/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R b KQkq -", vec![
                BookEntry { mossa: "f8c5".to_string(), peso: 90 },
                BookEntry { mossa: "f8b4".to_string(), peso: 80 },
            ]),
            // Arrocco bianco
            ("r1bqkbnr/pppp1ppp/2n5/4p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq -", vec![
                BookEntry { mossa: "e1g1".to_string(), peso: 100 },
            ]),
            // Arrocco nero
            ("r1bqk1nr/pppp1ppp/2n5/2b1p3/2B1P3/5N2/PPPP1PPP/RNBQK2R b KQkq -", vec![
                BookEntry { mossa: "e8g8".to_string(), peso: 100 },
            ]),
        ];
        
        for (fen, mosse) in posizioni_sviluppo {
            book.entry(fen.to_string())
                .or_insert_with(Vec::new)
                .extend(mosse);
        }
    }
    
    pub fn cerca_mossa(&self, scacchiera: &Scacchiera) -> Option<Mossa> {
        let fen = Self::fen_semplificato(scacchiera);
        
        // Debug: per vedere se il matching funziona
        // println!("DEBUG - Cercando FEN: {}", fen);
        // for (i, key) in self.book.keys().take(3).enumerate() {
        //     println!("DEBUG - Chiave {} nel libro: {}", i, key);
        // }
        
        if let Some(entries) = self.book.get(&fen) {
            if entries.is_empty() {
                return None;
            }
            
            let peso_totale: u32 = entries.iter().map(|e| e.peso).sum();
            if peso_totale == 0 {
                return None;
            }
            
            let mut rng = rand::thread_rng();
            let mut random_val = rng.gen_range(0..peso_totale);
            
            for entry in entries {
                if random_val < entry.peso {
                    // println!("DEBUG - Mossa dal libro: {}", entry.mossa);
                    return Self::mossa_da_string(&entry.mossa, scacchiera);
                }
                random_val -= entry.peso;
            }
        }
        
        None
    }
    
    fn fen_semplificato(scacchiera: &Scacchiera) -> String {
        // Usa il metodo a_fen() della scacchiera e prendi le prime 4 parti
        let fen_completo = scacchiera.a_fen();
        let parts: Vec<&str> = fen_completo.split_whitespace().collect();
        
        if parts.len() >= 4 {
            format!("{} {} {} {}", parts[0], parts[1], parts[2], parts[3])
        } else {
            fen_completo
        }
    }
    
    fn mossa_da_string(mossa_str: &str, _scacchiera: &Scacchiera) -> Option<Mossa> {
        if mossa_str.len() < 4 {
            return None;
        }
        
        let chars: Vec<char> = mossa_str.chars().collect();
        
        let from_file = chars[0] as u8 - b'a';
        let from_rank = chars[1] as u8 - b'1';
        let to_file = chars[2] as u8 - b'a';
        let to_rank = chars[3] as u8 - b'1';
        
        let from = Casella::nuova(from_file, from_rank)?;
        let to = Casella::nuova(to_file, to_rank)?;
        
        if mossa_str.len() > 4 {
            let pezzo_promosso = match chars[4] {
                'q' => Pezzo::Regina,
                'r' => Pezzo::Torre,
                'b' => Pezzo::Alfiere,
                'n' => Pezzo::Cavallo,
                _ => return None,
            };
            
            Some(Mossa::con_promozione(from, to, pezzo_promosso))
        } else {
            Some(Mossa::nuova(from, to))
        }
    }
}