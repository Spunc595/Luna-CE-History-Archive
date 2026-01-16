use crate::board::{Scacchiera, Colore};
use std::fs::File;
use std::io::Read;

pub const INPUT_SIZE: usize = 768;
pub const HIDDEN_SIZE: usize = 256;

pub struct LunaNNUE {
    pub feature_weights: Vec<i16>,
    pub feature_bias: Vec<i16>,
    pub output_weights: Vec<i16>,
    pub output_bias: i16,
}

impl LunaNNUE {
    pub fn load(path: &str) -> Option<Self> {
        let mut file = File::open(path).ok()?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).ok()?;

        let expected_size = (INPUT_SIZE * HIDDEN_SIZE * 2) + (HIDDEN_SIZE * 2) + (HIDDEN_SIZE * 2) + 2;
        if buffer.len() < expected_size { return None; }

        let mut offset = 0;
        let mut read_i16_vec = |size: usize| {
            let mut v = Vec::with_capacity(size);
            for _ in 0..size {
                v.push(i16::from_le_bytes([buffer[offset], buffer[offset + 1]]));
                offset += 2;
            }
            v
        };

        Some(Self {
            feature_weights: read_i16_vec(INPUT_SIZE * HIDDEN_SIZE),
            feature_bias: read_i16_vec(HIDDEN_SIZE),
            output_weights: read_i16_vec(HIDDEN_SIZE),
            output_bias: i16::from_le_bytes([buffer[offset], buffer[offset + 1]]),
        })
    }

    #[inline(always)]
    pub fn evaluate(&self, board: &Scacchiera) -> i32 {
        let mut acc = [0i32; HIDDEN_SIZE];

        // 1. Inizializzazione con bias
        for i in 0..HIDDEN_SIZE {
            acc[i] = self.feature_bias[i] as i32;
        }

        // 2. Accumulazione pesi (Trasformazione lineare)
        for color in 0..2 {
            let color_offset = color * 384;
            for piece_type in 0..6 {
                let mut bb = board.pezzi[piece_type] & board.colori[color];
                let piece_offset = piece_type * 64;
                
                while bb != 0 {
                    let sq = bb.trailing_zeros() as usize;
                    let feature_idx = color_offset + piece_offset + sq;
                    let weight_start = feature_idx * HIDDEN_SIZE;
                    let weights = &self.feature_weights[weight_start..weight_start + HIDDEN_SIZE];

                    // Questo loop viene ottimizzato in SIMD dal compilatore
                    for i in 0..HIDDEN_SIZE {
                        acc[i] += weights[i] as i32;
                    }
                    bb &= bb - 1;
                }
            }
        }

        // 3. Output layer con Clipped ReLU
        let mut output = 0i32;
        for i in 0..HIDDEN_SIZE {
            output += acc[i].clamp(0, 255) * (self.output_weights[i] as i32);
        }

        let final_score = (output / 64) + self.output_bias as i32;
        if board.turno == Colore::Nero { -final_score } else { final_score }
    }
}