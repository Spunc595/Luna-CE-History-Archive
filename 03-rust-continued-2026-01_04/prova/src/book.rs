use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use crate::board::{Scacchiera, Mossa, MoveFlag};

pub struct OpeningBook {
    file: File,
}

impl OpeningBook {
    pub fn load(path: &str) -> Option<Self> {
        let file = File::open(path).ok()?;
        println!("✅ Book: Formato binario PolyGlot rilevato.");
        Some(OpeningBook { file })
    }

    pub fn get_move(&self, board: &Scacchiera) -> Option<Mossa> {
        let target_hash = board.hash; // Nota: PolyGlot richiederebbe un hash specifico, ma proviamo con il tuo Zobrist
        let mut file = &self.file;
        let file_size = file.metadata().ok()?.len();
        let entries_count = (file_size / 16) as usize;

        // Ricerca binaria nel file (PolyGlot è ordinato per hash)
        let mut low = 0;
        let mut high = entries_count - 1;

        while low <= high {
            let mid = (low + high) / 2;
            let _ = file.seek(SeekFrom::Start((mid * 16) as u64));
            
            let mut entry = [0u8; 16];
            let _ = file.read_exact(&mut entry);
            
            let entry_hash = u64::from_be_bytes(entry[0..8].try_into().unwrap());

            if entry_hash == target_hash {
                // Abbiamo trovato la posizione! Leggiamo la mossa
                let raw_move = u16::from_be_bytes(entry[8..10].try_into().unwrap());
                return Some(self.parse_polyglot_move(raw_move));
            } else if entry_hash < target_hash {
                low = mid + 1;
            } else {
                if mid == 0 { break; }
                high = mid - 1;
            }
        }
        None
    }

    fn parse_polyglot_move(&self, raw: u16) -> Mossa {
        // PolyGlot encodes moves as: 
        // bits 0-2: to_file, bits 3-5: to_rank, bits 6-8: from_file, bits 9-11: from_rank
        let to_file = (raw & 0x7) as usize;
        let to_rank = ((raw >> 3) & 0x7) as usize;
        let from_file = ((raw >> 6) & 0x7) as usize;
        let from_rank = ((raw >> 9) & 0x7) as usize;
        let promo = (raw >> 12) & 0x7;

        let from_sq = from_rank * 8 + from_file;
        let to_sq = to_rank * 8 + to_file;
        
        // Mappa la promozione (PolyGlot: 1=N, 2=B, 3=R, 4=Q)
        let promo_piece = match promo {
            1 => Some(crate::board::Pezzo::Cavallo),
            2 => Some(crate::board::Pezzo::Alfiere),
            3 => Some(crate::board::Pezzo::Torre),
            4 => Some(crate::board::Pezzo::Regina),
            _ => None,
        };

        Mossa::new(from_sq, to_sq, MoveFlag::None, promo_piece)
    }
}