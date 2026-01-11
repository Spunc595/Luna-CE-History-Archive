pub struct ZobristKeys {
    pub pezzi: [[[u64; 64]; 6]; 2],
    pub turno: u64,
    pub arrocco: [u64; 16],
    pub ep_file: [u64; 8], // Per le catture En Passant
}

impl ZobristKeys {
    pub fn init() -> Self {
        // Generatore di numeri casuali Xorshift per determinismo
        let mut seed = 123456789u64;
        let mut next_u64 = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };

        let mut pezzi = [[[0u64; 64]; 6]; 2];
        for c in 0..2 {
            for p in 0..6 {
                for sq in 0..64 {
                    pezzi[c][p][sq] = next_u64();
                }
            }
        }

        let mut arrocco = [0u64; 16];
        for i in 0..16 {
            arrocco[i] = next_u64();
        }

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
}