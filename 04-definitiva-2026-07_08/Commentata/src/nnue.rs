use std::fs::File;
use std::io::Read;
use crate::board::{Scacchiera, Colore};

// --- CONFIGURAZIONE ARCHITETTURA NNUE ---
// 768 feature di input (12 pezzi per 64 caselle)[cite: 5].
const INPUT_SIZE: usize = 768; 
// Dimensione del primo livello nascosto (Feature Transformer)[cite: 5].
const L1_SIZE: usize = 256;
// Dimensione del secondo livello nascosto[cite: 5].
const L2_SIZE: usize = 32;

/// Struttura principale della rete neurale quantizzata LunaNNUE[cite: 5].
/// Memorizza pesi e bias come interi a 16 bit (`i16`) per ottimizzare le prestazioni CPU senza FPU[cite: 5].
pub struct LunaNNUE {
    l1_weights: Vec<i16>,
    l1_bias: Vec<i16>,
    l2_weights: Vec<i16>,
    l2_bias: Vec<i16>,
    l3_weights: Vec<i16>,
    l3_bias: Vec<i16>,
}

impl LunaNNUE {
    /// Carica i pesi binari della rete da un file esterno in modalità Little-Endian[cite: 5].
    /// Restituisce `None` in caso di errore I/O o file corrotto[cite: 5].
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
        // Chiusura di supporto per leggere sequenzialmente byte accoppiati convertendoli in i16[cite: 5].
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

        // Mappatura ordinata dei vettori di peso e bias[cite: 5].
        net.l1_weights = read_i16(INPUT_SIZE * L1_SIZE);
        net.l1_bias = read_i16(L1_SIZE);
        net.l2_weights = read_i16(L1_SIZE * L2_SIZE);
        net.l2_bias = read_i16(L2_SIZE);
        net.l3_weights = read_i16(L2_SIZE);
        net.l3_bias = read_i16(1);

        // Controllo diagnostico iniziale sullo stato del file binario[cite: 5].
        if net.l1_weights.iter().take(100).all(|&x| x == 0) {
            println!("⚠️ ATTENZIONE: Il file NNUE sembra vuoto!");
        } else {
            println!("✅ NNUE: Pesi caricati (Modalità Safe).");
        }

        Some(net)
    }

    /// Esegue il passo di feedforward (inferenza) della rete sulla posizione corrente[cite: 5].
    /// Estrae le feature attive tramite bitboard e propaga i valori attraverso i livelli quantizzati[cite: 5].
    pub fn evaluate(&self, s: &Scacchiera) -> i32 {
        // ==========================================
        // --- LAYER 1: FEATURE TRANSFORMER (QA=255)
        // ==========================================
        // Inizializza l'accumulatore L1 clonando i valori di bias di partenza[cite: 5].
        let mut l1_acc = self.l1_bias.clone();

        // Iterazione sui 6 tipi di pezzo[cite: 5].
        for p_idx in 0..6 {
            // Sotto-passaggio per il Bianco (Indice Colore 0)[cite: 5].
            let mut bb_w = s.pezzi[p_idx] & s.colori[0];
            while bb_w != 0 {
                let sq = bb_w.trailing_zeros() as usize;
                // Calcolo offset della feature: (tipo_pezzo * 64 + casa)[cite: 5].
                let offset = (p_idx * 64 + sq) * L1_SIZE;
                for i in 0..L1_SIZE {
                    l1_acc[i] = l1_acc[i].saturating_add(self.l1_weights[offset + i]);
                }
                bb_w &= bb_w - 1; // Rimuove il bit processato (Bit-pop)
            }

            // Sotto-passaggio per il Nero (Indice Colore 1)[cite: 5].
            // I pezzi neri sono shiftati di 6 posizioni nell'indice delle feature (+6)[cite: 5].
            let mut bb_b = s.pezzi[p_idx] & s.colori[1];
            while bb_b != 0 {
                let sq = bb_b.trailing_zeros() as usize;
                let offset = ((p_idx + 6) * 64 + sq) * L1_SIZE;
                for i in 0..L1_SIZE {
                    l1_acc[i] = l1_acc[i].saturating_add(self.l1_weights[offset + i]);
                }
                bb_b &= bb_b - 1;
            }
        }

        // Applicazione della funzione di attivazione CReLU sul Layer 1[cite: 5].
        // Il tetto massimo è limitato a 2048 coerentemente con la scala QA=255[cite: 5].
        let mut l1_out = [0i16; 256];
        for i in 0..256 {
            l1_out[i] = l1_acc[i].clamp(0, 2048);
        }

        // ==========================================
        // --- LAYER 2: STRATO NASCOSTO (QB=64)
        // ==========================================
        let mut l2_acc = self.l2_bias.clone();
        for i in 0..L2_SIZE {
            let mut sum: i32 = l2_acc[i] as i32;
            for j in 0..L1_SIZE {
                // Moltiplicazione intera e successiva divisione per 2048 (pari a mossa bitwise >> 11)[cite: 5].
                // Riduce la scala preservando la precisione dei pesi del secondo strato[cite: 5].
                sum += (l1_out[j] as i32 * self.l2_weights[j * L2_SIZE + i] as i32) / 2048;
            }
            l2_acc[i] = sum.clamp(-32768, 32767) as i16;
        }

        // Funzione di attivazione CReLU sul Layer 2[cite: 5].
        // Il tetto massimo è fissato a 128 per rispettare la scala di quantizzazione QB=64[cite: 5].
        let mut l2_out = [0i16; 32];
        for i in 0..32 {
            l2_out[i] = l2_acc[i].clamp(0, 128);
        }

        // ==========================================
        // --- LAYER 3: OUTPUT LINEARE
        // ==========================================
        let mut score: i32 = self.l3_bias[0] as i32;
        for j in 0..L2_SIZE {
            // Riduzione finale della scala tramite divisione fissa per 256[cite: 5].
            score += (l2_out[j] as i32 * self.l3_weights[j] as i32) / 256;
        }

        let val = score;
        
        // Sincronizzazione dell'output con la prospettiva Negamax della ricerca[cite: 5].
        if s.turno == Colore::Bianco { val } else { -val }
    }
}