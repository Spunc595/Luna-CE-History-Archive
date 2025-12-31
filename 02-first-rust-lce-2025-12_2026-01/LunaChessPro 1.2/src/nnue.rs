use std::io;
use crate::board::{Scacchiera, Pezzo, Colore};

// =============================================================
// 1. CARICAMENTO SICURO CON SANITY CHECK
// =============================================================
lazy_static! {
    pub static ref NNUE: Option<NnueNetwork> = {
        println!("info string Inizializzazione NNUE (Architettura HalfKP)...");
        
        // 1. Includi il file nell'eseguibile
        static RAW_NET: &[u8] = include_bytes!("../eval.nnue");

        // 2. Tenta il caricamento
        match NnueNetwork::load_from_memory(RAW_NET) {
            Ok(net) => {
                // 3. SANITY CHECK (IL SALVAGENTE)
                // Controlliamo il bias del primo neurone. In una rete sana è un numero piccolo (es. +/- 100).
                // Se abbiamo letto l'header di testo per sbaglio, questo numero sarà enorme (es. 28000).
                let test_val = net.feature_biases[0];
                if test_val.abs() > 2000 {
                    println!("info string ERRORE: Rete corrotta rilevata (Sanity Check Fallito: val={}).", test_val);
                    println!("info string CAUSA: Probabile disallineamento header.");
                    println!("info string AZIONE: Disabilito NNUE e uso PeSTO.");
                    None
                } else {
                    println!("info string SUCCESS: Rete NNUE caricata e verificata. Motore al 100% della potenza.");
                    Some(net)
                }
            }
            Err(e) => {
                println!("info string ERRORE CARICAMENTO: {}", e);
                println!("info string AZIONE: Uso PeSTO.");
                None
            }
        }
    };
}

// =============================================================
// 2. COSTANTI ARCHITETTURA (Stockfish 12 Standard)
// =============================================================
const TRANSFORMER_DIMS: usize = 256; 
// HalfKP Standard: (64 Re * 641 Stride) = 41024
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
        // CALCOLO ESATTO DIMENSIONE DATI RAW
        // Transformer Biases: 256 * 2 bytes
        // Transformer Weights: 41024 * 256 * 2 bytes
        // Output Weights: 512 * 2 bytes
        // Output Bias: 2 bytes
        
        const FEAT_BIAS_SIZE: usize = TRANSFORMER_DIMS * 2;
        const FEAT_WEIGHT_SIZE: usize = INPUT_SIZE * TRANSFORMER_DIMS * 2;
        const OUT_WEIGHT_SIZE: usize = TRANSFORMER_DIMS * 2 * 2;
        const OUT_BIAS_SIZE: usize = 2;
        
        const TOTAL_DATA_SIZE: usize = FEAT_BIAS_SIZE + FEAT_WEIGHT_SIZE + OUT_WEIGHT_SIZE + OUT_BIAS_SIZE;

        if data.len() < TOTAL_DATA_SIZE {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "File troppo piccolo"));
        }

        // TRUCCO: Ignoriamo l'inizio. Partiamo dalla fine e torniamo indietro.
        // Tutto ciò che c'è prima è header (testo, versioni, hash, copyright...)
        let header_size = data.len() - TOTAL_DATA_SIZE;
        let mut cursor = header_size; 

        let mut net = NnueNetwork::new();
        
        // Helper lettura
        let read_vec = |buff: &[u8], offset: &mut usize, count: usize| -> Vec<i16> {
            let mut v = Vec::with_capacity(count);
            for _ in 0..count {
                let val = i16::from_le_bytes([buff[*offset], buff[*offset+1]]);
                *offset += 2;
                v.push(val);
            }
            v
        };

        // ORDINE DI LETTURA STANDARD SF12
        // 1. Feature Transformer Biases (256)
        net.feature_biases = read_vec(data, &mut cursor, TRANSFORMER_DIMS);
        
        // 2. Feature Transformer Weights (41024 * 256)
        net.feature_weights = read_vec(data, &mut cursor, INPUT_SIZE * TRANSFORMER_DIMS);
        
        // 3. Output Network Weights (512)
        net.output_weights = read_vec(data, &mut cursor, TRANSFORMER_DIMS * 2);
        
        // 4. Output Network Bias (1)
        if cursor + 2 <= data.len() {
            let b_bytes = [data[cursor], data[cursor+1]];
            net.output_bias = i16::from_le_bytes(b_bytes);
        }

        Ok(net)
    }

    // =============================================================
    // 3. INFERENZA
    // =============================================================
    pub fn evaluate(&self, s: &Scacchiera) -> i32 {
        let mut active_features_w = Vec::with_capacity(32);
        let mut active_features_b = Vec::with_capacity(32);

        let k_sq_w = s.bitboard_pezzo_colore(Pezzo::Re, Colore::Bianco).trailing_zeros() as usize;
        let k_sq_b = s.bitboard_pezzo_colore(Pezzo::Re, Colore::Nero).trailing_zeros() as usize;
        let k_sq_b_rel = k_sq_b ^ 56; 

        for sq in 0..64 {
            if let Some((p, c)) = s.pezzo_su_casella(crate::board::Casella(sq)) {
                if p == Pezzo::Re { continue; } 
                
                let p_idx = p.indice(); 
                
                // Bianco
                let c_w = if c == Colore::Bianco { 0 } else { 1 }; 
                let final_idx_w = k_sq_w * 641 + (p_idx + c_w * 5) * 64 + sq; // 641 Stride
                active_features_w.push(final_idx_w);

                // Nero
                let sq_rel = sq ^ 56; 
                let c_b = if c == Colore::Nero { 0 } else { 1 };
                let final_idx_b = k_sq_b_rel * 641 + (p_idx + c_b * 5) * 64 + sq_rel; // 641 Stride
                active_features_b.push(final_idx_b);
            }
        }

        let acc_w = self.calculate_layer1(&active_features_w);
        let acc_b = self.calculate_layer1(&active_features_b);

        let (us, them) = if s.turno() == Colore::Bianco { (acc_w, acc_b) } else { (acc_b, acc_w) };
        
        // Output Layer
        let mut sum = self.output_bias as i32;
        for i in 0..TRANSFORMER_DIMS {
            // Clipped ReLU (0..127)
            let input_us = us[i].max(0).min(127) as i32; 
            let input_them = them[i].max(0).min(127) as i32;

            sum += input_us * self.output_weights[i] as i32;
            sum += input_them * self.output_weights[TRANSFORMER_DIMS + i] as i32;
        }

        // Divisione per 16 (Standard SF) -> Centipawns
        (sum / 16) as i32
    }

    fn calculate_layer1(&self, features: &[usize]) -> Vec<i16> {
        let mut acc = self.feature_biases.clone();
        for &idx in features {
            let offset = idx * TRANSFORMER_DIMS;
            if offset + TRANSFORMER_DIMS <= self.feature_weights.len() {
                for i in 0..TRANSFORMER_DIMS {
                    acc[i] = acc[i].saturating_add(self.feature_weights[offset + i]);
                }
            }
        }
        acc
    }
}