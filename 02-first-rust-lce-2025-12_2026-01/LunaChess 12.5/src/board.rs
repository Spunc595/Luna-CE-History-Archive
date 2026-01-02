use std::fmt;
use crate::attacks::{pawn_attacks, knight_attacks, king_attacks, bishop_attacks, rook_attacks, queen_attacks};
use lazy_static::lazy_static;

lazy_static! {
    static ref ZOBRIST: ZobristKeys = ZobristKeys::new();
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Colore { Bianco = 0, Nero = 1 }
impl Colore {
    pub fn opposto(&self) -> Self { if *self == Colore::Bianco { Colore::Nero } else { Colore::Bianco } }
    pub fn indice(&self) -> usize { *self as usize }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pezzo { Pedone=0, Cavallo=1, Alfiere=2, Torre=3, Regina=4, Re=5 }

impl Pezzo {
    pub fn indice(&self) -> usize { *self as usize } // Usa il cast diretto
    pub fn valore(&self) -> i32 {
        match self { 
            Pezzo::Pedone => 100, 
            Pezzo::Cavallo => 320, 
            Pezzo::Alfiere => 330, 
            Pezzo::Torre => 500, 
            Pezzo::Regina => 900, 
            Pezzo::Re => 20000 
        }
    }
}

pub type Bitboard = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveFlag { None, EnPassant, CastleKingSide, CastleQueenSide, Capture, Promotion, PromotionCapture, DoublePawnPush }
impl MoveFlag { 
    pub fn is_capture(&self) -> bool { matches!(self, MoveFlag::Capture | MoveFlag::PromotionCapture | MoveFlag::EnPassant) } 
    pub fn is_promo(&self) -> bool { matches!(self, MoveFlag::Promotion | MoveFlag::PromotionCapture) }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Casella(pub usize);
impl Casella {
    pub fn indice(&self) -> usize { self.0 }
    pub fn nome(&self) -> String { format!("{}{}", (b'a' + (self.0 % 8) as u8) as char, (b'1' + (self.0 / 8) as u8) as char) }
    pub fn da_nome(nome: &str) -> Option<Self> {
        if nome.len() != 2 { return None; }
        let b = nome.as_bytes();
        let f = b[0].wrapping_sub(b'a'); let r = b[1].wrapping_sub(b'1');
        if f < 8 && r < 8 { Some(Casella((r*8+f) as usize)) } else { None }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Mossa { pub data: u16 }
impl Mossa {
    pub fn new_with_flag(da: Casella, a: Casella, prom: Option<Pezzo>, flag: MoveFlag) -> Self {
        let da_bits = (da.indice() as u16) & 0x3F;
        let a_bits = (a.indice() as u16) & 0x3F;
        let f_bits: u16 = match flag {
            MoveFlag::None=>0, MoveFlag::DoublePawnPush=>1, MoveFlag::CastleKingSide=>2, MoveFlag::CastleQueenSide=>3,
            MoveFlag::Capture=>4, MoveFlag::EnPassant=>5,
            MoveFlag::Promotion | MoveFlag::PromotionCapture => {
                8 + if flag == MoveFlag::PromotionCapture { 4 } else { 0 } + match prom { Some(Pezzo::Cavallo)=>0, Some(Pezzo::Alfiere)=>1, Some(Pezzo::Torre)=>2, Some(Pezzo::Regina)=>3, _=>0 }
            }
        };
        Mossa { data: da_bits | (a_bits << 6) | (f_bits << 12) }
    }
    pub fn da(&self) -> Casella { Casella((self.data & 0x3F) as usize) }
    pub fn a(&self) -> Casella { Casella(((self.data >> 6) & 0x3F) as usize) }
    pub fn flag(&self) -> MoveFlag {
        match (self.data >> 12) & 0xF { 0=>MoveFlag::None, 1=>MoveFlag::DoublePawnPush, 2=>MoveFlag::CastleKingSide, 3=>MoveFlag::CastleQueenSide, 4=>MoveFlag::Capture, 5=>MoveFlag::EnPassant, 8..=11=>MoveFlag::Promotion, 12..=15=>MoveFlag::PromotionCapture, _=>MoveFlag::None }
    }
    pub fn promozione(&self) -> Option<Pezzo> {
        let f = (self.data >> 12) & 0xF; if f < 8 { return None; }
        match f & 3 { 0=>Some(Pezzo::Cavallo), 1=>Some(Pezzo::Alfiere), 2=>Some(Pezzo::Torre), 3=>Some(Pezzo::Regina), _=>None }
    }
    pub fn to_u16(&self) -> u16 { self.data }
    pub fn from_u16(val: u16) -> Option<Self> { if val == 0 { None } else { Some(Mossa { data: val }) } }
}

impl fmt::Display for Mossa {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let p = match self.promozione() { Some(Pezzo::Regina)=>"q", Some(Pezzo::Torre)=>"r", Some(Pezzo::Alfiere)=>"b", Some(Pezzo::Cavallo)=>"n", _=>"" };
        write!(f, "{}{}{}", self.da().nome(), self.a().nome(), p)
    }
}

#[derive(Clone, Copy)]
pub struct GameState { pub en_passant: Option<usize>, pub diritti_arrocco: u8, pub captured_piece: Option<Pezzo>, pub last_move: Mossa, pub hash: u64 }

#[derive(Clone)]
pub struct Scacchiera {
    pub pezzi: [Bitboard; 6], 
    pub colori: [Bitboard; 2], 
    pub turno: Colore, 
    pub en_passant: Option<usize>, 
    pub diritti_arrocco: u8, 
    pub current_hash: u64, 
    pub history: Vec<GameState>
}

impl Scacchiera {
    pub fn nuova() -> Self { Self::da_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap() }

    pub fn da_fen(fen: &str) -> Result<Self, String> {
        let parts: Vec<&str> = fen.split_whitespace().collect();
        let mut pezzi = [0; 6]; let mut colori = [0; 2];
        for (r, row) in parts[0].split('/').enumerate() {
            let rank = 7 - r; let mut file = 0;
            for c in row.chars() {
                if let Some(d) = c.to_digit(10) { file += d as usize; }
                else {
                    let p = match c.to_ascii_lowercase() { 'p'=>Pezzo::Pedone, 'n'=>Pezzo::Cavallo, 'b'=>Pezzo::Alfiere, 'r'=>Pezzo::Torre, 'q'=>Pezzo::Regina, 'k'=>Pezzo::Re, _=> return Err("Pezzo FEN errato".to_string()) };
                    let col = if c.is_uppercase() { Colore::Bianco } else { Colore::Nero };
                    let bit = 1u64 << (rank*8+file);
                    pezzi[p.indice()] |= bit; colori[col.indice()] |= bit; file += 1;
                }
            }
        }
        let turno = if parts[1] == "w" { Colore::Bianco } else { Colore::Nero };
        let mut cast = 0;
        if parts[2].contains('K') { cast |= 1; } if parts[2].contains('Q') { cast |= 2; }
        if parts[2].contains('k') { cast |= 4; } if parts[2].contains('q') { cast |= 8; }
        let ep = if parts[3] == "-" { None } else { Casella::da_nome(parts[3]).map(|c| c.indice()) };
        let mut s = Scacchiera { pezzi, colori, turno, en_passant: ep, diritti_arrocco: cast, current_hash: 0, history: Vec::with_capacity(256) };
        s.current_hash = s.genera_hash_completo();
        Ok(s)
    }

    // --- METODI MANCANTI RIPRISTINATI ---
    pub fn bitboard_colore(&self, c: Colore) -> Bitboard { self.colori[c.indice()] }
    pub fn bitboard_pezzo_colore(&self, p: Pezzo, c: Colore) -> Bitboard { self.pezzi[p.indice()] & self.colori[c.indice()] }
    pub fn numero_mosse(&self) -> usize { self.history.len() }
    pub fn history_mosse_str(&self) -> Vec<String> {
        self.history.iter().filter(|gs| gs.last_move.data != 0).map(|gs| gs.last_move.to_string()).collect()
    }
    pub fn ultima_mossa_giocata(&self) -> Option<Mossa> { self.history.last().map(|gs| gs.last_move) }

    pub fn get_attacks_for_piece(&self, p: Pezzo, sq: usize, occ: Bitboard) -> Bitboard {
        match p {
            Pezzo::Pedone => pawn_attacks(sq, self.get_color_at(sq).unwrap_or(Colore::Bianco)),
            Pezzo::Cavallo => knight_attacks(sq),
            Pezzo::Alfiere => bishop_attacks(sq, occ),
            Pezzo::Torre => rook_attacks(sq, occ),
            Pezzo::Regina => queen_attacks(sq, occ),
            Pezzo::Re => king_attacks(sq),
        }
    }

    pub fn diritti_arrocco_struct(&self) -> CastlingRights {
        CastlingRights {
            bianco_lato_re: (self.diritti_arrocco & 1) != 0, bianco_lato_regina: (self.diritti_arrocco & 2) != 0,
            nero_lato_re: (self.diritti_arrocco & 4) != 0, nero_lato_regina: (self.diritti_arrocco & 8) != 0,
        }
    }
    // ------------------------------------

    pub fn genera_hash_completo(&self) -> u64 {
        let mut h = 0u64;
        for c in 0..2 {
            for p in 0..6 {
                let mut bb = self.pezzi[p] & self.colori[c];
                while bb != 0 { h ^= ZOBRIST.pezzi[c][p][bb.trailing_zeros() as usize]; bb &= bb - 1; }
            }
        }
        if self.turno == Colore::Nero { h ^= ZOBRIST.turno; }
        h ^= ZOBRIST.arrocco[self.diritti_arrocco as usize];
        if let Some(sq) = self.en_passant { h ^= ZOBRIST.en_passant[sq % 8]; }
        h
    }

    pub fn esegui_mossa(&mut self, m: &Mossa) -> bool {
        let us = self.turno;
        let from = m.da().indice();
        let to = m.a().indice();
        let piece = self.get_piece_at(from).unwrap();
        let cap = if m.flag() == MoveFlag::EnPassant { Some(Pezzo::Pedone) } else { self.get_piece_at(to) };

        self.history.push(GameState {
            en_passant: self.en_passant, diritti_arrocco: self.diritti_arrocco,
            captured_piece: cap, last_move: *m, hash: self.current_hash,
        });

        if let Some(sq) = self.en_passant { self.current_hash ^= ZOBRIST.en_passant[sq % 8]; }

        self.toggle_piece(us, piece, from);
        if let Some(cp) = cap {
            let cs = if m.flag() == MoveFlag::EnPassant { if us == Colore::Bianco { to - 8 } else { to + 8 } } else { to };
            self.toggle_piece(us.opposto(), cp, cs);
        }
        let fp = m.promozione().unwrap_or(piece);
        self.toggle_piece(us, fp, to);

        if m.flag() == MoveFlag::CastleKingSide {
            let (rf, rt) = if us == Colore::Bianco { (7, 5) } else { (63, 61) };
            self.toggle_piece(us, Pezzo::Torre, rf); self.toggle_piece(us, Pezzo::Torre, rt);
        } else if m.flag() == MoveFlag::CastleQueenSide {
            let (rf, rt) = if us == Colore::Bianco { (0, 3) } else { (56, 59) };
            self.toggle_piece(us, Pezzo::Torre, rf); self.toggle_piece(us, Pezzo::Torre, rt);
        }

        self.current_hash ^= ZOBRIST.arrocco[self.diritti_arrocco as usize];
        self.diritti_arrocco &= self.get_castle_mask(from) & self.get_castle_mask(to);
        self.current_hash ^= ZOBRIST.arrocco[self.diritti_arrocco as usize];

        self.en_passant = if m.flag() == MoveFlag::DoublePawnPush {
            let ep_sq = if us == Colore::Bianco { from + 8 } else { from - 8 };
            self.current_hash ^= ZOBRIST.en_passant[ep_sq % 8];
            Some(ep_sq)
        } else { None };

        self.turno = us.opposto();
        self.current_hash ^= ZOBRIST.turno;
        true
    }

    pub fn esegui_mossa_nulla(&mut self) {
        self.history.push(GameState {
            en_passant: self.en_passant, diritti_arrocco: self.diritti_arrocco,
            captured_piece: None, last_move: Mossa { data: 0 }, hash: self.current_hash,
        });
        if let Some(sq) = self.en_passant { self.current_hash ^= ZOBRIST.en_passant[sq % 8]; }
        self.en_passant = None;
        self.turno = self.turno.opposto();
        self.current_hash ^= ZOBRIST.turno;
    }

    pub fn annulla_mossa(&mut self) {
        let s = self.history.pop().expect("No moves to undo");
        self.turno = self.turno.opposto();
        self.en_passant = s.en_passant;
        self.diritti_arrocco = s.diritti_arrocco;
        self.current_hash = s.hash;
        if s.last_move.data == 0 { return; } 
        let m = s.last_move;
        let us = self.turno;
        let from = m.da().indice();
        let to = m.a().indice();
        let p_to = self.get_piece_at(to).unwrap();
        let p_from = if m.promozione().is_some() { Pezzo::Pedone } else { p_to };
        self.pezzi[p_to.indice()] &= !(1 << to); self.colori[us.indice()] &= !(1 << to);
        self.pezzi[p_from.indice()] |= 1 << from; self.colori[us.indice()] |= 1 << from;
        if let Some(cp) = s.captured_piece {
            let cs = if m.flag() == MoveFlag::EnPassant { if us == Colore::Bianco { to - 8 } else { to + 8 } } else { to };
            self.pezzi[cp.indice()] |= 1 << cs; self.colori[self.turno.opposto().indice()] |= 1 << cs;
        }
        if m.flag() == MoveFlag::CastleKingSide {
            let (rf, rt) = if us == Colore::Bianco { (7, 5) } else { (63, 61) };
            self.move_rook_raw(rt, rf, us);
        } else if m.flag() == MoveFlag::CastleQueenSide {
            let (rf, rt) = if us == Colore::Bianco { (0, 3) } else { (56, 59) };
            self.move_rook_raw(rt, rf, us);
        }
    }

    pub fn toggle_piece(&mut self, c: Colore, p: Pezzo, s: usize) {
        let b = 1u64 << s;
        self.pezzi[p.indice()] ^= b;
        self.colori[c.indice()] ^= b;
        self.current_hash ^= ZOBRIST.pezzi[c.indice()][p.indice()][s];
    }

    pub fn get_piece_at(&self, sq: usize) -> Option<Pezzo> {
        let bit = 1u64 << sq;
        for p in 0..6 { if (self.pezzi[p] & bit) != 0 { return Some(match p {0=>Pezzo::Pedone, 1=>Pezzo::Cavallo, 2=>Pezzo::Alfiere, 3=>Pezzo::Torre, 4=>Pezzo::Regina, _=>Pezzo::Re}); } }
        None
    }

    pub fn get_color_at(&self, sq: usize) -> Option<Colore> {
        let bit = 1u64 << sq;
        if (self.colori[0] & bit) != 0 { Some(Colore::Bianco) }
        else if (self.colori[1] & bit) != 0 { Some(Colore::Nero) }
        else { None }
    }

    pub fn re_in_scacco(&self, c: Colore) -> bool {
        let k = self.pezzi[Pezzo::Re.indice()] & self.colori[c.indice()];
        if k == 0 { return false; }
        self.casella_attaccata(Casella(k.trailing_zeros() as usize), c.opposto())
    }

    pub fn casella_attaccata(&self, sq: Casella, by: Colore) -> bool {
        let s = sq.indice(); let occ = self.occupazione();
        if (pawn_attacks(s, by.opposto()) & (self.pezzi[0] & self.colori[by.indice()])) != 0 { return true; }
        if (knight_attacks(s) & (self.pezzi[1] & self.colori[by.indice()])) != 0 { return true; }
        if (king_attacks(s) & (self.pezzi[5] & self.colori[by.indice()])) != 0 { return true; }
        let q = self.pezzi[4] & self.colori[by.indice()];
        if (bishop_attacks(s, occ) & (self.pezzi[2] & self.colori[by.indice()] | q)) != 0 { return true; }
        if (rook_attacks(s, occ) & (self.pezzi[3] & self.colori[by.indice()] | q)) != 0 { return true; }
        false
    }

    pub fn ha_pezzi_maggiori(&self, c: Colore) -> bool {
        let occ = self.colori[c.indice()];
        (self.pezzi[1] | self.pezzi[2] | self.pezzi[3] | self.pezzi[4]) & occ != 0
    }

    pub fn occupazione(&self) -> Bitboard { self.colori[0] | self.colori[1] }
    pub fn turno(&self) -> Colore { self.turno }
    pub fn hash(&self) -> u64 { self.current_hash }

    fn move_rook_raw(&mut self, f: usize, t: usize, c: Colore) {
        let mask = !(1 << f); self.pezzi[3] &= mask; self.colori[c.indice()] &= mask;
        let bit = 1 << t; self.pezzi[3] |= bit; self.colori[c.indice()] |= bit;
    }

    fn get_castle_mask(&self, s: usize) -> u8 {
        match s { 4 => !3, 60 => !12, 0 => !2, 7 => !1, 56 => !8, 63 => !4, _ => 0xFF }
    }
}

pub struct CastlingRights { pub bianco_lato_re: bool, pub bianco_lato_regina: bool, pub nero_lato_re: bool, pub nero_lato_regina: bool }

struct ZobristKeys { pezzi: [[[u64; 64]; 6]; 2], turno: u64, arrocco: [u64; 16], en_passant: [u64; 8] }
impl ZobristKeys {
    fn new() -> Self {
        let mut seed = 0x9D2C5680_25F71D35u64;
        let mut next = || { seed = seed.wrapping_mul(47900159953467).wrapping_add(1); seed };
        let mut pezzi = [[[0u64; 64]; 6]; 2];
        for c in 0..2 { for p in 0..6 { for s in 0..64 { pezzi[c][p][s] = next(); } } }
        let mut arrocco = [0u64; 16]; for i in 0..16 { arrocco[i] = next(); }
        let mut en_passant = [0u64; 8]; for i in 0..8 { en_passant[i] = next(); }
        ZobristKeys { pezzi, turno: next(), arrocco, en_passant }
    }
}