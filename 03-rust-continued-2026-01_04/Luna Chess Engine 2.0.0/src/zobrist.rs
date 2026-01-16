pub struct ZobristKeys {
    pub pezzi: [[[u64; 64]; 6]; 2],
    pub turno: u64,
    pub arrocco: [u64; 16],
    pub ep_file: [u64; 8],
}

impl ZobristKeys {
    pub fn init() -> Self {
        // Generatore Xorshift64
        // Usiamo un seed costante così l'hash di una posizione è lo stesso 
        // ogni volta che riavviamo il motore (fondamentale per il dataset).
        let mut seed = 0x9E3779B97F4A7C15u64;
        let mut next_u64 = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };

        // "Riscaldamento": scartiamo i primi numeri per migliorare la casualità
        for _ in 0..16 { next_u64(); }

        // 1. Chiavi per Pezzi (Bianco/Nero, Tipo Pezzo, Casa)
        let mut pezzi = [[[0u64; 64]; 6]; 2];
        for c in 0..2 {
            for p in 0..6 {
                for sq in 0..64 {
                    pezzi[c][p][sq] = next_u64();
                }
            }
        }

        // 2. Chiavi per diritti di Arrocco
        let mut arrocco = [0u64; 16];
        for i in 0..16 {
            arrocco[i] = next_u64();
        }

        // 3. Chiavi per En Passant (basate sulla colonna/file)
        let mut ep_file = [0u64; 8];
        for i in 0..8 {
            ep_file[i] = next_u64();
        }

        ZobristKeys {
            pezzi,
            turno: next_u64(),
            arrocco,
            ep_file,
        }
    }

    /// Calcola l'hash completo (usato per inizializzare board.hash)
    pub fn hash_board(&self, board: &crate::board::Scacchiera) -> u64 {
        let mut h = 0u64;

        for p in 0..6 {
            // Bianchi
            let mut bb_w = board.pezzi[p] & board.colori[0];
            while bb_w != 0 {
                let sq = bb_w.trailing_zeros() as usize;
                h ^= self.pezzi[0][p][sq];
                bb_w &= bb_w - 1;
            }
            // Neri
            let mut bb_b = board.pezzi[p] & board.colori[1];
            while bb_b != 0 {
                let sq = bb_b.trailing_zeros() as usize;
                h ^= self.pezzi[1][p][sq];
                bb_b &= bb_b - 1;
            }
        }

        if board.turno == crate::board::Colore::Nero {
            h ^= self.turno;
        }

        h ^= self.arrocco[board.diritti_arrocco as usize];

        if let Some(sq) = board.ep_square {
            h ^= self.ep_file[sq % 8];
        }

        h
    }
}