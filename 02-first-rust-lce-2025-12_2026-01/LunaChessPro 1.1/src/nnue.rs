use std::io;
use crate::board::{Scacchiera, Pezzo, Colore};

// =============================================================
// 1. CARICAMENTO EMBEDDED (Rete inclusa nell'EXE)
// =============================================================
lazy_static! {
    pub static ref NNUE: Option<NnueNetwork> = {
        println!("info string Caricamento rete NNUE integrata...");
        
        // Questa macro legge il file DURANTE LA COMPILAZIONE.
        // Se il file non esiste, il codice NON COMPILERÀ (ti darà errore subito).
        // Il percorso "../eval.nnue" parte dalla cartella 'src', quindi cerca nella root.
        static RAW_NET: &[u8] = include_bytes!("../eval.nnue");

        match NnueNetwork::load_from_memory(RAW_NET) {
            Ok(net) => {
                println!("info string Rete NNUE integrata attivata con successo!");
                Some(net)
            }
            Err(e) => {
                println!("info string ERRORE CRITICO INTERNO: {}", e);
                None
            }
        }
    };
}

// =============================================================
// 2. ARCHITETTURA RETE (HalfKP 256x2)
// =============================================================
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

    // =============================================================
    // 3. LOADER DA MEMORIA (Non legge più da disco)
    // =============================================================
    pub fn load_from_memory(data: &[u8]) -> io::Result<Self> {
        // Calcolo dimensione dati matematici
        let input_weights_size = INPUT_SIZE * TRANSFORMER_DIMS * 2;
        let input_biases_size = TRANSFORMER_DIMS * 2;
        let output_weights_size = TRANSFORMER_DIMS * 2 * 2;
        let output_bias_size = 2;
        
        let total_data_size = input_weights_size + input_biases_size + output_weights_size + output_bias_size;
        
        if data.len() < total_data_size {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof, 
                format!("Rete corrotta/piccola! Attesi {} bytes, trovati {}", total_data_size, data.len())
            ));
        }

        // Saltiamo l'header (che è all'inizio) prendendo gli ultimi N bytes
        let header_size = data.len() - total_data_size;
        let mut cursor = header_size; 

        let mut net = NnueNetwork::new();
        
        // Helper per leggere direttamente dalla slice di byte
        let read_vec = |buff: &[u8], offset: &mut usize, count: usize| -> Vec<i16> {
            let mut v = Vec::with_capacity(count);
            // Sicurezza: controlliamo di non uscire dai bordi (panic prevention)
            if *offset + count * 2 > buff.len() {
                return v; // O panic, ma qui ritorniamo vuoto per sicurezza
            }
            
            // Unrolling manuale o iterazione
            for _ in 0..count {
                // Little Endian conversion
                let val = i16::from_le_bytes([buff[*offset], buff[*offset+1]]);
                *offset += 2;
                v.push(val);
            }
            v
        };

        // CARICAMENTO PESI
        net.feature_biases = read_vec(data, &mut cursor, TRANSFORMER_DIMS);
        net.feature_weights = read_vec(data, &mut cursor, INPUT_SIZE * TRANSFORMER_DIMS);
        net.output_weights = read_vec(data, &mut cursor, TRANSFORMER_DIMS * 2);
        
        if cursor + 2 <= data.len() {
            let b_bytes = [data[cursor], data[cursor+1]];
            net.output_bias = i16::from_le_bytes(b_bytes);
        } else {
             return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "File tronco su output bias"));
        }

        Ok(net)
    }

    // =============================================================
    // 4. INFERENZA (Il Calcolo)
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
                let final_idx_w = k_sq_w * 640 + (p_idx + c_w * 5) * 64 + sq;
                active_features_w.push(final_idx_w);

                // Nero (Flip)
                let sq_rel = sq ^ 56; 
                let c_b = if c == Colore::Nero { 0 } else { 1 };
                let final_idx_b = k_sq_b_rel * 640 + (p_idx + c_b * 5) * 64 + sq_rel;
                active_features_b.push(final_idx_b);
            }
        }

        let acc_w = self.calculate_layer1(&active_features_w);
        let acc_b = self.calculate_layer1(&active_features_b);

        let (us, them) = if s.turno() == Colore::Bianco { (acc_w, acc_b) } else { (acc_b, acc_w) };
        let output = self.calculate_output(&us, &them);
        
        (output / 16) as i32
    }

    fn calculate_layer1(&self, features: &[usize]) -> Vec<i16> {
        let mut acc = self.feature_biases.clone();
        for &idx in features {
            let offset = idx * TRANSFORMER_DIMS;
            for i in 0..TRANSFORMER_DIMS {
                // Safety check: se l'indice calcolato è fuori bounds, lo ignoriamo
                if offset + i < self.feature_weights.len() {
                    acc[i] = acc[i].saturating_add(self.feature_weights[offset + i]);
                }
            }
        }
        acc
    }

    fn calculate_output(&self, us: &[i16], them: &[i16]) -> i32 {
        let mut sum = self.output_bias as i32;

        for i in 0..TRANSFORMER_DIMS {
            let input_us = us[i].max(0).min(127) as i32; 
            let input_them = them[i].max(0).min(127) as i32;

            sum += input_us * self.output_weights[i] as i32;
            sum += input_them * self.output_weights[TRANSFORMER_DIMS + i] as i32;
        }
        sum
    }
}