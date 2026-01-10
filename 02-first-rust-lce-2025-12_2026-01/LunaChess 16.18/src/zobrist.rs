use crate::board::{Scacchiera, Colore};
use rand::Rng;
use rand_pcg::Pcg64;
use rand::SeedableRng;

pub struct ZobristKeys {
    pub pieces: [[[u64; 64]; 6]; 2],
    pub side: u64,
    pub castling: [u64; 16],
    pub ep: [u64; 64],
}

impl ZobristKeys {
    pub fn new() -> Self {
        let mut rng = Pcg64::seed_from_u64(123456789);
        let mut keys = Self {
            pieces: [[[0; 64]; 6]; 2],
            side: rng.gen(),
            castling: [0; 16],
            ep: [0; 64],
        };
        for c in 0..2 {
            for p in 0..6 {
                for sq in 0..64 { keys.pieces[c][p][sq] = rng.gen(); }
            }
        }
        for i in 0..16 { keys.castling[i] = rng.gen(); }
        for i in 0..64 { keys.ep[i] = rng.gen(); }
        keys
    }
}

pub fn compute_hash(board: &Scacchiera, keys: &ZobristKeys) -> u64 {
    let mut h = 0u64;
    for c in 0..2 {
        for p in 0..6 {
            let mut bb = board.pezzi[p] & board.colori[c];
            while bb != 0 {
                let sq = bb.trailing_zeros() as usize;
                h ^= keys.pieces[c][p][sq];
                bb &= bb - 1;
            }
        }
    }
    if board.turno == Colore::Nero { h ^= keys.side; }
    h ^= keys.castling[board.diritti_arrocco as usize];
    if let Some(sq) = board.ep_square { h ^= keys.ep[sq]; }
    h
}