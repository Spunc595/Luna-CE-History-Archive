use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use rand::Rng;
use crate::board::{Scacchiera, Mossa, Colore, Pezzo, Casella, MoveFlag};
use crate::polyglot_keys; // Assicurati di aver creato il file polyglot_keys.rs

struct PolyGlotEntry {
    key: u64,
    move_bits: u16,
    weight: u16,
    learn: u32,
}

pub struct OpeningBook {
    file: Option<File>,
    // Chiavi Zobrist specifiche per PolyGlot
    piece_keys: [[u64; 64]; 12], // [PezzoType][Casella]
    castle_keys: [u64; 16],
    en_passant_keys: [u64; 65],
    turn_key: u64,
}

impl OpeningBook {
    pub fn new() -> Self {
        let mut book = OpeningBook {
            file: None,
            piece_keys: [[0; 64]; 12],
            castle_keys: [0; 16],
            en_passant_keys: [0; 65],
            turn_key: 0,
        };
        book.init_polyglot_keys();
        book
    }

    /// Carica il file .bin dal percorso specificato
    pub fn load(&mut self, path: &str) {
        match File::open(Path::new(path)) {
            Ok(f) => {
                self.file = Some(f);
                println!("INFO: Libro aperture caricato correttamente: {}", path);
            },
            Err(_) => {
                println!("WARNING: Impossibile trovare il libro aperture: {}", path);
                self.file = None;
            }
        }
    }

    /// Cerca una mossa nel libro per la posizione corrente
    pub fn get_book_move(&mut self, board: &Scacchiera) -> Option<Mossa> {
        if self.file.is_none() { return None; }

        let hash = self.compute_polyglot_hash(board);
        let entries = self.find_entries(hash);

        if entries.is_empty() { return None; }

        // Selezione Ponderata (Weighted Random Selection)
        // Più alto è il 'weight', più probabile è che la mossa venga scelta.
        let total_weight: u32 = entries.iter().map(|e| e.weight as u32).sum();
        
        if total_weight == 0 { 
            // Se tutte le mosse hanno peso 0, prendiamo la prima se esiste
            return entries.first().and_then(|e| self.polyglot_to_move(e.move_bits, board));
        }

        let mut rng = rand::thread_rng();
        let mut choice = rng.gen_range(0..total_weight);

        for entry in entries {
            if choice < entry.weight as u32 {
                return self.polyglot_to_move(entry.move_bits, board);
            }
            choice -= entry.weight as u32;
        }

        None
    }

    // --- INTERNI: Calcolo Hash e Ricerca ---

    fn init_polyglot_keys(&mut self) {
        polyglot_keys::initialize_keys(
            &mut self.piece_keys,
            &mut self.castle_keys,
            &mut self.en_passant_keys,
            &mut self.turn_key
        );
    }

    fn compute_polyglot_hash(&self, board: &Scacchiera) -> u64 {
        let mut hash = 0;

        // 1. Pezzi
        // PolyGlot mapping: Bianco (0-5), Nero (6-11)
        for sq in 0..64 {
            if let Some((pezzo, colore)) = board.pezzo_su_casella(Casella(sq)) {
                let offset = if colore == Colore::Bianco { 0 } else { 6 };
                let p_idx = pezzo.indice() + offset;
                hash ^= self.piece_keys[p_idx][sq];
            }
        }

        // 2. Arrocco
        // Assumiamo che diritti_arrocco sia bitmask: K=1, Q=2, k=4, q=8 (0-15)
        let castle_rights = board.diritti_arrocco as usize; 
        hash ^= self.castle_keys[castle_rights];

        // 3. En Passant
        // Hashiamo la colonna solo se c'è un pedone che può catturare? 
        // Lo standard PolyGlot hasha la colonna se EP è disponibile.
        if let Some(ep_sq) = board.en_passant {
            let file = ep_sq % 8;
            hash ^= self.en_passant_keys[file];
        }

        // 4. Turno
        if board.turno == Colore::Bianco {
            hash ^= self.turn_key;
        }

        hash
    }

    fn find_entries(&mut self, key: u64) -> Vec<PolyGlotEntry> {
        let mut entries = Vec::new();
        let file = self.file.as_mut().unwrap();

        // I file .bin sono formati da entry di 16 byte
        let len = file.metadata().unwrap().len();
        let entry_size = 16;
        let num_entries = len / entry_size;

        // Binary Search
        let mut left = 0;
        let mut right = num_entries - 1;

        while left <= right {
            let mid = (left + right) / 2;
            file.seek(SeekFrom::Start(mid * entry_size)).unwrap();
            
            let entry = read_entry(file);
            
            if entry.key < key {
                left = mid + 1;
            } else if entry.key > key {
                if mid == 0 { break; }
                right = mid - 1;
            } else {
                // Trovato una entry corrispondente.
                // Poiché possono esserci più mosse per la stessa posizione,
                // dobbiamo trovare la prima e poi raccoglierle tutte.
                
                // Vai indietro fino alla prima
                let mut first = mid;
                while first > 0 {
                    file.seek(SeekFrom::Start((first - 1) * entry_size)).unwrap();
                    let prev = read_entry(file);
                    if prev.key != key { break; }
                    first -= 1;
                }

                // Raccogli tutte le entry in avanti
                let mut curr = first;
                loop {
                    file.seek(SeekFrom::Start(curr * entry_size)).unwrap();
                    if curr >= num_entries { break; }
                    
                    let e = read_entry(file);
                    if e.key != key { break; }
                    
                    entries.push(e);
                    curr += 1;
                }
                break;
            }
        }
        entries
    }

    /// Decodifica la mossa dal formato PolyGlot (u16) al formato Engine (Mossa)
    fn polyglot_to_move(&self, bits: u16, board: &Scacchiera) -> Option<Mossa> {
        // Encoding: to(0-5), from(6-11), promo(12-14)
        let to_sq = (bits & 0x3F) as usize;
        let from_sq = ((bits >> 6) & 0x3F) as usize;
        let promo_idx = (bits >> 12) & 0x7;

        let promo_piece = match promo_idx {
            1 => Some(Pezzo::Cavallo),
            2 => Some(Pezzo::Alfiere),
            3 => Some(Pezzo::Torre),
            4 => Some(Pezzo::Regina),
            _ => None,
        };

        // Dobbiamo dedurre il MoveFlag guardando la scacchiera
        let mut flag = MoveFlag::None;
        
        let pezzo_mosso = match board.pezzo_su_casella(Casella(from_sq)) {
            Some((p, _)) => p,
            None => return None, // Errore: casella vuota?
        };
        let cattura = board.pezzo_su_casella(Casella(to_sq));

        // 1. Arrocco (Re si muove di 2 passi)
        if pezzo_mosso == Pezzo::Re && (from_sq as i32 - to_sq as i32).abs() == 2 {
            flag = if to_sq > from_sq { MoveFlag::CastleKingSide } else { MoveFlag::CastleQueenSide };
        }
        // 2. Pedoni
        else if pezzo_mosso == Pezzo::Pedone {
            // Spinta doppia
            if (from_sq as i32 - to_sq as i32).abs() == 16 {
                flag = MoveFlag::DoublePawnPush;
            } 
            // En Passant (movimento diagonale su casella vuota)
            else if cattura.is_none() && (from_sq % 8 != to_sq % 8) {
                flag = MoveFlag::EnPassant;
            }
            // Promozione
            else if promo_piece.is_some() {
                flag = if cattura.is_some() { MoveFlag::PromotionCapture } else { MoveFlag::Promotion };
            }
            // Cattura Normale Pedone
            else if cattura.is_some() {
                flag = MoveFlag::Capture;
            }
        }
        // 3. Cattura Normale (Altri pezzi)
        else if cattura.is_some() {
            flag = MoveFlag::Capture;
        }

        Some(Mossa::new_with_flag(Casella(from_sq), Casella(to_sq), promo_piece, flag))
    }
}

// Helper per leggere 16 byte e convertirli in struct
fn read_entry(file: &mut File) -> PolyGlotEntry {
    let mut buf = [0u8; 16];
    // Se la read fallisce, torniamo tutto 0 (non crasherà, ma non troverà nulla)
    if file.read_exact(&mut buf).is_err() {
        return PolyGlotEntry { key: 0, move_bits: 0, weight: 0, learn: 0 };
    }
    PolyGlotEntry {
        key: u64::from_be_bytes(buf[0..8].try_into().unwrap()),
        move_bits: u16::from_be_bytes(buf[8..10].try_into().unwrap()),
        weight: u16::from_be_bytes(buf[10..12].try_into().unwrap()),
        learn: u32::from_be_bytes(buf[12..16].try_into().unwrap()),
    }
}