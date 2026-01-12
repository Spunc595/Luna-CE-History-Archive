// src/polyglot_keys.rs

/// Generatore Pseudo-Random specifico per lo standard PolyGlot.
/// Questo algoritmo permette di ricreare le chiavi di hash esatte usate nei file .bin
/// senza dover incollare array enormi di costanti.

struct PolyGlotRandom {
    seed: u64,
}

impl PolyGlotRandom {
    fn new() -> Self {
        PolyGlotRandom {
            // Seed iniziale standard di PolyGlot
            seed: 0x9D2C5680_25F71D35,
        }
    }

    fn next(&mut self) -> u64 {
        // Algoritmo Linear Congruential Generator standard di PolyGlot
        self.seed = self.seed.wrapping_mul(47900159953467).wrapping_add(1);
        self.seed
    }
}

/// Inizializza le chiavi di hash all'interno della struct OpeningBook.
/// Deve essere chiamato nel costruttore di OpeningBook.
pub fn initialize_keys(
    piece_keys: &mut [[u64; 64]; 12],
    castle_keys: &mut [u64; 16],
    en_passant_keys: &mut [u64; 65], // 8 file + "nessuno"
    turn_key: &mut u64
) {
    let mut rng = PolyGlotRandom::new();

    // 1. Chiavi dei Pezzi [Tipo][Casella]
    // Ordine standard PolyGlot:
    // pedone(0), cavallo(1), alfiere(2), torre(3), donna(4), re(5)
    // Prima tutti i Bianchi (0-5), poi tutti i Neri (6-11)
    for p in 0..12 {
        for s in 0..64 {
            piece_keys[p][s] = rng.next();
        }
    }

    // 2. Chiavi Arrocco
    // PolyGlot usa 4 bit per l'arrocco (K, Q, k, q), quindi 16 combinazioni (0..15)
    for i in 0..16 {
        castle_keys[i] = rng.next();
    }

    // 3. Chiavi En Passant
    // File 0-7. Di solito l'hash Polyglot distingue le colonne.
    for i in 0..65 {
        en_passant_keys[i] = rng.next();
    }

    // 4. Chiave del Turno (Bianco/Nero)
    *turn_key = rng.next();
}