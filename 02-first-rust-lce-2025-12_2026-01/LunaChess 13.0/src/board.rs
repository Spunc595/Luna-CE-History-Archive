use std::fmt;
use crate::attacks::{pawn_attacks, knight_attacks, king_attacks, bishop_attacks, rook_attacks, queen_attacks};

pub type Bitboard = u64;
pub const NNUE_LAYER1_SIZE: usize = 1536;
pub const INFINITO: i16 = 30000;

#[derive(Clone, Copy)]
pub struct NNUEAccumulator { pub v: [i16; NNUE_LAYER1_SIZE] }

// --- LOGICA ZOBRIST ---
lazy_static::lazy_static! {
    static ref ZOBRIST_TABLE: ZobristKeys = ZobristKeys::new();
}

struct ZobristKeys {
    pezzi: [[[u64; 64]; 6]; 2],
    turno: u64,
    arrocco: [u64; 16],
}

impl ZobristKeys {
    fn new() -> Self {
        use rand::{Rng, SeedableRng};
        let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(42);
        let mut pezzi = [[[0u64; 64]; 6]; 2];
        for c in 0..2 { for p in 0..6 { for s in 0..64 { pezzi[c][p][s] = rng.gen(); } } }
        let mut arrocco = [0u64; 16];
        for i in 0..16 { arrocco[i] = rng.gen(); }
        Self { pezzi, turno: rng.gen(), arrocco }
    }
}

// --- ENUM E STRUTTURE BASE ---

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Colore { Bianco = 0, Nero = 1 }
impl Colore {
    pub fn indice(&self) -> usize { *self as usize }
    pub fn opposto(&self) -> Self { match self { Self::Bianco => Self::Nero, Self::Nero => Self::Bianco } }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pezzo { Pedone=0, Cavallo=1, Alfiere=2, Torre=3, Regina=4, Re=5 }
impl Pezzo { pub fn indice(&self) -> usize { *self as usize } }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveFlag { None, DoublePawnPush, Capture, EnPassant, Promotion, Castle }

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Mossa { pub data: u16 }
impl Mossa {
    pub fn new(da: usize, a: usize, flag: MoveFlag) -> Self {
        let f = match flag {
            MoveFlag::None => 0, MoveFlag::DoublePawnPush => 1,
            MoveFlag::Capture => 4, MoveFlag::EnPassant => 5, MoveFlag::Promotion => 6, MoveFlag::Castle => 7,
        };
        Mossa { data: (da as u16) | ((a as u16) << 6) | (f << 12) }
    }
    pub fn da(&self) -> usize { (self.data & 0x3F) as usize }
    pub fn a(&self) -> usize { ((self.data >> 6) & 0x3F) as usize }
    pub fn flag(&self) -> u16 { self.data >> 12 }
    pub fn to_string(&self) -> String {
        let da = self.da(); let a = self.a();
        format!("{}{}{}{}", 
            (b'a' + (da % 8) as u8) as char, (b'1' + (da / 8) as u8) as char,
            (b'a' + (a % 8) as u8) as char, (b'1' + (a / 8) as u8) as char)
    }
}
impl fmt::Display for Mossa { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}", self.to_string()) } }

// --- STRUTTURA SCACCHIERA ---

#[derive(Clone)]
pub struct Scacchiera {
    pub pezzi: [Bitboard; 6],
    pub colori: [Bitboard; 2],
    pub turno: Colore,
    pub hash: u64,
    pub diritti_arrocco: u8,
    pub history: Vec<(u8, Option<Pezzo>, u64)>, // (diritti, cattura, hash)
    pub current_acc: [NNUEAccumulator; 2],
}



impl Scacchiera {
    pub fn nuova() -> Self {
        Self::da_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap()
    }

    pub fn da_fen(fen: &str) -> Result<Self, String> {
        let mut pezzi = [0u64; 6];
        let mut colori = [0u64; 2];
        let parts: Vec<&str> = fen.split_whitespace().collect();
        let ranks: Vec<&str> = parts[0].split('/').collect();

        for (r, rank_str) in ranks.iter().enumerate() {
            let mut file = 0;
            let rank = 7 - r;
            for c in rank_str.chars() {
                if let Some(digit) = c.to_digit(10) { file += digit as usize; }
                else {
                    let color = if c.is_uppercase() { Colore::Bianco } else { Colore::Nero };
                    let p = match c.to_ascii_lowercase() {
                        'p'=>Pezzo::Pedone, 'n'=>Pezzo::Cavallo, 'b'=>Pezzo::Alfiere,
                        'r'=>Pezzo::Torre, 'q'=>Pezzo::Regina, 'k'=>Pezzo::Re, _ => return Err("FEN Error".into())
                    };
                    let sq = rank * 8 + file;
                    pezzi[p.indice()] |= 1 << sq;
                    colori[color.indice()] |= 1 << sq;
                    file += 1;
                }
            }
        }
        let turno = if parts[1] == "w" { Colore::Bianco } else { Colore::Nero };
        
        let mut s = Scacchiera {
            pezzi, colori, turno, hash: 0, diritti_arrocco: 0xF,
            history: Vec::new(), 
            current_acc: [NNUEAccumulator { v: [0; 1536] }; 2],
        };
        s.hash = s.genera_hash_completo();
        Ok(s)
    }

    pub fn genera_hash_completo(&self) -> u64 {
        let mut h = 0;
        for c in 0..2 { for p in 0..6 {
            let mut bb = self.pezzi[p] & self.colori[c];
            while bb != 0 {
                let sq = bb.trailing_zeros() as usize;
                h ^= ZOBRIST_TABLE.pezzi[c][p][sq];
                bb &= bb - 1;
            }
        }}
        if self.turno == Colore::Nero { h ^= ZOBRIST_TABLE.turno; }
        h ^= ZOBRIST_TABLE.arrocco[self.diritti_arrocco as usize];
        h
    }

    pub fn occupazione(&self) -> Bitboard {
        self.colori[0] | self.colori[1]
    }

    pub fn get_piece_at(&self, sq: usize) -> Option<Pezzo> {
        let bit = 1u64 << sq;
        if (self.pezzi[0] & bit) != 0 { return Some(Pezzo::Pedone); }
        if (self.pezzi[1] & bit) != 0 { return Some(Pezzo::Cavallo); }
        if (self.pezzi[2] & bit) != 0 { return Some(Pezzo::Alfiere); }
        if (self.pezzi[3] & bit) != 0 { return Some(Pezzo::Torre); }
        if (self.pezzi[4] & bit) != 0 { return Some(Pezzo::Regina); }
        if (self.pezzi[5] & bit) != 0 { return Some(Pezzo::Re); }
        None
    }

    pub fn esegui_mossa(&mut self, m: &Mossa, _net: &crate::nnue::Network) -> bool {
        let (us, from, to) = (self.turno, m.da(), m.a());
        let piece = self.get_piece_at(from).expect("Nessun pezzo nella casa di partenza");
        let cap = self.get_piece_at(to);

        self.history.push((self.diritti_arrocco, cap, self.hash));

        // Aggiorna Hash Zobrist (Pezzo rimosso da from, messo in to)
        self.hash ^= ZOBRIST_TABLE.pezzi[us.indice()][piece.indice()][from];
        self.hash ^= ZOBRIST_TABLE.pezzi[us.indice()][piece.indice()][to];
        self.hash ^= ZOBRIST_TABLE.arrocco[self.diritti_arrocco as usize];

        // Se è arrocco, muovi anche la torre
        if m.flag() == 7 {
            let (tf, tt) = match to { 
                6 => (7, 5), 2 => (0, 3), 
                62 => (63, 61), 58 => (56, 59), 
                _ => (0, 0) 
            };
            self.toggle_piece(us, Pezzo::Torre, tf);
            self.toggle_piece(us, Pezzo::Torre, tt);
            self.hash ^= ZOBRIST_TABLE.pezzi[us.indice()][Pezzo::Torre.indice()][tf];
            self.hash ^= ZOBRIST_TABLE.pezzi[us.indice()][Pezzo::Torre.indice()][tt];
        }

        self.toggle_piece(us, piece, from);
        if let Some(cp) = cap {
            self.hash ^= ZOBRIST_TABLE.pezzi[us.opposto().indice()][cp.indice()][to];
            self.pezzi[cp.indice()] &= !(1 << to);
            self.colori[us.opposto().indice()] &= !(1 << to);
        }
        self.toggle_piece(us, piece, to);

        self.aggiorna_diritti(from, to);
        self.hash ^= ZOBRIST_TABLE.arrocco[self.diritti_arrocco as usize];
        self.hash ^= ZOBRIST_TABLE.turno;
        self.turno = us.opposto();
        true
    }

    pub fn annulla_mossa(&mut self, m: &Mossa, cap: Option<Pezzo>) {
        let (diritti, _, h_vecchio) = self.history.pop().expect("Nessuna mossa da annullare");
        self.turno = self.turno.opposto();
        let us = self.turno;
        let (from, to) = (m.da(), m.a());
        let piece = self.get_piece_at(to).expect("Nessun pezzo nella casa di arrivo");

        if m.flag() == 7 {
            let (tf, tt) = match to { 
                6 => (7, 5), 2 => (0, 3), 
                62 => (63, 61), 58 => (56, 59), 
                _ => (0, 0) 
            };
            self.toggle_piece(us, Pezzo::Torre, tf); 
            self.toggle_piece(us, Pezzo::Torre, tt);
        }
        self.toggle_piece(us, piece, to);
        if let Some(cp) = cap {
            self.pezzi[cp.indice()] |= (1 << to);
            self.colori[us.opposto().indice()] |= (1 << to);
        }
        self.toggle_piece(us, piece, from);

        self.diritti_arrocco = diritti;
        self.hash = h_vecchio;
    }

    fn aggiorna_diritti(&mut self, from: usize, to: usize) {
        if from == 4 || to == 4 { self.diritti_arrocco &= !3; } // Re Bianco
        if from == 60 || to == 60 { self.diritti_arrocco &= !12; } // Re Nero
        if from == 0 || to == 0 { self.diritti_arrocco &= !2; } // Torre a1
        if from == 7 || to == 7 { self.diritti_arrocco &= !1; } // Torre h1
        if from == 56 || to == 56 { self.diritti_arrocco &= !8; } // Torre a8
        if from == 63 || to == 63 { self.diritti_arrocco &= !4; } // Torre h8
    }

    pub fn toggle_piece(&mut self, c: Colore, p: Pezzo, s: usize) {
        let b = 1 << s;
        self.pezzi[p.indice()] ^= b;
        self.colori[c.indice()] ^= b;
    }

    pub fn re_in_scacco(&self, c: Colore) -> bool {
        let k = self.pezzi[5] & self.colori[c.indice()];
        if k == 0 { return false; }
        self.casella_attaccata(k.trailing_zeros() as usize, c.opposto())
    }

    pub fn casella_attaccata(&self, s: usize, by: Colore) -> bool {
        let occ = self.occupazione();
        if (pawn_attacks(s, by.opposto()) & (self.pezzi[0] & self.colori[by.indice()])) != 0 { return true; }
        if (knight_attacks(s) & (self.pezzi[1] & self.colori[by.indice()])) != 0 { return true; }
        if (king_attacks(s) & (self.pezzi[5] & self.colori[by.indice()])) != 0 { return true; }
        let q = self.pezzi[4] & self.colori[by.indice()];
        if (bishop_attacks(s, occ) & (self.pezzi[2] & self.colori[by.indice()] | q)) != 0 { return true; }
        if (rook_attacks(s, occ) & (self.pezzi[3] & self.colori[by.indice()] | q)) != 0 { return true; }
        false
    }

    pub fn get_attacks_for_piece(&self, p: Pezzo, sq: usize, occ: Bitboard) -> Bitboard {
        match p {
            Pezzo::Cavallo => knight_attacks(sq), 
            Pezzo::Alfiere => bishop_attacks(sq, occ),
            Pezzo::Torre => rook_attacks(sq, occ), 
            Pezzo::Regina => queen_attacks(sq, occ),
            Pezzo::Re => king_attacks(sq), 
            _ => 0
        }
    }

    pub fn inizializza_acc_nnue(&mut self, net: &crate::nnue::Network) {
        self.current_acc[0].v.copy_from_slice(&net.feature_bias);
        self.current_acc[1].v.copy_from_slice(&net.feature_bias);
    }
}