use crate::board::{Pezzo, Colore, Casella, DirittiArrocco};

pub struct ZobristHash {
    pezzi: [[[u64; 64]; 6]; 2],    // [colore][pezzo][casella]
    colore_attivo: u64,
    diritti_arrocco: [u64; 4],      // [0: bianco O-O, 1: bianco O-O-O, 2: nero O-O, 3: nero O-O-O]
    en_passant: [u64; 8],           // [file]
}

impl ZobristHash {
    pub fn nuova() -> Self {
        let mut z = ZobristHash {
            pezzi: [[[0; 64]; 6]; 2],
            colore_attivo: 0,
            diritti_arrocco: [0; 4],
            en_passant: [0; 8],
        };
        
        // Inizializza con numeri pseudo-casuali
        z.inizializza();
        z
    }
    
    fn inizializza(&mut self) {
        // Usa un seed fisso per riproducibilità
        let mut rng = fastrand::Rng::with_seed(0x123456789ABCDEF);
        
        // Inizializza pezzi
        for colore in 0..2 {
            for pezzo in 0..6 {
                for casella in 0..64 {
                    self.pezzi[colore][pezzo][casella] = rng.u64(..);
                }
            }
        }
        
        self.colore_attivo = rng.u64(..);
        
        for i in 0..4 {
            self.diritti_arrocco[i] = rng.u64(..);
        }
        
        for i in 0..8 {
            self.en_passant[i] = rng.u64(..);
        }
    }
    
    pub fn hash_pezzo(&self, pezzo: Pezzo, colore: Colore, casella: u8) -> u64 {
        self.pezzi[colore.indice()][pezzo.indice()][casella as usize]
    }
    
    pub fn hash_colore_attivo(&self) -> u64 {
        self.colore_attivo
    }
    
    pub fn hash_diritti_arrocco(&self, diritti: DirittiArrocco) -> u64 {
        let mut hash = 0;
        
        if diritti.bianco_lato_re {
            hash ^= self.diritti_arrocco[0];
        }
        if diritti.bianco_lato_regina {
            hash ^= self.diritti_arrocco[1];
        }
        if diritti.nero_lato_re {
            hash ^= self.diritti_arrocco[2];
        }
        if diritti.nero_lato_regina {
            hash ^= self.diritti_arrocco[3];
        }
        
        hash
    }
    
    pub fn hash_en_passant(&self, file: Option<u8>) -> u64 {
        if let Some(f) = file {
            self.en_passant[f as usize]
        } else {
            0
        }
    }
}