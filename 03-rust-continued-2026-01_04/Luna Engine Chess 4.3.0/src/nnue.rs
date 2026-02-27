use std::fs::File;
use std::io::Read;
use crate::board::{Scacchiera, Colore};

const INPUT_SIZE: usize = 768; // 64 case * 6 pezzi * 2 colori
const L1_SIZE: usize = 256;
const L2_SIZE: usize = 32;

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

        let mut net = LunaNNUE {
            l1_weights: vec![0; INPUT_SIZE * L1_SIZE],
            l1_bias: vec![0; L1_SIZE],
            l2_weights: vec![0; L1_SIZE * L2_SIZE],
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
                } else {
                    v.push(0);
                }
            }
            v
        };

        net.l1_weights = read_i16(INPUT_SIZE * L1_SIZE);
        net.l1_bias = read_i16(L1_SIZE);
        net.l2_weights = read_i16(L1_SIZE * L2_SIZE);
        net.l2_bias = read_i16(L2_SIZE);
        net.l3_weights = read_i16(L2_SIZE);
        net.l3_bias = read_i16(1);

        if net.l1_weights.iter().take(100).all(|&x| x == 0) {
            return None;
        }

        Some(net)
    }

    pub fn evaluate(&self, s: &Scacchiera) -> i32 {
        // --- LAYER 1: Feature Transformer ---
        // Inizializziamo l'accumulatore con i bias del primo layer
        let mut l1_acc = self.l1_bias.clone();

        // Mappatura efficiente dei pezzi tramite Bitboard
        for p_idx in 0..6 {
            // Pezzi Bianchi (0-5)
            let mut bb_w = s.pezzi[p_idx] & s.colori[0];
            while bb_w != 0 {
                let sq = bb_w.trailing_zeros() as usize;
                let feature_idx = p_idx * 64 + sq;
                let weights_offset = feature_idx * L1_SIZE;
                
                let piece_weights = &self.l1_weights[weights_offset..weights_offset + L1_SIZE];
                for i in 0..L1_SIZE {
                    l1_acc[i] = l1_acc[i].saturating_add(piece_weights[i]);
                }
                bb_w &= bb_w - 1; // Rimuove il bit processato
            }

            // Pezzi Neri (6-11)
            let mut bb_b = s.pezzi[p_idx] & s.colori[1];
            while bb_b != 0 {
                let sq = bb_b.trailing_zeros() as usize;
                let feature_idx = (p_idx + 6) * 64 + sq;
                let weights_offset = feature_idx * L1_SIZE;
                
                let piece_weights = &self.l1_weights[weights_offset..weights_offset + L1_SIZE];
                for i in 0..L1_SIZE {
                    l1_acc[i] = l1_acc[i].saturating_add(piece_weights[i]);
                }
                bb_b &= bb_b - 1;
            }
        }

        // Funzione di attivazione: Clipped ReLU
        let mut l1_out = [0i16; L1_SIZE];
        for i in 0..L1_SIZE {
            l1_out[i] = l1_acc[i].clamp(0, 127); // Quantizzazione standard
        }

        // --- LAYER 2: Hidden Layer ---
        let mut l2_out = [0i16; L2_SIZE];
        for i in 0..L2_SIZE {
            let mut sum: i32 = self.l2_bias[i] as i32;
            for j in 0..L1_SIZE {
                sum += (l1_out[j] as i32 * self.l2_weights[j * L2_SIZE + i] as i32);
            }
            // Clipping e scaling per il layer successivo
            l2_out[i] = (sum >> 6).clamp(0, 127) as i16;
        }

        // --- LAYER 3: Output Layer ---
        let mut score: i32 = self.l3_bias[0] as i32;
        for j in 0..L2_SIZE {
            score += (l2_out[j] as i32 * self.l3_weights[j] as i32);
        }

        // Scala finale per centipedoni UCI (solitamente / 16 o / 32 a seconda del training)
        let final_val = score / 16;
        
        if s.turno == Colore::Bianco { final_val } else { -final_val }
    }
}