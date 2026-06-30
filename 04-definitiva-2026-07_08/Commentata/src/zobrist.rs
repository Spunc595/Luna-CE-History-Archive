use crate::board::{Scacchiera, Colore};
use std::sync::OnceLock;
use rand::{RngCore, SeedableRng};
use rand_chacha::ChaCha20Rng;

/// Contenitore per i numeri pseudo-casuali a 64 bit utilizzati per calcolare l'hash di Zobrist[cite: 7].
/// Ogni configurazione ha una chiave univoca associata per minimizzare il rischio di collisioni[cite: 7].
#[derive(Clone, Debug)]
pub struct ZobristKeys {
    // Matrice a 3 dimensioni: [Colore (2)][Tipo Pezzo (6)][Casa della scacchiera (64)][cite: 7]
    pub pezzi: [[[u64; 64]; 6]; 2],
    // Chiave XOR per indicare il tratto al Nero[cite: 7]
    pub turno: u64,
    // Chiavi associate alla colonna (file) in cui è attiva la cattura En Passant[cite: 7]
    pub ep_file: [u64; 8],
    // 16 combinazioni possibili per i diritti di arrocco mappati su 4 bit totali[cite: 7]
    pub arrocco_completo: [u64; 16],
}

impl ZobristKeys {
    /// Inizializza le chiavi numeriche utilizzando un seed costante deterministico[cite: 7].
    /// Questo garantisce che ad ogni avvio del motore gli stessi identici stati producano chiavi identiche[cite: 7].
    pub fn init_deterministic() -> Self {
        // Inizializzazione del PRNG crittografico ChaCha20 con seed costante fisso[cite: 7].
        let mut rng = ChaCha20Rng::seed_from_u64(0x123456789ABCDEF0);
        
        // 1. Generazione delle chiavi per la combinazione Colore/Pezzo/Casa[cite: 7]
        let mut pezzi = [[[0u64; 64]; 6]; 2];
        for c in 0..2 {
            for p in 0..6 {
                for sq in 0..64 {
                    pezzi[c][p][sq] = rng.next_u64();
                }
            }
        }
        
        // 2. Generazione della chiave del turno[cite: 7]
        let turno = rng.next_u64();
        
        // 3. Generazione delle chiavi En Passant per colonna (0..8)[cite: 7]
        let mut ep_file = [0u64; 8];
        for i in 0..8 {
            ep_file[i] = rng.next_u64();
        }
        
        // 4. Generazione delle chiavi per i diritti di arrocco complessivi[cite: 7]
        let mut arrocco_completo = [0u64; 16];
        for i in 0..16 {
            arrocco_completo[i] = rng.next_u64();
        }
        
        ZobristKeys {
            pezzi,
            turno,
            ep_file,
            arrocco_completo,
        }
    }

    /// Calcola da zero l'hash completo di una scacchiera scandendo le bitboard correnti[cite: 7].
    /// Sfrutta l'operatore bitwise XOR per comporre la firma a 64 bit in tempo lineare[cite: 7].
    pub fn hash_board(&self, board: &Scacchiera) -> u64 {
        let mut hash = 0u64;

        // Scansione bitwise dei pezzi divisi per colore[cite: 7].
        for c in 0..2 {
            for p in 0..6 {
                // Isola la bitboard specifica intersecando il tipo di pezzo con la maschera del colore[cite: 7].
                let mut bb = board.pezzi[p] & board.colori[c];
                
                // Bit-scanning efficiente tramite rimozione progressiva dell'ultimo bit attivo[cite: 7].
                while bb != 0 {
                    let sq = bb.trailing_zeros() as usize;
                    hash ^= self.pezzi[c][p][sq]; // Applica la chiave tramite XOR[cite: 7]
                    bb &= bb - 1;                 // Sfoltisce il bit elaborato (Bit-pop)[cite: 7]
                }
            }
        }

        // Se il tratto è del Nero, applica la chiave di inversione del turno[cite: 7].
        if board.turno == Colore::Nero { 
            hash ^= self.turno; 
        }

        // Applica i diritti di arrocco usando lo stato grezzo come indice diretto (0..16)[cite: 7].
        hash ^= self.arrocco_completo[board.diritti_arrocco as usize];

        // Se è presente una casa En Passant valida, estrae la colonna (modulo 8) e ne applica la chiave[cite: 7].
        if let Some(sq) = board.ep_square {
            hash ^= self.ep_file[sq % 8];
        }

        hash
    }
}

/// Istanza globale allocata staticamente e protetta da inizializzazione unica lazy (thread-safe)[cite: 7].
static ZOBRIST_KEYS: OnceLock<ZobristKeys> = OnceLock::new();

/// Restituisce un riferimento statico e immutabile alle chiavi Zobrist condivise globalmente[cite: 7].
/// È l'interfaccia principale raccomandata per calcolare gli hash nel motore senza riallocazioni[cite: 7].
pub fn get_zobrist_keys() -> &'static ZobristKeys {
    ZOBRIST_KEYS.get_or_init(|| ZobristKeys::init_deterministic())
}

/// Tratto Default implementato per agganciare in modo nativo la procedura deterministica[cite: 7].
impl Default for ZobristKeys {
    fn default() -> Self {
        ZobristKeys::init_deterministic()
    }
}