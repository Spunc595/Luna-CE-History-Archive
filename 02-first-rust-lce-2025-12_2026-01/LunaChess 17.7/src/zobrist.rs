use crate::board::{Scacchiera, Colore};
use rand::Rng;

pub struct ZobristKeys {
    pub pezzi: [[[u64; 64]; 6]; 2],
    pub turno: u64,
    pub arrocco: [u64; 16], // 16 combinazioni possibili di diritti (4 bit)
}

impl ZobristKeys {
    pub fn new() -> Self {
        let mut rng = rand::thread_rng();
        let mut pezzi = [[[0u64; 64]; 6]; 2];
        let mut arrocco = [0u64; 16];

        for c in 0..2 {
            for p in 0..6 {
                for s in 0..64 { pezzi[c][p][s] = rng.gen(); }
            }
        }
        for i in 0..16 { arrocco[i] = rng.gen(); }

        Self { pezzi, turno: rng.gen(), arrocco }
    }

    pub fn hash(&self, board: &Scacchiera) -> u64 {
        let mut h = 0u64;

        for colore in 0..2 {
            for pezzo in 0..6 {
                let mut bb = board.pezzi[pezzo] & board.colori[colore];
                while bb != 0 {
                    let sq = bb.trailing_zeros() as usize;
                    h ^= self.pezzi[colore][pezzo][sq];
                    bb &= bb - 1;
                }
            }
        }

        if board.turno == Colore::Nero { h ^= self.turno; }
        
        // Includiamo i diritti di arrocco nell'hash!
        h ^= self.arrocco[board.diritti_arrocco as usize];

        h
    }
}