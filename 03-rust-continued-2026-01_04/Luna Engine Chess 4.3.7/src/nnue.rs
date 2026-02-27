use std::fs::File;
use std::io::Read;
use crate::board::{Scacchiera, Colore, Pezzo};

pub const INPUT_SIZE: usize = 768; 
pub const L1_SIZE: usize = 256;
pub const L2_SIZE: usize = 32;

const QA: i32 = 255; 
const QB: i32 = 64;
const SCALE: i32 = 3000; 

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
    // l1_bias rimosso perché il tuo training Python non lo usa nel forward
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

        // Caricamento pesi seguendo l'ordine dei layer del tuo modello
        let l1_weights = read_i16(L1_SIZE * INPUT_SIZE);
        let _l1_bias = read_i16(L1_SIZE); // Leggiamo ma scartiamo (non usato nel forward)
        
        let l2_weights = read_i16(L2_SIZE * L1_SIZE);
        let l2_bias = read_i16(L2_SIZE);
        
        let l3_weights = read_i16(L2_SIZE);
        let l3_bias = read_i16(1);

        if l1_weights.iter().take(100).all(|&x| x == 0) { return None; }

        Some(LunaNNUE {
            l1_weights,
            l2_weights,
            l2_bias,
            l3_weights,
            l3_bias,
        })
    }

    #[inline(always)]
    pub fn get_feature_index(colore: Colore, pezzo: Pezzo, sq: usize) -> usize {
        let p_type = match pezzo {
            Pezzo::Pedone => 0, Pezzo::Cavallo => 1, Pezzo::Alfiere => 2,
            Pezzo::Torre => 3, Pezzo::Regina => 4, Pezzo::Re => 5,
        };
        // Coincide con SimpleFeatureSet: Bianco 0-5, Nero 6-11
        let p_idx = if colore == Colore::Bianco { p_type } else { p_type + 6 };
        p_idx * 64 + sq
    }

    pub fn update_add(&self, acc: &mut Accumulator, feature_idx: usize) {
        for i in 0..L1_SIZE {
            // L'indice i * INPUT_SIZE + feature_idx riflette il weight.t() di PyTorch
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
            acc.vals[i] = 0; 
        }
    }

    pub fn evaluate(&self, s: &Scacchiera) -> i32 {
        let mut l2_acc = [0i32; L2_SIZE];
        
        // --- Layer 2 ---
        for i in 0..L2_SIZE {
            let mut sum = (self.l2_bias[i] as i32) * QA;
            let offset = i * L1_SIZE;
            for j in 0..L1_SIZE {
                let activation = s.accumulator.vals[j].clamp(0, QA);
                sum += activation * (self.l2_weights[offset + j] as i32);
            }
            l2_acc[i] = sum / QA;
        }

        // --- Layer 3 ---
        let mut score = (self.l3_bias[0] as i32) * QB;
        for i in 0..L2_SIZE {
            let activation = l2_acc[i].max(0); // ReLU standard
            score += activation * (self.l3_weights[i] as i32);
        }

        let target_parita = (QB * QB) / 2;
        let centered_score = score - target_parita;
        let final_cp = (centered_score * SCALE) / (QB * QB);
        
        if s.turno == Colore::Bianco { final_cp } else { -final_cp }
    }
}