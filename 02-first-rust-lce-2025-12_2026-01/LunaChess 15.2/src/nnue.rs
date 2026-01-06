use crate::board::{Colore, Pezzo, Scacchiera};
use std::fs::File;
use std::io::Read;

const NNUE_SCALE: i32 = 400; 

#[derive(Clone)]
pub struct Network {
    pub feature_weights: Vec<i16>,
    pub feature_bias: Vec<i16>,
    pub output_weights: Vec<i16>,
    pub output_bias: i16,
}

impl Network {
    pub fn carica(path: &str) -> Self {
        let mut file = match File::open(path) {
            Ok(f) => f,
            Err(_) => {
                println!("info string Warning: NNUE file not found.");
                return Self::dummy();
            }
        };

        let mut buffer = Vec::new();
        if file.read_to_end(&mut buffer).is_err() {
            return Self::dummy();
        }
        
        let feature_input = 768; 
        let hidden_size = 256;   
        
        // Lettura sicura (Best Effort)
        let safe_read_i16 = |buf: &[u8], start: usize, count: usize| -> Vec<i16> {
            let mut res = Vec::with_capacity(count);
            let mut i = start;
            for _ in 0..count {
                if i + 1 >= buf.len() { break; }
                let val = i16::from_le_bytes([buf[i], buf[i+1]]);
                res.push(val);
                i += 2;
            }
            res
        };

        let f_bias = safe_read_i16(&buffer, 0, hidden_size); 
        let f_weights = safe_read_i16(&buffer, hidden_size * 2, feature_input * hidden_size);

        // Output weights alla fine
        let output_w_size = hidden_size * 2 * 2; 
        let output_start = if buffer.len() > output_w_size + 2 {
            buffer.len() - output_w_size - 2
        } else { 0 };

        let o_weights = safe_read_i16(&buffer, output_start, hidden_size * 2);
        let o_bias = if buffer.len() >= 2 {
            i16::from_le_bytes([buffer[buffer.len()-2], buffer[buffer.len()-1]])
        } else { 0 };

        if f_weights.len() < feature_input * hidden_size {
             return Self::dummy();
        }

        Network {
            feature_weights: f_weights,
            feature_bias: f_bias,
            output_weights: o_weights,
            output_bias: o_bias,
        }
    }

    pub fn dummy() -> Self {
        Network {
            feature_weights: vec![0; 768 * 256],
            feature_bias: vec![0; 256],
            output_weights: vec![0; 256 * 2],
            output_bias: 0,
        }
    }
}

pub fn get_feature_index(pezzo: Pezzo, colore: Colore, sq: usize) -> usize {
    let piece_offset = pezzo.indice();
    let color_offset = if colore == Colore::Bianco { 0 } else { 6 };
    (color_offset + piece_offset) * 64 + sq
}

pub fn evaluate_nnue(board: &Scacchiera, net: &Network) -> i32 {
    let us = board.turno.indice();
    let them = board.turno.opposto().indice();
    
    let acc_us = &board.current_acc[us];
    let acc_them = &board.current_acc[them];
    
    let mut output_sum: i32 = net.output_bias as i32;
    let hidden_size = 256; 
    
    if net.output_weights.len() < hidden_size * 2 { return 0; }

    for i in 0..hidden_size {
        // Attivazione Clipped ReLU: clamp(0, 127) -> Fondamentale!
        let val_us = acc_us.v[i].max(0).min(127) as i32;
        let val_them = acc_them.v[i].max(0).min(127) as i32;
        
        let w_us = net.output_weights[i] as i32;
        let w_them = net.output_weights[hidden_size + i] as i32;
        
        output_sum += val_us * w_us;
        output_sum += val_them * w_them;
    }
    
    // Scaling: Dividiamo per 400 per ottenere centipawn
    output_sum / NNUE_SCALE
}