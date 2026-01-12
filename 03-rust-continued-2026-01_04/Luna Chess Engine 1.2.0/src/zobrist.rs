pub struct ZobristKeys {
    pub pezzi: [[[u64; 64]; 6]; 2],
    pub turno: u64,
    pub arrocco: [u64; 16],
    pub ep_file: [u64; 8],
}

impl ZobristKeys {
    pub fn init() -> Self {
        // Generatore Xorshift64 con un seed più "distribuito"
        let mut seed = 0x9E3779B97F4A7C15u64;
        let mut next_u64 = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };

        // 1. Chiavi per ogni Pezzo/Colore/Casa
        let mut pezzi = [[[0u64; 64]; 6]; 2];
        for c in 0..2 {
            for p in 0..6 {
                for sq in 0..64 {
                    pezzi[c][p][sq] = next_u64();
                }
            }
        }

        // 2. Chiavi per i diritti di Arrocco (16 combinazioni: 0000 a 1111)
        let mut arrocco = [0u64; 16];
        for i in 0..16 {
            arrocco[i] = next_u64();
        }

        // 3. Chiavi per la colonna (file) dell'En Passant
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

    /// Calcola l'hash completo di una scacchiera da zero.
    /// Viene usato all'inizio della partita o per verifiche di integrità.
    pub fn hash_board(&self, board: &crate::board::Scacchiera) -> u64 {
        let mut h = 0u64;

        // Pezzi
        for p in 0..6 {
            let mut bb_bianchi = board.pezzi[p] & board.colori[0];
            while bb_bianchi != 0 {
                let sq = bb_bianchi.trailing_zeros() as usize;
                h ^= self.pezzi[0][p][sq];
                bb_bianchi &= bb_bianchi - 1;
            }

            let mut bb_neri = board.pezzi[p] & board.colori[1];
            while bb_neri != 0 {
                let sq = bb_neri.trailing_zeros() as usize;
                h ^= self.pezzi[1][p][sq];
                bb_neri &= bb_neri - 1;
            }
        }

        // Turno
        if board.turno == crate::board::Colore::Nero {
            h ^= self.turno;
        }

        // Arrocco
        h ^= self.arrocco[board.diritti_arrocco as usize];

        // En Passant (solo se è effettivamente possibile una cattura)
        if let Some(sq) = board.ep_square {
            h ^= self.ep_file[sq % 8];
        }

        h
    }
}