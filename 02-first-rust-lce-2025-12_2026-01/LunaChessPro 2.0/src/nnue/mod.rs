use crate::board::{Scacchiera, Colore, Pezzo, NNUE_LAYER1_SIZE};
use std::fs::File;
use std::io::{Read, BufReader, Seek, SeekFrom};

const INPUT_SIZE: usize = 768; 
const L1_SIZE: usize = NNUE_LAYER1_SIZE;

#[derive(Clone, Copy)]
pub struct CastlingRights {
    pub bianco_lato_re: bool,
    pub bianco_lato_regina: bool,
    pub nero_lato_re: bool,
    pub nero_lato_regina: bool,
}

pub struct Network {
    pub feature_weights: Vec<i16>,
    pub feature_bias: Vec<i16>,
    pub output_weights: Vec<i16>,
    pub output_bias: i16,
}

impl Network {
    pub fn carica(path: &str) -> Self {
        let file = File::open(path).expect("Errore: File luna_net.nnue non trovato!");
        let mut reader = BufReader::new(file);

        // Salta l'header di Stockfish (solitamente 176 byte)
        reader.seek(SeekFrom::Start(176)).unwrap(); 

        let mut feature_weights = vec![0i16; INPUT_SIZE * L1_SIZE];
        let mut feature_bias = vec![0i16; L1_SIZE];
        let mut output_weights = vec![0i16; L1_SIZE * 2];

        unsafe {
            let ptr_fw = feature_weights.as_mut_ptr() as *mut u8;
            reader.read_exact(std::slice::from_raw_parts_mut(ptr_fw, feature_weights.len() * 2)).unwrap();

            let ptr_fb = feature_bias.as_mut_ptr() as *mut u8;
            reader.read_exact(std::slice::from_raw_parts_mut(ptr_fb, feature_bias.len() * 2)).unwrap();

            let ptr_ow = output_weights.as_mut_ptr() as *mut u8;
            reader.read_exact(std::slice::from_raw_parts_mut(ptr_ow, output_weights.len() * 2)).unwrap();
        }

        let mut ob_buf = [0u8; 2];
        reader.read_exact(&mut ob_buf).unwrap();
        let output_bias = i16::from_le_bytes(ob_buf);

        Network { feature_weights, feature_bias, output_weights, output_bias }
    }

    pub fn valuta(&self, s: &Scacchiera) -> i16 {
        let (acc_us, acc_them) = if s.turno == Colore::Bianco {
            (&s.current_acc[0], &s.current_acc[1])
        } else {
            (&s.current_acc[1], &s.current_acc[0])
        };

        let mut sum: i32 = self.output_bias as i32;
        for i in 0..L1_SIZE {
            let val = acc_us.v[i].clamp(0, 127) as i32;
            sum += val * self.output_weights[i] as i32;
        }
        for i in 0..L1_SIZE {
            let val = acc_them.v[i].clamp(0, 127) as i32;
            sum += val * self.output_weights[L1_SIZE + i] as i32;
        }
        (sum / 600) as i16
    }
}

pub fn get_feature_index(pezzo: Pezzo, colore: Colore, sq: usize) -> usize {
    pezzo.indice() * 64 + colore.indice() * 384 + sq
}