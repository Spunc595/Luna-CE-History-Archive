use crate::board::{Scacchiera, Colore};
use std::fs::File;
use std::io::Read;

// Dimensioni della rete (Half-KP standard)
pub const INPUT_SIZE: usize = 768;   // 64 case * 12 pezzi
pub const HIDDEN_SIZE: usize = 256;  // Neuroni nello strato nascosto

// Fattori di Quantizzazione usati nel training (QA per pesi/bias input, QB per output)
const QA: i32 = 255;
const QB: i32 = 64;

// SCALE_DIVISOR: Questo è il nostro "calmieratore". 
// Poiché il training usa un target molto alto (2000), dividiamo qui per 16 
// per riportare la valutazione nel range standard UCI dei centipedoni (-1000/+1000).
const SCALE_DIVISOR: i32 = 16; 

pub struct LunaNNUE {
    pub feature_weights: Vec<i16>,
    pub feature_bias: Vec<i16>,
    pub output_weights: Vec<i16>,
    pub output_bias: i16,
}

impl LunaNNUE {
    /// Carica il file .nnue binario esportato dallo script Python
    pub fn load(path: &str) -> Option<Self> {
        let mut file = File::open(path).ok()?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).ok()?;
        
        if buffer.len() < 100 { return None; }
        
        let mut offset = 0;
        let mut read_quantized = |count: usize, scale: f32| -> Vec<i16> {
            let mut v = Vec::with_capacity(count);
            for _ in 0..count {
                if offset + 4 > buffer.len() { break; }
                let bytes = [buffer[offset], buffer[offset+1], buffer[offset+2], buffer[offset+3]];
                offset += 4;
                let val_f32 = f32::from_le_bytes(bytes);
                let val_i16 = (val_f32 * scale).round() as i16;
                v.push(val_i16);
            }
            v
        };

        // Caricamento pesi nell'ordine esatto dell'export
        let fw = read_quantized(INPUT_SIZE * HIDDEN_SIZE, QA as f32);
        let fb = read_quantized(HIDDEN_SIZE, QA as f32);
        let ow = read_quantized(HIDDEN_SIZE, QB as f32);
        let ob_vec = read_quantized(1, QB as f32);
        let ob = if !ob_vec.is_empty() { ob_vec[0] } else { 0 };

        Some(Self { 
            feature_weights: fw, 
            feature_bias: fb, 
            output_weights: ow, 
            output_bias: ob 
        })
    }

    /// Valuta la posizione. Ritorna score in Negamax (positivo = buono per chi muove)
    #[inline(always)]
    pub fn evaluate(&self, board: &Scacchiera) -> i32 {
        let mut acc = [0i32; HIDDEN_SIZE];
        
        // 1. Inizializzazione con i bias dello strato nascosto
        for i in 0..HIDDEN_SIZE { 
            acc[i] = self.feature_bias[i] as i32; 
        }

        // 2. Accumulazione Feature (Iteriamo sui pezzi presenti)
        for p_idx in 0..6 {
            // Bianchi (0-5)
            let mut white_bb = board.pezzi[p_idx] & board.colori[0];
            while white_bb != 0 {
                let sq = white_bb.trailing_zeros() as usize;
                let feature_idx = (p_idx * 64) + sq;
                self.add_feature(feature_idx, &mut acc);
                white_bb &= white_bb - 1;
            }

            // Neri (6-11)
            let mut black_bb = board.pezzi[p_idx] & board.colori[1];
            while black_bb != 0 {
                let sq = black_bb.trailing_zeros() as usize;
                let feature_idx = ((p_idx + 6) * 64) + sq;
                self.add_feature(feature_idx, &mut acc);
                black_bb &= black_bb - 1;
            }
        }

        // 3. Calcolo dello strato di Output
        // Usiamo i64 per l'accumulatore finale per evitare overflow tecnici
        let mut output = 0i64;
        for i in 0..HIDDEN_SIZE {
            // Attivazione Clipped ReLU (0 - QA)
            let activation = acc[i].clamp(0, QA);
            output += (activation as i64) * (self.output_weights[i] as i64);
        }

        // 4. Dequantizzazione e Normalizzazione UCI
        // (output / QA) riporta lo score alla scala QB del training.
        let raw_score = (output / QA as i64) + (self.output_bias as i64);
        
        // Lo dividiamo per il nostro SCALE_DIVISOR per avere centipedoni umani
        let scaled_score = (raw_score as i32) / SCALE_DIVISOR;

        // 5. Negamax: l'engine vuole lo score relativo a chi tocca muovere
        if board.turno == Colore::Bianco { 
            scaled_score 
        } else { 
            -scaled_score 
        }
    }

    #[inline(always)]
    fn add_feature(&self, feature_idx: usize, acc: &mut [i32; HIDDEN_SIZE]) {
        let weight_start = feature_idx * HIDDEN_SIZE;
        if let Some(weights) = self.feature_weights.get(weight_start..weight_start + HIDDEN_SIZE) {
            // Questo loop è scritto in modo che il compilatore possa ottimizzarlo con SIMD
            for i in 0..HIDDEN_SIZE {
                acc[i] += weights[i] as i32;
            }
        }
    }
}