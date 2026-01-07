use std::fmt;
use crate::attacks::{pawn_attacks, knight_attacks, king_attacks, bishop_attacks, rook_attacks, queen_attacks};

pub type Bitboard = u64;
pub const NNUE_LAYER1_SIZE: usize = 256;

#[derive(Clone, Copy)]
pub struct NNUEAccumulator { pub v: [i16; NNUE_LAYER1_SIZE] }

lazy_static::lazy_static! {
    static ref ZOBRIST_TABLE: ZobristKeys = ZobristKeys::new();
}

struct ZobristKeys {
    pezzi: [[[u64; 64]; 6]; 2],
    turno: u64,
    arrocco: [u64; 16],
    ep_file: [u64; 8],
}

impl ZobristKeys {
    fn new() -> Self {
        use rand::{Rng, SeedableRng};
        let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(42);
        let mut pezzi = [[[0u64; 64]; 6]; 2];
        for c in 0..2 { for p in 0..6 { for s in 0..64 { pezzi[c][p][s] = rng.gen(); } } }
        let mut arrocco = [0u64; 16];
        for i in 0..16 { arrocco[i] = rng.gen(); }
        let mut ep_file = [0u64; 8];
        for i in 0..8 { ep_file[i] = rng.gen(); }
        Self { pezzi, turno: rng.gen(), arrocco, ep_file }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Colore { Bianco = 0, Nero = 1 }
impl Colore {
    pub fn indice(&self) -> usize { *self as usize }
    pub fn opposto(&self) -> Self { match self { Self::Bianco => Self::Nero, Self::Nero => Self::Bianco } }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Pezzo { Pedone=0, Cavallo=1, Alfiere=2, Torre=3, Regina=4, Re=5 }
impl Pezzo { pub fn indice(&self) -> usize { *self as usize } }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
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
    pub fn move_flag(&self) -> MoveFlag {
        match self.flag() {
            1 => MoveFlag::DoublePawnPush,
            4 => MoveFlag::Capture,
            5 => MoveFlag::EnPassant,
            6 => MoveFlag::Promotion,
            7 => MoveFlag::Castle,
            _ => MoveFlag::None
        }
    }
}

impl fmt::Display for Mossa {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let da = self.da(); let a = self.a();
        let mut s = format!("{}{}{}{}", 
            (b'a' + (da % 8) as u8) as char, (b'1' + (da / 8) as u8) as char,
            (b'a' + (a % 8) as u8) as char, (b'1' + (a / 8) as u8) as char);
        if self.flag() == 6 { s.push('q'); }
        write!(f, "{}", s)
    }
}

#[derive(Clone)]
pub struct Scacchiera {
    pub pezzi: [Bitboard; 6],
    pub colori: [Bitboard; 2],
    pub turno: Colore,
    pub hash: u64,
    pub diritti_arrocco: u8,
    pub ep_square: Option<u8>,
    pub history: Vec<(u8, Option<Pezzo>, u64, Option<u8>)>,
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
            let mut file = 0; let rank = 7 - r;
            for c in rank_str.chars() {
                if let Some(digit) = c.to_digit(10) { file += digit as usize; }
                else {
                    let color = if c.is_uppercase() { Colore::Bianco } else { Colore::Nero };
                    let p = match c.to_ascii_lowercase() {
                        'p'=>Pezzo::Pedone, 'n'=>Pezzo::Cavallo, 'b'=>Pezzo::Alfiere,
                        'r'=>Pezzo::Torre, 'q'=>Pezzo::Regina, 'k'=>Pezzo::Re, _ => Pezzo::Pedone
                    };
                    pezzi[p.indice()] |= 1u64 << (rank * 8 + file);
                    colori[color.indice()] |= 1u64 << (rank * 8 + file);
                    file += 1;
                }
            }
        }
        let turno = if parts.len() > 1 && parts[1] == "b" { Colore::Nero } else { Colore::Bianco };
        let mut diritti = 0;
        if parts.len() > 2 {
            if parts[2].contains('K') { diritti |= 1; }
            if parts[2].contains('Q') { diritti |= 2; }
            if parts[2].contains('k') { diritti |= 4; }
            if parts[2].contains('q') { diritti |= 8; }
        }
        let ep_square = if parts.len() > 3 && parts[3] != "-" {
            let s = parts[3];
            let f = s.chars().nth(0).unwrap() as u8 - b'a';
            let r = s.chars().nth(1).unwrap() as u8 - b'1';
            Some(r * 8 + f)
        } else { None };

        let mut s = Scacchiera { 
            pezzi, colori, turno, hash: 0, diritti_arrocco: diritti, ep_square,
            history: Vec::new(), current_acc: [NNUEAccumulator { v: [0; NNUE_LAYER1_SIZE] }; 2] 
        };
        s.hash = s.genera_hash_completo();
        Ok(s)
    }

    pub fn occupazione(&self) -> Bitboard {
        self.colori[0] | self.colori[1]
    }

    pub fn get_attacks_for_piece(&self, p: Pezzo, sq: usize, occ: Bitboard) -> Bitboard {
        match p {
            Pezzo::Cavallo => knight_attacks(sq),
            Pezzo::Alfiere => bishop_attacks(sq, occ),
            Pezzo::Torre => rook_attacks(sq, occ),
            Pezzo::Regina => queen_attacks(sq, occ),
            Pezzo::Re => king_attacks(sq),
            _ => 0,
        }
    }

    pub fn genera_hash_completo(&self) -> u64 {
        let mut h = 0;
        for c in 0..2 { for p in 0..6 {
            let mut bb = self.pezzi[p] & self.colori[c];
            while bb != 0 { 
                h ^= ZOBRIST_TABLE.pezzi[c][p][bb.trailing_zeros() as usize]; 
                bb &= bb - 1; 
            }
        }}
        if self.turno == Colore::Nero { h ^= ZOBRIST_TABLE.turno; }
        h ^= ZOBRIST_TABLE.arrocco[self.diritti_arrocco as usize];
        if let Some(ep) = self.ep_square { h ^= ZOBRIST_TABLE.ep_file[(ep % 8) as usize]; }
        h
    }

    pub fn esegui_mossa(&mut self, m: &Mossa, net: Option<&crate::nnue::Network>) -> bool {
        let us = self.turno;
        let (from, to) = (m.da(), m.a());
        let piece = self.get_piece_at(from).unwrap();
        let flag = m.move_flag();
        let cap = if flag == MoveFlag::EnPassant { Some(Pezzo::Pedone) } else { self.get_piece_at(to) };

        self.history.push((self.diritti_arrocco, cap, self.hash, self.ep_square));

        // Rimuovi pezzo da casa di partenza
        self.toggle_piece(us, piece, from);
        self.hash ^= ZOBRIST_TABLE.pezzi[us.indice()][piece.indice()][from];

        if let Some(cp) = cap {
            let cap_sq = if flag == MoveFlag::EnPassant {
                if us == Colore::Bianco { to - 8 } else { to + 8 }
            } else { to };
            self.pezzi[cp.indice()] &= !(1u64 << cap_sq);
            self.colori[us.opposto().indice()] &= !(1u64 << cap_sq);
            self.hash ^= ZOBRIST_TABLE.pezzi[us.opposto().indice()][cp.indice()][cap_sq];
        }

        if flag == MoveFlag::Castle {
            let (r_from, r_to) = match to {
                6 => (7, 5), 2 => (0, 3), 62 => (63, 61), 58 => (56, 59), _ => (0, 0)
            };
            self.toggle_piece(us, Pezzo::Torre, r_from);
            self.toggle_piece(us, Pezzo::Torre, r_to);
            self.hash ^= ZOBRIST_TABLE.pezzi[us.indice()][Pezzo::Torre.indice()][r_from];
            self.hash ^= ZOBRIST_TABLE.pezzi[us.indice()][Pezzo::Torre.indice()][r_to];
        }

        let final_piece = if flag == MoveFlag::Promotion { Pezzo::Regina } else { piece };
        self.toggle_piece(us, final_piece, to);
        self.hash ^= ZOBRIST_TABLE.pezzi[us.indice()][final_piece.indice()][to];

        self.hash ^= ZOBRIST_TABLE.arrocco[self.diritti_arrocco as usize];
        self.aggiorna_diritti(from, to);
        self.hash ^= ZOBRIST_TABLE.arrocco[self.diritti_arrocco as usize];

        if let Some(ep) = self.ep_square { self.hash ^= ZOBRIST_TABLE.ep_file[(ep % 8) as usize]; }
        if flag == MoveFlag::DoublePawnPush {
            let ep_sq = if us == Colore::Bianco { from + 8 } else { from - 8 };
            self.ep_square = Some(ep_sq as u8);
            self.hash ^= ZOBRIST_TABLE.ep_file[ep_sq % 8];
        } else {
            self.ep_square = None;
        }

        self.hash ^= ZOBRIST_TABLE.turno;
        self.turno = us.opposto();
        true
    }

    pub fn annulla_mossa(&mut self, m: &Mossa, _cap_unused: Option<Pezzo>, _net: Option<&crate::nnue::Network>) {
        if let Some((dir, cap, h, ep_old)) = self.history.pop() {
            let us = self.turno.opposto();
            let (from, to) = (m.da(), m.a());
            let flag = m.move_flag();

            let p_on_board = if flag == MoveFlag::Promotion { Pezzo::Regina } else { self.get_piece_at(to).unwrap() };
            let p_moved = if flag == MoveFlag::Promotion { Pezzo::Pedone } else { p_on_board };

            self.toggle_piece(us, p_on_board, to);

            if flag == MoveFlag::Castle {
                let (r_from, r_to) = match to {
                    6 => (7, 5), 2 => (0, 3), 62 => (63, 61), 58 => (56, 59), _ => (0,0)
                };
                self.toggle_piece(us, Pezzo::Torre, r_to);
                self.toggle_piece(us, Pezzo::Torre, r_from);
            }

            self.toggle_piece(us, p_moved, from);

            if let Some(cp) = cap {
                let cap_sq = if flag == MoveFlag::EnPassant {
                    if us == Colore::Bianco { to - 8 } else { to + 8 }
                } else { to };
                self.pezzi[cp.indice()] |= 1u64 << cap_sq;
                self.colori[us.opposto().indice()] |= 1u64 << cap_sq;
            }

            self.diritti_arrocco = dir;
            self.hash = h;
            self.ep_square = ep_old;
            self.turno = us;
        }
    }

    pub fn genera_mosse(&self) -> Vec<Mossa> { crate::movegen::genera_mosse(self) }

    pub fn get_piece_at(&self, sq: usize) -> Option<Pezzo> {
        let bit = 1u64 << sq;
        for p in 0..6 { if (self.pezzi[p] & bit) != 0 { return Some(match p {
            0 => Pezzo::Pedone, 1 => Pezzo::Cavallo, 2 => Pezzo::Alfiere,
            3 => Pezzo::Torre, 4 => Pezzo::Regina, _ => Pezzo::Re
        }); } }
        None
    }

    pub fn toggle_piece(&mut self, c: Colore, p: Pezzo, s: usize) {
        let b = 1u64 << s;
        self.pezzi[p.indice()] ^= b;
        self.colori[c.indice()] ^= b;
    }

    pub fn re_in_scacco(&self, c: Colore) -> bool {
        let k = self.pezzi[Pezzo::Re.indice()] & self.colori[c.indice()];
        if k == 0 { return false; }
        self.casella_attaccata(k.trailing_zeros() as usize, c.opposto())
    }

    pub fn casella_attaccata(&self, s: usize, by: Colore) -> bool {
        let occ = self.occupazione();
        let enemy = self.colori[by.indice()];
        if (pawn_attacks(s, by.opposto()) & (self.pezzi[Pezzo::Pedone.indice()] & enemy)) != 0 { return true; }
        if (knight_attacks(s) & (self.pezzi[Pezzo::Cavallo.indice()] & enemy)) != 0 { return true; }
        if (king_attacks(s) & (self.pezzi[Pezzo::Re.indice()] & enemy)) != 0 { return true; }
        let q_b = (self.pezzi[Pezzo::Alfiere.indice()] | self.pezzi[Pezzo::Regina.indice()]) & enemy;
        let q_r = (self.pezzi[Pezzo::Torre.indice()] | self.pezzi[Pezzo::Regina.indice()]) & enemy;
        if (bishop_attacks(s, occ) & q_b) != 0 { return true; }
        if (rook_attacks(s, occ) & q_r) != 0 { return true; }
        false
    }

    fn aggiorna_diritti(&mut self, from: usize, to: usize) {
        if from == 4 || to == 4 { self.diritti_arrocco &= !3; }
        if from == 60 || to == 60 { self.diritti_arrocco &= !12; }
        if from == 0 || to == 0 { self.diritti_arrocco &= !2; }
        if from == 7 || to == 7 { self.diritti_arrocco &= !1; }
        if from == 56 || to == 56 { self.diritti_arrocco &= !8; }
        if from == 63 || to == 63 { self.diritti_arrocco &= !4; }
    }
}