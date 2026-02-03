use std::fs::File;
use std::io::Read;
use crate::board::{Scacchiera, Colore, Mossa, MoveFlag, Pezzo};

const INPUT_SIZE: usize = 768; 
const L1_SIZE: usize = 256;
const L2_SIZE: usize = 32;

#[derive(Clone, Debug)]
pub struct Accumulator {
    pub vals: [i32; L1_SIZE],
}

impl Default for Accumulator {
    fn default() -> Self {
        Accumulator { vals: [0; L1_SIZE] }
    }
}

pub struct LunaNNUE {
    l1_weights: Vec<i16>,
    l1_bias: Vec<i16>,
    l2_weights_f: Vec<f32>,
    l2_bias_f: Vec<f32>,
    l3_weights_f: Vec<f32>,
    l3_bias_f: Vec<f32>,
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
                    v.push(i16::from_le_bytes([buffer[offset], buffer[offset+1]]));
                    offset += 2;
                }
            }
            v
        };

        let raw_l1_w = read_i16(INPUT_SIZE * L1_SIZE);
        let raw_l1_b = read_i16(L1_SIZE);
        let raw_l2_w = read_i16(L1_SIZE * L2_SIZE);
        let raw_l2_b = read_i16(L2_SIZE);
        let raw_l3_w = read_i16(L2_SIZE);
        let raw_l3_b = read_i16(1);

        let net = LunaNNUE {
            l1_weights: raw_l1_w,
            l1_bias: raw_l1_b,
            l2_weights_f: raw_l2_w.iter().map(|&w| w as f32 / 64.0).collect(),
            l2_bias_f: raw_l2_b.iter().map(|&w| w as f32 / 64.0).collect(),
            l3_weights_f: raw_l3_w.iter().map(|&w| w as f32 / 64.0).collect(),
            l3_bias_f: raw_l3_b.iter().map(|&w| w as f32 / 64.0).collect(),
        };

        if net.l1_weights.iter().take(100).all(|&x| x == 0) {
            println!("⚠️ ATTENZIONE: Il file NNUE sembra vuoto!");
        } else {
            println!("🚀 NNUE: Ottimizzazione Incrementale Attiva (Precisione PyTorch)");
        }

        Some(net)
    }

    #[inline(always)]
    fn get_feature_idx(p_idx: usize, col: usize, sq: usize) -> usize {
        let color_offset = if col == 0 { 0 } else { 6 };
        (p_idx + color_offset) * 64 + sq
    }

    pub fn refresh_accumulator(&self, s: &Scacchiera) -> Accumulator {
        let mut acc = Accumulator::default();
        for i in 0..L1_SIZE { acc.vals[i] = self.l1_bias[i] as i32; }

        for p_idx in 0..6 {
            for col in 0..2 {
                let mut bb = s.pezzi[p_idx] & s.colori[col];
                while bb != 0 {
                    let sq = bb.trailing_zeros() as usize;
                    let feature_idx = Self::get_feature_idx(p_idx, col, sq);
                    for i in 0..L1_SIZE {
                        acc.vals[i] += self.l1_weights[i * INPUT_SIZE + feature_idx] as i32;
                    }
                    bb &= bb - 1;
                }
            }
        }
        acc
    }

    /// LA CHIAVE DELLA VELOCITÀ: Aggiorna solo ciò che è cambiato!
    pub fn update_accumulator(&self, old_acc: &Accumulator, m: &Mossa, s: &Scacchiera) -> Accumulator {
        let mut new_acc = old_acc.clone();
        let from = m.da();
        let to = m.a();
        let flag = m.move_flag();
        let us = s.turno.indice();
        let them = 1 - us;

        // Trova il pezzo mosso (sulla scacchiera attuale, prima che la mossa sia processata)
        let mut moved_p = 0;
        for p in 0..6 { if (s.pezzi[p] & (1u64 << from)) != 0 { moved_p = p; break; } }

        // 1. Sottrai il pezzo dalla casa di partenza
        let idx_from = Self::get_feature_idx(moved_p, us, from);
        for i in 0..L1_SIZE {
            new_acc.vals[i] -= self.l1_weights[i * INPUT_SIZE + idx_from] as i32;
        }

        // 2. Gestisci la cattura
        if flag == MoveFlag::EnPassant {
            let cap_sq = if us == 0 { to - 8 } else { to + 8 };
            let idx_cap = Self::get_feature_idx(0, them, cap_sq);
            for i in 0..L1_SIZE {
                new_acc.vals[i] -= self.l1_weights[i * INPUT_SIZE + idx_cap] as i32;
            }
        } else if let Some(p_cap) = s.pezzo_in(to) {
            let idx_cap = Self::get_feature_idx(p_cap, them, to);
            for i in 0..L1_SIZE {
                new_acc.vals[i] -= self.l1_weights[i * INPUT_SIZE + idx_cap] as i32;
            }
        }

        // 3. Aggiungi il pezzo (o il pezzo promosso) alla casa di arrivo
        let final_p = if flag.is_promotion() { m.pezzo_promosso().unwrap().indice() } else { moved_p };
        let idx_to = Self::get_feature_idx(final_p, us, to);
        for i in 0..L1_SIZE {
            new_acc.vals[i] += self.l1_weights[i * INPUT_SIZE + idx_to] as i32;
        }

        // 4. Gestione Arrocco: muovi anche la Torre
        if flag == MoveFlag::Castle {
            let (r_from, r_to) = match to {
                6 => (7, 5), 2 => (0, 3), 62 => (63, 61), 58 => (56, 59), _ => (0,0)
            };
            let idx_rf = Self::get_feature_idx(3, us, r_from);
            let idx_rt = Self::get_feature_idx(3, us, r_to);
            for i in 0..L1_SIZE {
                new_acc.vals[i] = new_acc.vals[i] - (self.l1_weights[i * INPUT_SIZE + idx_rf] as i32) + (self.l1_weights[i * INPUT_SIZE + idx_rt] as i32);
            }
        }

        new_acc
    }

    pub fn evaluate_from_acc(&self, acc: &Accumulator, turno: Colore) -> i32 {
        let mut l1_out = [0.0_f32; 256];
        for i in 0..256 {
            l1_out[i] = (acc.vals[i] as f32 / 255.0).max(0.0);
        }

        let mut l2_out = [0.0_f32; 32];
        for i in 0..32 {
            let mut sum = self.l2_bias_f[i];
            let offset = i * 256;
            for j in 0..256 {
                sum += l1_out[j] * self.l2_weights_f[offset + j];
            }
            l2_out[i] = sum.max(0.0);
        }

        let mut score = self.l3_bias_f[0];
        for j in 0..32 {
            score += l2_out[j] * self.l3_weights_f[j];
        }

        let cp_score = (score - 0.5) * 1500.0; 
        let final_score = (cp_score.round() as i32).clamp(-40000, 40000);
        
        if turno == Colore::Bianco { final_score } else { -final_score }
    }
}