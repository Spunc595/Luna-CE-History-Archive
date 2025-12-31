use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use rand::Rng;
use crate::board::{Scacchiera, Mossa, Colore, Pezzo, Casella, MoveFlag};
use crate::polyglot_keys; // Assicurati di avere polyglot_keys.rs compilato

// Struttura di una riga nel file .bin (16 bytes)
struct PolyGlotEntry {
    key: u64,       // Hash della posizione (8 bytes)
    move_bits: u16, // Mossa codificata (2 bytes)
    weight: u16,    // Peso della mossa (2 bytes)
    learn: u32,     // Info apprendimento (4 bytes - non usato qui)
}

pub struct OpeningBook {
    file: Option<File>,
    // Chiavi Zobrist per calcolare l'hash PolyGlot
    piece_keys: [[u64; 64]; 12], 
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
        // Inizializza le chiavi usando il modulo polyglot_keys
        book.init_polyglot_keys();
        book
    }

    /// Carica il file del libro (es. "book.bin")
    pub fn load(&mut self, path: &str) {
        match File::open(Path::new(path)) {
            Ok(f) => {
                self.file = Some(f);
                println!("LIBRO: Caricato correttamente '{}'", path);
            },
            Err(_) => {
                println!("LIBRO: File '{}' non trovato. Il motore userà solo il calcolo.", path);
                self.file = None;
            }
        }
    }

    /// Restituisce una mossa dal libro pesata in base alla probabilità
    pub fn get_book_move(&mut self, board: &Scacchiera) -> Option<Mossa> {
        if self.file.is_none() { return None; }

        let hash = self.compute_polyglot_hash(board);
        let entries = self.find_entries(hash);

        if entries.is_empty() { return None; }

        // Somma i pesi per la scelta casuale ponderata
        let total_weight: u32 = entries.iter().map(|e| e.weight as u32).sum();
        
        // Se il peso totale è 0 (raro), prendiamo la prima mossa
        if total_weight == 0 {
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

    // --- METODI INTERNI ---

    fn init_polyglot_keys(&mut self) {
        polyglot_keys::initialize_keys(
            &mut self.piece_keys,
            &mut self.castle_keys,
            &mut self.en_passant_keys,
            &mut self.turn_key
        );
    }

    /// Calcola l'Hash Zobrist secondo lo standard rigoroso di PolyGlot
    fn compute_polyglot_hash(&self, board: &Scacchiera) -> u64 {
        let mut hash = 0;

        // 1. PEZZI
        // Mapping PolyGlot: Bianco=0..5, Nero=6..11
        // Ordine: Pedone(0), Cavallo(1), Alfiere(2), Torre(3), Regina(4), Re(5)
        for sq in 0..64 {
            if let Some((pezzo, colore)) = board.pezzo_su_casella(Casella(sq)) {
                let offset = if colore == Colore::Bianco { 0 } else { 6 };
                let p_idx = pezzo.indice() + offset;
                hash ^= self.piece_keys[p_idx][sq];
            }
        }

        // 2. ARROCCO
        // PolyGlot usa 4 bit: Bianco K(1), Q(2), Nero k(4), q(8) -> Totale 0-15
        // board.diritti_arrocco deve corrispondere a questa maschera
        let castle_rights = board.diritti_arrocco as usize; 
        hash ^= self.castle_keys[castle_rights];

        // 3. EN PASSANT (CRITICO)
        // Lo standard richiede di includere l'hash EP *SOLO SE* è possibile catturare legalmente.
        if let Some(ep_sq) = board.en_passant {
            let ep_file = ep_sq % 8;
            let us = board.turno;
            let enemy = us.opposto();

            // Calcoliamo se c'è un pedone nostro che può mangiare in ep_sq.
            // Usiamo il trucco degli attacchi inversi:
            // "Chi attacca ep_sq come se fosse un pedone nemico?" -> I nostri pedoni.
            
            // Nota: Importa pawn_attacks da attacks.rs
            let potential_attackers = crate::attacks::pawn_attacks(ep_sq, enemy); 
            let my_pawns = board.bitboard_pezzo_colore(Pezzo::Pedone, us);
            
            if (potential_attackers & my_pawns) != 0 {
                // Cattura possibile! Hashiamo la colonna.
                hash ^= self.en_passant_keys[ep_file];
            }
        }

        // 4. TURNO
        if board.turno == Colore::Bianco {
            hash ^= self.turn_key;
        }

        hash
    }

    /// Cerca le entry nel file usando Binary Search
    fn find_entries(&mut self, key: u64) -> Vec<PolyGlotEntry> {
        let mut entries = Vec::new();
        let file = self.file.as_mut().unwrap();

        let len = file.metadata().unwrap().len();
        let entry_size = 16;
        let num_entries = len / entry_size;

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
                // Trovato!
                // Potrebbero esserci più mosse per la stessa chiave (collisione o varianti)
                // Torniamo indietro per trovare la prima
                let mut first = mid;
                while first > 0 {
                    file.seek(SeekFrom::Start((first - 1) * entry_size)).unwrap();
                    let prev = read_entry(file);
                    if prev.key != key { break; }
                    first -= 1;
                }

                // Raccogliamo tutte le entry uguali in avanti
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

    /// Converte 16 bit PolyGlot in struct Mossa
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

        // Determiniamo il Flag
        let mut flag = MoveFlag::None;
        
        let pezzo_mosso = match board.pezzo_su_casella(Casella(from_sq)) {
            Some((p, _)) => p,
            None => return None, 
        };
        let cattura = board.pezzo_su_casella(Casella(to_sq));

        // 1. Arrocco
        if pezzo_mosso == Pezzo::Re && (from_sq as i32 - to_sq as i32).abs() == 2 {
            flag = if to_sq > from_sq { MoveFlag::CastleKingSide } else { MoveFlag::CastleQueenSide };
        }
        // 2. Pedone
        else if pezzo_mosso == Pezzo::Pedone {
            // Doppia Spinta
            if (from_sq as i32 - to_sq as i32).abs() == 16 {
                flag = MoveFlag::DoublePawnPush;
            }
            // En Passant
            else if cattura.is_none() && (from_sq % 8 != to_sq % 8) {
                flag = MoveFlag::EnPassant;
            }
            // Promozione
            else if promo_piece.is_some() {
                flag = if cattura.is_some() { MoveFlag::PromotionCapture } else { MoveFlag::Promotion };
            }
            // Cattura Semplice
            else if cattura.is_some() {
                flag = MoveFlag::Capture;
            }
        }
        // 3. Altri Pezzi
        else if cattura.is_some() {
            flag = MoveFlag::Capture;
        }

        Some(Mossa::new_with_flag(Casella(from_sq), Casella(to_sq), promo_piece, flag))
    }
}

// Helper per leggere i bytes Big Endian dal file
fn read_entry(file: &mut File) -> PolyGlotEntry {
    let mut buf = [0u8; 16];
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