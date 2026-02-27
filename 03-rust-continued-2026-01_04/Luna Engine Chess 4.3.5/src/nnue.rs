use std::fs::File;
use std::io::Read;
use crate::board::{Scacchiera, Colore, Pezzo};

pub const INPUT_SIZE: usize = 768; 
pub const L1_SIZE: usize = 256;
pub const L2_SIZE: usize = 32;

const QA: i64 = 255;
const QB: i64 = 64;

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

        let mut net = LunaNNUE {
            l1_weights: vec![0; L1_SIZE * INPUT_SIZE],
            l2_weights: vec![0; L2_SIZE * L1_SIZE],
            l2_bias: vec![0; L2_SIZE],
            l3_weights: vec![0; L2_SIZE],
            l3_bias: vec![0; 1],
        };

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

        net.l1_weights = read_i16(L1_SIZE * INPUT_SIZE);
        let _unused_l1_bias = read_i16(L1_SIZE); 
        net.l2_weights = read_i16(L2_SIZE * L1_SIZE);
        net.l2_bias = read_i16(L2_SIZE);
        net.l3_weights = read_i16(L2_SIZE);
        net.l3_bias = read_i16(1);

        if net.l1_weights.iter().take(100).all(|&x| x == 0) { return None; }
        Some(net)
    }

    pub fn get_feature_index(colore: Colore, pezzo: Pezzo, sq: usize) -> usize {
        let p_type = match pezzo {
            Pezzo::Pedone => 0, Pezzo::Cavallo => 1, Pezzo::Alfiere => 2,
            Pezzo::Torre => 3, Pezzo::Regina => 4, Pezzo::Re => 5,
        };
        let p_idx = if colore == Colore::Bianco { p_type } else { p_type + 6 };
        p_idx * 64 + sq
    }

    pub fn update_add(&self, acc: &mut Accumulator, feature_idx: usize) {
        let offset = feature_idx; 
        for i in 0..L1_SIZE {
            acc.vals[i] += self.l1_weights[i * INPUT_SIZE + offset] as i32;
        }
    }

    pub fn update_remove(&self, acc: &mut Accumulator, feature_idx: usize) {
        let offset = feature_idx;
        for i in 0..L1_SIZE {
            acc.vals[i] -= self.l1_weights[i * INPUT_SIZE + offset] as i32;
        }
    }

    pub fn evaluate(&self, s: &Scacchiera) -> i32 {
        // --- LAYER 2 (Ora allocato sullo STACK e non sull'HEAP!) ---
        let mut l2_acc = [0i64; L2_SIZE];
        for i in 0..L2_SIZE {
            let mut sum = (self.l2_bias[i] as i64) * QA;
            for j in 0..L1_SIZE {
                let activation = s.accumulator.vals[j].clamp(0, QA as i32) as i64;
                sum += activation * (self.l2_weights[i * L1_SIZE + j] as i64);
            }
            l2_acc[i] = sum / QA;
        }

        // --- LAYER 3 ---
        let mut score = (self.l3_bias[0] as i64) * QB;
        for i in 0..L2_SIZE {
            let activation = l2_acc[i].max(0);
            score += activation * (self.l3_weights[i] as i64);
        }

        let v_win_prob_scaled = score; 
        let v_draw_point_scaled = (QB * QB) / 2;
        let cp = ((v_win_prob_scaled - v_draw_point_scaled) * 1000) / (QB * QB / 2);
        
        let val = cp as i32;
        if s.turno == Colore::Bianco { val } else { -val }
    }
}