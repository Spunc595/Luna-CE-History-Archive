use crate::board::{Scacchiera, Colore, Pezzo, NNUE_LAYER1_SIZE};
use std::fs::File;
use std::io::Read;

const INPUT_SIZE: usize = 64 * 6 * 64; 

pub struct Network {
    pub feature_weights: Vec<i16>,
    pub feature_bias: Vec<i16>,
    pub output_weights: Vec<i16>,
    pub output_bias: i16,
}

impl Network {
    pub fn carica(path: &str) -> Self {
        println!("Tentativo di caricamento rete da: {}", path);
        let mut file = File::open(path).unwrap_or_else(|_| {
            panic!("ERRORE CRITICO: Impossibile trovare il file '{}'.", path);
        });

        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).expect("Errore durante la lettura del file NNUE");

        let mut offset = 0;

        let read_i16 = |buff: &[u8], off: &mut usize| -> i16 {
            let val = i16::from_le_bytes([buff[*off], buff[*off + 1]]);
            *off += 2;
            val
        };

        let f_weights_len = buffer.len() / 2;
        
        let mut net = Network {
            feature_weights: vec![0; f_weights_len.min(64 * 6 * 2 * NNUE_LAYER1_SIZE)],
            feature_bias: vec![0; NNUE_LAYER1_SIZE],
            output_weights: vec![0; NNUE_LAYER1_SIZE * 2],
            output_bias: 0,
        };
        
        if buffer.len() > 100 { 
            // Warning fix: Rimosso il reset ridondante di offset qui
            
            for i in 0..NNUE_LAYER1_SIZE {
                if offset + 2 <= buffer.len() {
                     net.feature_bias[i] = read_i16(&buffer, &mut offset);
                }
            }
            
            for i in 0..NNUE_LAYER1_SIZE * 2 {
                if offset + 2 <= buffer.len() {
                    net.output_weights[i] = read_i16(&buffer, &mut offset);
                }
            }

            if offset + 2 <= buffer.len() {
                net.output_bias = read_i16(&buffer, &mut offset);
            }
        }

        println!("Rete caricata. Dimensione buffer: {} bytes", buffer.len());
        net
    }

    pub fn valuta(&self, s: &Scacchiera) -> i16 {
        let us = s.turno.indice();
        let them = s.turno.opposto().indice();

        let acc_us = &s.current_acc[us];
        let acc_them = &s.current_acc[them];

        let mut output: i32 = self.output_bias as i32;
        
        for i in 0..NNUE_LAYER1_SIZE {
            let activation_us = acc_us.v[i].clamp(0, 127) as i32;
            let activation_them = acc_them.v[i].clamp(0, 127) as i32;

            output += activation_us * self.output_weights[i] as i32;
            output += activation_them * self.output_weights[i + NNUE_LAYER1_SIZE] as i32;
        }

        (output / 16) as i16 
    }
}

pub fn get_feature_index(p: Pezzo, c: Colore, sq: usize) -> usize {
    c.indice() * 384 + p.indice() * 64 + sq
}

pub fn evaluate_nnue(s: &Scacchiera, net: &Network) -> i32 {
    let val = net.valuta(s);
    val as i32
}