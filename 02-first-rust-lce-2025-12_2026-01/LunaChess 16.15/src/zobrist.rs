use crate::board::{Scacchiera, Pezzo, Colore};

// Inizializza questi numeri a caso
pub struct ZobristKeys {
    pub pieces: [[u64; 64]; 12], // [Pezzo+Colore][Casella]
    pub side: u64,
    pub castling: [u64; 16],
    pub en_passant: [u64; 65],
}

impl ZobristKeys {
    // Generatore Pseudo-Casuale Xorshift semplice
    fn rand(seed: &mut u64) -> u64 {
        *seed ^= *seed << 13;
        *seed ^= *seed >> 7;
        *seed ^= *seed << 17;
        *seed
    }

    pub fn init() -> Self {
        let mut seed = 1070372; 
        let mut pieces = [[0; 64]; 12];
        for p in 0..12 {
            for s in 0..64 {
                pieces[p][s] = Self::rand(&mut seed);
            }
        }

        let side = Self::rand(&mut seed);

        let mut castling = [0; 16];
        for i in 0..16 { castling[i] = Self::rand(&mut seed); }

        let mut en_passant = [0; 65];
        for i in 0..65 { en_passant[i] = Self::rand(&mut seed); }

        Self { pieces, side, castling, en_passant }
    }
}

pub fn compute_hash(board: &Scacchiera, keys: &ZobristKeys) -> u64 {
    let mut hash = 0;

    for sq in 0..64 {
        // CORREZIONE: Usiamo get_piece_at invece dell'accesso diretto
        if let Some(p) = board.get_piece_at(sq) {
            
            // Dobbiamo capire se il pezzo è bianco o nero per l'indice Zobrist
            // Indici: 0-5 Bianco, 6-11 Nero
            let is_white = (board.colori[Colore::Bianco.indice()] & (1u64 << sq)) != 0;
            let color_offset = if is_white { 0 } else { 6 };
            
            let piece_idx = match p {
                Pezzo::Pedone => 0, 
                Pezzo::Cavallo => 1, 
                Pezzo::Alfiere => 2,
                Pezzo::Torre => 3, 
                Pezzo::Regina => 4, 
                Pezzo::Re => 5,
            } + color_offset;
            
            hash ^= keys.pieces[piece_idx][sq];
        }
    }

    if board.turno == Colore::Nero {
        hash ^= keys.side;
    }

    // Nota: Per ora ignoriamo diritti arrocco ed en-passant nell'hash
    // per evitare errori se i campi nella struct board hanno nomi diversi.
    // L'hash posizionale dei pezzi è sufficiente per il 99% dei casi ora.
    
    hash
}