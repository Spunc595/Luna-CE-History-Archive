use std::fs::File;
use std::io::Read;
use crate::board::{Scacchiera, Colore, Pezzo};

// --- CONFIGURAZIONE ---
// Devono combaciare con il tuo script Python
const INPUT_SIZE: usize = 768;
const HIDDEN_SIZE: usize = 256;
const SCALE: i32 = 255; // Usato per de-quantizzare l'output

pub struct LunaNNUE {
    // Input -> Hidden (Flattened: 768 blocchi da 256 pesi)
    feature_weights: Vec<i16>, 
    // Bias del layer Hidden
    feature_biases: Vec<i16>,
    // Hidden -> Output
    output_weights: Vec<i16>,
    // Bias Output
    output_bias: i16,
}

impl LunaNNUE {
    /// Carica il file binario generato da train.py
    pub fn load(path: &str) -> Option<Self> {
        let mut file = match File::open(path) {
            Ok(f) => f,
            Err(_) => return None,
        };

        let mut buffer = Vec::new();
        if file.read_to_end(&mut buffer).is_err() {
            return None;
        }

        // Calcoliamo le dimensioni attese in byte (ogni i16 sono 2 byte)
        let w1_size = INPUT_SIZE * HIDDEN_SIZE * 2;
        let b1_size = HIDDEN_SIZE * 2;
        let w2_size = HIDDEN_SIZE * 2;
        let b2_size = 2;

        let total_size = w1_size + b1_size + w2_size + b2_size;

        if buffer.len() != total_size {
            println!("NNUE Error: File size mismatch. Expected {} bytes, got {}", total_size, buffer.len());
            return None;
        }

        let mut offset = 0;

        // Helper per leggere un vettore di i16
        let read_i16_vec = |buff: &[u8], count: usize, off: &mut usize| -> Vec<i16> {
            let mut vec = Vec::with_capacity(count);
            for _ in 0..count {
                let chunk = &buff[*off..*off+2];
                // Python struct.pack usa Little Endian di default su x86, assumiamo questo
                let val = i16::from_le_bytes([chunk[0], chunk[1]]);
                vec.push(val);
                *off += 2;
            }
            vec
        };

        // 1. Feature Weights (768 * 256)
        let feature_weights = read_i16_vec(&buffer, INPUT_SIZE * HIDDEN_SIZE, &mut offset);

        // 2. Feature Biases (256)
        let feature_biases = read_i16_vec(&buffer, HIDDEN_SIZE, &mut offset);

        // 3. Output Weights (256)
        let output_weights = read_i16_vec(&buffer, HIDDEN_SIZE, &mut offset);

        // 4. Output Bias (1)
        let output_bias = i16::from_le_bytes([buffer[offset], buffer[offset+1]]);

        Some(LunaNNUE {
            feature_weights,
            feature_biases,
            output_weights,
            output_bias,
        })
    }

    /// Valuta la posizione.
    /// Replica la logica di parse_fen() del tuo script Python.
    pub fn evaluate(&self, board: &Scacchiera) -> i32 {
        // 1. Inizializza l'accumulatore con i bias
        // Usiamo i32 per evitare overflow durante la somma
        let mut accumulator: Vec<i32> = self.feature_biases.iter().map(|&x| x as i32).collect();

        // 2. Trova le feature attive (pezzi sulla scacchiera)
        // Dobbiamo applicare la prospettiva RELATIVA a chi muove, come nel training.
        let us = board.turno;
        let is_black = us == Colore::Nero;

        // Iteriamo su tutte le caselle per trovare i pezzi
        let occ = board.occupazione();
        let mut temp_occ = occ;
        
        while temp_occ != 0 {
            let sq = temp_occ.trailing_zeros() as usize; // 0..63
            temp_occ &= temp_occ - 1;

            if let Some((colore_pezzo, tipo_pezzo)) = board.pezzo_e_colore_in(sq) {
                // Logica di conversione uguale a parse_fen di Python
                let mut rank = sq / 8;
                let col = sq % 8;
                
                // Indice base pezzo (Bianco: 0-5, Nero: 6-11)
                // P=0, N=1, B=2, R=3, Q=4, K=5
                let mut p_idx = tipo_pezzo.indice(); 
                if colore_pezzo == Colore::Nero {
                    p_idx += 6;
                }

                // APPLICAZIONE PROSPETTIVA (Flip)
                if is_black {
                    // 1. Flip Verticale
                    rank = 7 - rank;
                    // 2. Swap Colore (Il mio pezzo deve finire negli indici 0-5)
                    if p_idx >= 6 {
                        p_idx -= 6; // Nemico (era nero 6-11) diventa Amico (0-5)
                    } else {
                        p_idx += 6; // Amico (era bianco 0-5) diventa Nemico (6-11)
                    }
                }

                // Calcolo feature index (Simple-768: Pezzo * 64 + Casella)
                let feat_sq = rank * 8 + col;
                let feat_idx = p_idx * 64 + feat_sq;

                // Somma i pesi di questa feature all'accumulatore
                // feature_weights è flattened [768 * 256].
                // Poiché in Python hai fatto w1.t(), i dati sono raggruppati per feature.
                // Cioè: primi 256 valori sono i pesi della feature 0 per i 256 neuroni.
                let offset = feat_idx * HIDDEN_SIZE;
                
                // SIMD manuale (loop unrolling parziale dal compilatore)
                for i in 0..HIDDEN_SIZE {
                    accumulator[i] += self.feature_weights[offset + i] as i32;
                }
            }
        }

        // 3. Attivazione + Output Layer
        let mut output_sum: i32 = self.output_bias as i32;

        for i in 0..HIDDEN_SIZE {
            // Attivazione: Nel training era torch.clamp(x, 0, 1.0) con input float.
            // Qui siamo scalati a 255. Quindi clamp(0, 255).
            let val = accumulator[i];
            let activated = val.max(0).min(255); // Clipped ReLU (SCReLU)

            // Moltiplicazione con output weights
            output_sum += activated * (self.output_weights[i] as i32);
        }

        // 4. De-quantizzazione
        // Output finale = sum / SCALE.
        // Poiché abbiamo scalato input e pesi, il valore è molto grande.
        // Training: 1.0 (float) -> 255 (int).
        // Layer1 Out: 255 * W1(255) ~ 65025.
        // Layer2 Out: 65025 * W2(255) ~ 16M.
        // Dobbiamo riportarlo a Centipawns.
        // Di solito si divide per una costante fissa.
        // Proviamo a dividere per SCALE * k.
        // Dividendo per SCALE (255) una volta, otteniamo valori nell'ordine di ~65000 se vittoria.
        // Dividiamo per 255 per tornare alla scala dell'output training.
        
        let eval = output_sum / SCALE; 

        // Ritorna il valore. Nota: NNUE è allenata sul punteggio assoluto (White perspective)
        // o relativo? parse_fen dice: "Chi muove è sempre colore amico".
        // Quindi l'output della rete è GIA' relativo al giocatore che muove.
        eval
    }
}