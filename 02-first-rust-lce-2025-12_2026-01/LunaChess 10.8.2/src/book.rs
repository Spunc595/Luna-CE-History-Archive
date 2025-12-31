use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use rand::Rng;
use crate::board::{Scacchiera, Mossa, Colore, Pezzo, Casella, MoveFlag};
use crate::polyglot_keys; // Ignorato se non usato, va bene

struct PolyGlotEntry {
    key: u64,
    move_bits: u16,
    weight: u16,
    learn: u32,
}

pub struct OpeningBook {
    file: Option<File>,
}

impl OpeningBook {
    pub fn new() -> Self {
        OpeningBook { file: None }
    }

    pub fn load(&mut self, path: &str) {
        // DEBUG: Stampa dove cerca il file
        if let Ok(curr) = std::env::current_dir() {
            println!("BOOK: Searching in directory: {:?}", curr);
        }
        
        match File::open(Path::new(path)) {
            Ok(f) => {
                self.file = Some(f);
                println!("BOOK: SUCCESS! Loaded '{}'", path);
            },
            Err(e) => {
                println!("BOOK: FAILED to load '{}'. Error: {}", path, e);
                self.file = None;
            }
        }
    }

    pub fn get_book_move(&mut self, board: &Scacchiera) -> Option<Mossa> {
        if self.file.is_none() { return None; }

        // Calcoliamo l'hash PolyGlot "on the fly" usando le chiavi del board
        // MA applicando la regola EP restrittiva
        let hash = self.compute_strict_hash(board);
        
        let entries = self.find_entries(hash);
        if entries.is_empty() { return None; }

        let total_weight: u32 = entries.iter().map(|e| e.weight as u32).sum();
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

    // Calcolo Hash Manuale con EP Check
    fn compute_strict_hash(&self, board: &Scacchiera) -> u64 {
        let mut hash = 0;

        // 1. Pezzi
        for sq in 0..64 {
            if let Some((p, c)) = board.pezzo_su_casella(Casella(sq)) {
                hash ^= board.get_zobrist_piece(c, p, sq);
            }
        }

        // 2. Arrocco
        hash ^= board.get_zobrist_castle(board.diritti_arrocco);

        // 3. En Passant (SOLO SE CATTURABILE)
        if let Some(ep_sq) = board.en_passant {
            let ep_file = ep_sq % 8;
            let us = board.turno;
            let enemy = us.opposto();
            
            // Verifica se un pedone nemico può mangiare
            let potential_attackers = crate::attacks::pawn_attacks(ep_sq, enemy); 
            let my_pawns = board.bitboard_pezzo_colore(Pezzo::Pedone, us);
            
            if (potential_attackers & my_pawns) != 0 {
                hash ^= board.get_zobrist_ep(ep_file);
            }
        }

        // 4. Turno
        if board.turno == Colore::Bianco {
            hash ^= board.get_zobrist_turn(); // Nota: Polyglot non hasha il turno se è bianco?
            // VERIFICA: Polyglot hasha il turno casuale se è il bianco a muovere?
            // NO. Polyglot standard: "The key for the side to move is XOR-ed in if it is Black."
            // Il mio Zobrist in Board fa: if Nero -> XOR.
            // Quindi qui non devo fare nulla, l'hash base è zero. 
            // Aspetta, board.get_zobrist_turn() ritorna il random number.
            // Se board.rs fa "if Nero ^= turn", allora qui devo fare lo stesso.
        }
        
        // Aspetta, rifacciamo il sync col board.
        // Board: if Nero { h ^= ZOBRIST.turno }
        // PolyGlot Spec: "The key for the side to move is XOR-ed in if it is Black."
        // Quindi se è Nero, XOR. Se è Bianco, niente.
        if board.turno == Colore::Nero {
            hash ^= board.get_zobrist_turn();
        }

        hash
    }

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
            if file.seek(SeekFrom::Start(mid * entry_size)).is_err() { break; }
            let entry = read_entry(file);
            
            if entry.key < key {
                left = mid + 1;
            } else if entry.key > key {
                if mid == 0 { break; }
                right = mid - 1;
            } else {
                let mut first = mid;
                while first > 0 {
                    file.seek(SeekFrom::Start((first - 1) * entry_size)).unwrap();
                    let prev = read_entry(file);
                    if prev.key != key { break; }
                    first -= 1;
                }
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

    fn polyglot_to_move(&self, bits: u16, board: &Scacchiera) -> Option<Mossa> {
        let to_sq = (bits & 0x3F) as usize;
        let from_sq = ((bits >> 6) & 0x3F) as usize;
        let promo_idx = (bits >> 12) & 0x7;
        let promo_piece = match promo_idx { 1=>Some(Pezzo::Cavallo), 2=>Some(Pezzo::Alfiere), 3=>Some(Pezzo::Torre), 4=>Some(Pezzo::Regina), _=>None };

        let mut flag = MoveFlag::None;
        let pezzo_mosso = match board.pezzo_su_casella(Casella(from_sq)) { Some((p, _)) => p, None => return None };
        let cattura = board.pezzo_su_casella(Casella(to_sq));

        if pezzo_mosso == Pezzo::Re && (from_sq as i32 - to_sq as i32).abs() == 2 {
            flag = if to_sq > from_sq { MoveFlag::CastleKingSide } else { MoveFlag::CastleQueenSide };
        } else if pezzo_mosso == Pezzo::Pedone {
            if (from_sq as i32 - to_sq as i32).abs() == 16 { flag = MoveFlag::DoublePawnPush; }
            else if cattura.is_none() && (from_sq % 8 != to_sq % 8) { flag = MoveFlag::EnPassant; }
            else if promo_piece.is_some() { flag = if cattura.is_some() { MoveFlag::PromotionCapture } else { MoveFlag::Promotion }; }
            else if cattura.is_some() { flag = MoveFlag::Capture; }
        } else if cattura.is_some() { flag = MoveFlag::Capture; }

        Some(Mossa::new_with_flag(Casella(from_sq), Casella(to_sq), promo_piece, flag))
    }
}

fn read_entry(file: &mut File) -> PolyGlotEntry {
    let mut buf = [0u8; 16];
    if file.read_exact(&mut buf).is_err() { return PolyGlotEntry { key: 0, move_bits: 0, weight: 0, learn: 0 }; }
    PolyGlotEntry {
        key: u64::from_be_bytes(buf[0..8].try_into().unwrap()),
        move_bits: u16::from_be_bytes(buf[8..10].try_into().unwrap()),
        weight: u16::from_be_bytes(buf[10..12].try_into().unwrap()),
        learn: u32::from_be_bytes(buf[12..16].try_into().unwrap()),
    }
}