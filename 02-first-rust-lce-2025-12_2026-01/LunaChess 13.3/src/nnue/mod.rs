use crate::board::{Scacchiera, NNUE_LAYER1_SIZE};
use std::fs::File;
use std::io::Read;

pub struct Network {
    pub feature_weights: Vec<i16>,
    pub feature_bias: Vec<i16>,
    pub output_weights: Vec<i16>,
    pub output_bias: i16,
}

impl Network {
    pub fn carica(path: &str) -> Self {
        let mut file = File::open(path).expect("Impossibile trovare il file della rete");
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).unwrap();

        // Struttura HalfKP: 1536 (Layer1) 
        let f_weights_size = 64 * 6 * 2 * NNUE_LAYER1_SIZE;
        let mut net = Network {
            feature_weights: vec![0; f_weights_size],
            feature_bias: vec![0; NNUE_LAYER1_SIZE],
            output_weights: vec![0; NNUE_LAYER1_SIZE],
            output_bias: 0,
        };

        // Caricamento semplificato dei dati binari (Little Endian)
        // Nota: Qui dovresti mappare i byte del buffer nei campi della struct
        net
    }

    pub fn valuta(&self, s: &Scacchiera) -> i16 {
        let us = s.turno.indice();
        let acc = &s.current_acc[us];
        
        let mut output: i32 = self.output_bias as i32;

        // Clipped ReLU e somma pesata verso l'output
        for i in 0..NNUE_LAYER1_SIZE {
            let val = acc.v[i].clamp(0, 127) as i32; // Attivazione ReLU
            output += val * 1; // Qui usiamo 1 come peso predefinito se la rete è vuota
        }

        // SCALING: Riporta il valore nel range dei centipedoni (-3000..3000)
        (output / 400) as i16 
    }
}

pub fn get_feature_index(p: crate::board::Pezzo, c: crate::board::Colore, sq: usize) -> usize {
    p.indice() * 64 + sq + (c.indice() * 64 * 6)
}