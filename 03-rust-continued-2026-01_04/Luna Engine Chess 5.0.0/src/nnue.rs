use std::fs::File;
use std::io::Read;
use crate::board::{Scacchiera, Colore, Pezzo};

pub const INPUT_SIZE: usize = 768; 
pub const L1_SIZE: usize = 256;
pub const L2_SIZE: usize = 32;

const QA: i32 = 255; 
const QB: i32 = 64;

#[derive(Clone, Debug)]
pub struct Accumulator {
    pub vals: [i32; L1_SIZE],
}

impl Accumulator {
    pub fn new() -> Self {
        Self { vals: [0; L1_SIZE] }
    }
}

pub struct LunaNNUE {
    l1_weights: Vec<i16>,
    l1_bias: Vec<i16>,
    l2_weights: Vec<i16>,
    l2_bias: Vec<i16>,
    l3_weights: Vec<i16>,
    l3_bias: Vec<i16>,
}

impl LunaNNUE {
    pub fn load(path: &str) -> Option<Self> {
        let mut file = File::open(path).ok()?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).ok()?;

        let mut offset = 0;
        let mut read_i16 = |count: usize| -> Vec<i16> {
            let mut v = Vec::with_capacity(count);
            for _ in 0..count {
                if offset + 2 <= buffer.len() {
                    v.push(i16::from_le_bytes([buffer[offset], buffer[offset + 1]]));
                    offset += 2;
                } else { v.push(0); }
            }
            v
        };

        Some(LunaNNUE {
            l1_weights: read_i16(L1_SIZE * INPUT_SIZE),
            l1_bias: read_i16(L1_SIZE),
            l2_weights: read_i16(L2_SIZE * L1_SIZE),
            l2_bias: read_i16(L2_SIZE),
            l3_weights: read_i16(L2_SIZE),
            l3_bias: read_i16(1),
        })
    }

    #[inline(always)]
    pub fn get_feature_index(colore: Colore, pezzo: Pezzo, sq: usize) -> usize {
        let p_type = match pezzo {
            Pezzo::Pedone => 0, 
            Pezzo::Cavallo => 1, 
            Pezzo::Alfiere => 2,
            Pezzo::Torre => 3, 
            Pezzo::Regina => 4, 
            Pezzo::Re => 5,
        };
        
        let p_idx = if colore == Colore::Bianco { p_type } else { p_type + 6 };
        p_idx * 64 + sq
    }

    pub fn update_add(&self, acc: &mut Accumulator, feature_idx: usize) {
        for i in 0..L1_SIZE {
            acc.vals[i] += self.l1_weights[i * INPUT_SIZE + feature_idx] as i32;
        }
    }

    pub fn update_remove(&self, acc: &mut Accumulator, feature_idx: usize) {
        for i in 0..L1_SIZE {
            acc.vals[i] -= self.l1_weights[i * INPUT_SIZE + feature_idx] as i32;
        }
    }

    pub fn reset_accumulator(&self, acc: &mut Accumulator) {
        for i in 0..L1_SIZE {
            acc.vals[i] = self.l1_bias[i] as i32; 
        }
    }

    pub fn evaluate(&self, acc: &Accumulator, turno: Colore) -> i32 {
        let mut l2_acc = [0i32; L2_SIZE];
        
        // Livello 1 -> Livello 2 (SCReLU clamp 0..255)
        for i in 0..L2_SIZE {
            let mut sum: i32 = (self.l2_bias[i] as i32) * QA;
            let offset = i * L1_SIZE;
            for j in 0..L1_SIZE {
                let activation = acc.vals[j].clamp(0, QA);
                if activation > 0 {
                    sum += activation * (self.l2_weights[offset + j] as i32);
                }
            }
            l2_acc[i] = sum / QA;
        }

        // Livello 2 -> Livello 3 (ReLU)
        let mut score: i32 = (self.l3_bias[0] as i32) * QB;
        for i in 0..L2_SIZE {
            let activation = l2_acc[i].max(0); 
            if activation > 0 {
                score += activation * (self.l3_weights[i] as i32);
            }
        }

        // Scalatura finale in centipawn
        let final_cp = score / 600; 

        if turno == Colore::Bianco { final_cp } else { -final_cp }
    }
}