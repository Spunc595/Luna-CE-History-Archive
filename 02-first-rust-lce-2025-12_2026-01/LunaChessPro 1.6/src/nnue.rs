use std::io;
use crate::board::{Scacchiera, Pezzo, Colore, Casella};

lazy_static! {
    pub static ref NNUE: Option<NnueNetwork> = {
        println!("info string Inizializzazione NNUE (Architettura HalfKP)...");
        // Il file eval.nnue deve trovarsi nella cartella principale del progetto
        static RAW_NET: &[u8] = include_bytes!("../eval.nnue");

        match NnueNetwork::load_from_memory(RAW_NET) {
            Ok(net) => {
                println!("info string SUCCESS: Rete NNUE caricata e verificata.");
                Some(net)
            }
            Err(e) => {
                println!("info string ERRORE CARICAMENTO: {}. Uso valutazione classica.", e);
                None
            }
        }
    };
}

const TRANSFORMER_DIMS: usize = 256; 
const INPUT_SIZE: usize = 41024; 

#[derive(Clone)]
pub struct NnueNetwork {
    pub feature_weights: Vec<i16>, 
    pub feature_biases: Vec<i16>,  
    pub output_weights: Vec<i16>,  
    pub output_bias: i16,
}

impl NnueNetwork {
    pub fn new() -> Self {
        NnueNetwork {
            feature_weights: Vec::new(),
            feature_biases: vec![0; TRANSFORMER_DIMS],
            output_weights: vec![0; TRANSFORMER_DIMS * 2],
            output_bias: 0,
        }
    }

    pub fn load_from_memory(data: &[u8]) -> io::Result<Self> {
        const FEAT_BIAS_SIZE: usize = TRANSFORMER_DIMS * 2;
        const FEAT_WEIGHT_SIZE: usize = INPUT_SIZE * TRANSFORMER_DIMS * 2;
        const OUT_WEIGHT_SIZE: usize = TRANSFORMER_DIMS * 2 * 2;
        const OUT_BIAS_SIZE: usize = 2;
        const TOTAL_DATA_SIZE: usize = FEAT_BIAS_SIZE + FEAT_WEIGHT_SIZE + OUT_WEIGHT_SIZE + OUT_BIAS_SIZE;

        if data.len() < TOTAL_DATA_SIZE {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "File NNUE troppo piccolo"));
        }

        let header_size = data.len() - TOTAL_DATA_SIZE;
        let mut cursor = header_size; 

        let mut net = NnueNetwork::new();
        
        let mut read_vec = |count: usize| {
            let mut v = Vec::with_capacity(count);
            for _ in 0..count {
                v.push(i16::from_le_bytes([data[cursor], data[cursor+1]]));
                cursor += 2;
            }
            v
        };

        net.feature_biases = read_vec(TRANSFORMER_DIMS);
        net.feature_weights = read_vec(INPUT_SIZE * TRANSFORMER_DIMS);
        net.output_weights = read_vec(TRANSFORMER_DIMS * 2);
        net.output_bias = i16::from_le_bytes([data[cursor], data[cursor+1]]);

        Ok(net)
    }

    pub fn evaluate(&self, s: &Scacchiera) -> i32 {
        let mut active_w = Vec::with_capacity(32);
        let mut active_b = Vec::with_capacity(32);

        let k_sq_w = s.bitboard_pezzo_colore(Pezzo::Re, Colore::Bianco).trailing_zeros() as usize;
        let k_sq_b = s.bitboard_pezzo_colore(Pezzo::Re, Colore::Nero).trailing_zeros() as usize;
        let k_sq_b_rel = k_sq_b ^ 56; // Specchia la casa del re nero

        for sq in 0..64 {
            if let Some((p, c)) = s.pezzo_su_casella(Casella(sq)) {
                if p == Pezzo::Re { continue; } 
                let p_idx = p.indice(); 
                
                // Feature Mapping: King-Piece relative positions
                let c_w = if c == Colore::Bianco { 0 } else { 1 }; 
                active_w.push(k_sq_w * 641 + (p_idx + c_w * 5) * 64 + sq);

                let c_b = if c == Colore::Nero { 0 } else { 1 };
                active_b.push(k_sq_b_rel * 641 + (p_idx + c_b * 5) * 64 + (sq ^ 56));
            }
        }

        let acc_w = self.calculate_layer1(&active_w);
        let acc_b = self.calculate_layer1(&active_b);

        // Seleziona l'accumulatore in base al turno
        let (us, them) = if s.turno() == Colore::Bianco { (acc_w, acc_b) } else { (acc_b, acc_w) };
        
        let mut sum = self.output_bias as i32;
        for i in 0..TRANSFORMER_DIMS {
            // Clipped ReLU (0-127)
            let val_us = us[i].max(0).min(127) as i32; 
            let val_them = them[i].max(0).min(127) as i32;
            
            sum += val_us * self.output_weights[i] as i32;
            sum += val_them * self.output_weights[TRANSFORMER_DIMS + i] as i32;
        }

        // DIVISORE DI SCALA: 4000
        // Necessario per stabilizzare la valutazione in Arena ed evitare salti improvvisi.
        sum / 4000
    }

    fn calculate_layer1(&self, features: &[usize]) -> Vec<i16> {
        let mut acc = self.feature_biases.clone();
        for &idx in features {
            let offset = idx * TRANSFORMER_DIMS;
            for i in 0..TRANSFORMER_DIMS {
                acc[i] = acc[i].saturating_add(self.feature_weights[offset + i]);
            }
        }
        acc
    }
}