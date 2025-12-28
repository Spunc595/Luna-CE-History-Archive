use std::fmt;
use crate::attacks::{pawn_attacks, knight_attacks, king_attacks, bishop_attacks, rook_attacks, queen_attacks};
use lazy_static::lazy_static;
use rand::{Rng, SeedableRng};
use rand_pcg::Pcg64;

lazy_static! {
    static ref ZOBRIST: ZobristKeys = ZobristKeys::new();
}

struct ZobristKeys {
    pezzi: [[[u64; 64]; 6]; 2],
    turno: u64,
    arrocco: [u64; 16],
    en_passant: [u64; 65],
}

impl ZobristKeys {
    fn new() -> Self {
        let mut rng = Pcg64::seed_from_u64(12345);
        let mut pezzi = [[[0u64; 64]; 6]; 2];
        for c in 0..2 { for p in 0..6 { for s in 0..64 { pezzi[c][p][s] = rng.gen(); } } }
        let mut arrocco = [0u64; 16];
        for i in 0..16 { arrocco[i] = rng.gen(); }
        let mut en_passant = [0u64; 65];
        for i in 0..65 { en_passant[i] = rng.gen(); }
        ZobristKeys { pezzi, turno: rng.gen(), arrocco, en_passant }
    }
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
    pub fn indice(&self) -> usize { *self as usize }
    pub fn valore(&self) -> i32 {
        match self { Pezzo::Pedone=>100, Pezzo::Cavallo=>320, Pezzo::Alfiere=>330, Pezzo::Torre=>500, Pezzo::Regina=>900, Pezzo::Re=>20000 }
    }
    pub fn da_char(c: char) -> Option<Self> {
        match c.to_ascii_lowercase() { 'p'=>Some(Pezzo::Pedone), 'n'=>Some(Pezzo::Cavallo), 'b'=>Some(Pezzo::Alfiere), 'r'=>Some(Pezzo::Torre), 'q'=>Some(Pezzo::Regina), 'k'=>Some(Pezzo::Re), _=>None }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Casella(pub usize);
impl Casella {
    pub fn indice(&self) -> usize { self.0 }
    pub fn rank(&self) -> u8 { (self.0 / 8) as u8 }
    pub fn da_nome(nome: &str) -> Option<Self> {
        if nome.len() != 2 { return None; }
        let b = nome.as_bytes();
        let f = b[0].wrapping_sub(b'a'); let r = b[1].wrapping_sub(b'1');
        if f < 8 && r < 8 { Some(Casella((r*8+f) as usize)) } else { None }
    }
    pub fn nome(&self) -> String {
        format!("{}{}", (b'a' + (self.0 % 8) as u8) as char, (b'1' + (self.0 / 8) as u8) as char)
    }
}

pub type Bitboard = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveFlag { None, EnPassant, CastleKingSide, CastleQueenSide, Capture, Promotion, PromotionCapture, DoublePawnPush }
impl MoveFlag {
    pub fn is_capture(&self) -> bool { matches!(self, MoveFlag::Capture | MoveFlag::PromotionCapture | MoveFlag::EnPassant) }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Mossa { pub data: u32 }
impl Mossa {
    pub fn new_with_flag(da: Casella, a: Casella, prom: Option<Pezzo>, flag: MoveFlag) -> Self {
        let p_bits = match prom { None=>0, Some(Pezzo::Regina)=>1, Some(Pezzo::Torre)=>2, Some(Pezzo::Alfiere)=>3, Some(Pezzo::Cavallo)=>4, _=>0 };
        let f_bits = match flag { MoveFlag::None=>0, MoveFlag::EnPassant=>1, MoveFlag::CastleKingSide=>2, MoveFlag::CastleQueenSide=>3, MoveFlag::Capture=>4, MoveFlag::Promotion=>5, MoveFlag::PromotionCapture=>6, MoveFlag::DoublePawnPush=>7 };
        Mossa { data: (f_bits << 15) | (p_bits << 12) | ((a.indice() as u32) << 6) | (da.indice() as u32) }
    }
    pub fn da(&self) -> Casella { Casella((self.data & 0x3F) as usize) }
    pub fn a(&self) -> Casella { Casella(((self.data >> 6) & 0x3F) as usize) }
    pub fn promozione(&self) -> Option<Pezzo> { match (self.data >> 12) & 0x7 { 1=>Some(Pezzo::Regina), 2=>Some(Pezzo::Torre), 3=>Some(Pezzo::Alfiere), 4=>Some(Pezzo::Cavallo), _=>None } }
    pub fn flag(&self) -> MoveFlag { match (self.data >> 15) & 0xF { 1=>MoveFlag::EnPassant, 2=>MoveFlag::CastleKingSide, 3=>MoveFlag::CastleQueenSide, 4=>MoveFlag::Capture, 5=>MoveFlag::Promotion, 6=>MoveFlag::PromotionCapture, 7=>MoveFlag::DoublePawnPush, _=>MoveFlag::None } }
    pub fn to_u32(&self) -> u32 { self.data }
    pub fn from_u32(val: u32) -> Option<Self> { Some(Mossa { data: val }) }
    pub fn from_uci(uci: &str) -> Option<Self> {
        if uci.len() < 4 { return None; }
        let from = Casella::da_nome(&uci[0..2])?; let to = Casella::da_nome(&uci[2..4])?;
        let promo = if uci.len() > 4 { Pezzo::da_char(uci.chars().nth(4)?) } else { None };
        Some(Self::new_with_flag(from, to, promo, MoveFlag::None))
    }
}

impl fmt::Display for Mossa {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}{}", self.da().nome(), self.a().nome(), match self.promozione() { Some(Pezzo::Regina)=>"q", Some(Pezzo::Torre)=>"r", Some(Pezzo::Alfiere)=>"b", Some(Pezzo::Cavallo)=>"n", _=>"" })
    }
}

#[derive(Clone, Copy)]
pub struct GameState {
    pub en_passant: Option<usize>,
    pub diritti_arrocco: u8,
    pub captured_piece: Option<Pezzo>,
    pub last_move: Mossa,
    pub hash: u64,
}

#[derive(Clone, Copy)]
pub struct CastlingRights { pub bianco_lato_re: bool, pub bianco_lato_regina: bool, pub nero_lato_re: bool, pub nero_lato_regina: bool }

#[derive(Clone)]
pub struct Scacchiera {
    pub pezzi: [Bitboard; 6],
    pub colori: [Bitboard; 2],
    pub turno: Colore,
    pub en_passant: Option<usize>,
    pub diritti_arrocco: u8,
    pub current_hash: u64,
    pub history: Vec<GameState>,
}

impl Scacchiera {
    pub fn nuova() -> Self { Self::da_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap() }

    pub fn da_fen(fen: &str) -> Result<Self, String> {
        let parts: Vec<&str> = fen.split_whitespace().collect();
        let mut pezzi = [0; 6]; let mut colori = [0; 2];
        let rows: Vec<&str> = parts[0].split('/').collect();
        for (r, row) in rows.iter().enumerate() {
            let rank = 7 - r; let mut file = 0;
            for c in row.chars() {
                if let Some(d) = c.to_digit(10) { file += d as usize; }
                else {
                    let p = Pezzo::da_char(c).ok_or("Pezzo")?;
                    let col = if c.is_uppercase() { Colore::Bianco } else { Colore::Nero };
                    let bit = 1u64 << (rank*8+file);
                    pezzi[p.indice()] |= bit; colori[col.indice()] |= bit;
                    file += 1;
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

    pub fn genera_hash_completo(&self) -> u64 {
        let mut h = 0u64;
        for c in 0..2 {
            for p in 0..6 {
                let mut bb = self.pezzi[p] & self.colori[c];
                while bb != 0 {
                    let sq = bb.trailing_zeros() as usize; h ^= ZOBRIST.pezzi[c][p][sq]; bb &= bb - 1;
                }
            }
        }
        if self.turno == Colore::Nero { h ^= ZOBRIST.turno; }
        h ^= ZOBRIST.arrocco[self.diritti_arrocco as usize];
        h ^= ZOBRIST.en_passant[self.en_passant.unwrap_or(64)];
        h
    }

    #[inline(always)] pub fn hash(&self) -> u64 { self.current_hash }
    #[inline(always)] pub fn turno(&self) -> Colore { self.turno }
    #[inline(always)] pub fn occupazione(&self) -> Bitboard { self.colori[0] | self.colori[1] }
    #[inline(always)] pub fn bitboard_colore(&self, c: Colore) -> Bitboard { self.colori[c.indice()] }
    #[inline(always)] pub fn bitboard_pezzo_colore(&self, p: Pezzo, c: Colore) -> Bitboard { self.pezzi[p.indice()] & self.colori[c.indice()] }
    
    pub fn pezzo_su_casella(&self, casella: Casella) -> Option<(Pezzo, Colore)> {
        let bit = 1u64 << casella.indice();
        if (self.occupazione() & bit) == 0 { return None; }
        let col = if (self.colori[0] & bit) != 0 { Colore::Bianco } else { Colore::Nero };
        for p in 0..6 { if (self.pezzi[p] & bit) != 0 { return Some((match p {0=>Pezzo::Pedone, 1=>Pezzo::Cavallo, 2=>Pezzo::Alfiere, 3=>Pezzo::Torre, 4=>Pezzo::Regina, _=>Pezzo::Re}, col)); } }
        None
    }

    pub fn esegui_mossa(&mut self, mossa: &Mossa) -> bool {
        let us = self.turno; let them = us.opposto();
        let from = mossa.da().indice(); let to = mossa.a().indice();
        let piece = match self.get_piece_at(from, us) { Some(p) => p, None => return false };
        let mut captured = None;
        if mossa.flag().is_capture() {
            if mossa.flag() == MoveFlag::EnPassant { captured = Some(Pezzo::Pedone); }
            else { captured = self.get_piece_at(to, them); }
        }
        self.history.push(GameState { en_passant: self.en_passant, diritti_arrocco: self.diritti_arrocco, captured_piece: captured, last_move: *mossa, hash: self.current_hash });
        
        self.toggle_piece(us, piece, from);
        if let Some(cap_p) = captured {
            let cap_sq = if mossa.flag() == MoveFlag::EnPassant { if us == Colore::Bianco { to - 8 } else { to + 8 } } else { to };
            self.toggle_piece(them, cap_p, cap_sq);
        }
        let final_piece = mossa.promozione().unwrap_or(piece);
        self.toggle_piece(us, final_piece, to);
        if mossa.flag() == MoveFlag::CastleKingSide { let (rf, rt) = if us == Colore::Bianco { (7, 5) } else { (63, 61) }; self.toggle_piece(us, Pezzo::Torre, rf); self.toggle_piece(us, Pezzo::Torre, rt); }
        if mossa.flag() == MoveFlag::CastleQueenSide { let (rf, rt) = if us == Colore::Bianco { (0, 3) } else { (56, 59) }; self.toggle_piece(us, Pezzo::Torre, rf); self.toggle_piece(us, Pezzo::Torre, rt); }
        
        self.current_hash ^= ZOBRIST.arrocco[self.diritti_arrocco as usize];
        if piece == Pezzo::Re { if us == Colore::Bianco { self.diritti_arrocco &= !3; } else { self.diritti_arrocco &= !12; } }
        self.diritti_arrocco &= !self.get_castle_mask(from); self.diritti_arrocco &= !self.get_castle_mask(to);
        self.current_hash ^= ZOBRIST.arrocco[self.diritti_arrocco as usize];
        
        self.current_hash ^= ZOBRIST.en_passant[self.en_passant.unwrap_or(64)];
        self.en_passant = if mossa.flag() == MoveFlag::DoublePawnPush { Some(if us == Colore::Bianco { to - 8 } else { to + 8 }) } else { None };
        self.current_hash ^= ZOBRIST.en_passant[self.en_passant.unwrap_or(64)];
        
        self.turno = them; self.current_hash ^= ZOBRIST.turno;
        true
    }

    pub fn annulla_mossa(&mut self) {
        let state = match self.history.pop() { Some(s) => s, None => return };
        self.turno = self.turno.opposto(); let us = self.turno; let them = us.opposto();
        self.en_passant = state.en_passant; self.diritti_arrocco = state.diritti_arrocco; self.current_hash = state.hash;
        let m = state.last_move; let from = m.da().indice(); let to = m.a().indice();
        let p_on_to = m.promozione().unwrap_or_else(|| self.get_piece_at_no_color(to).unwrap());
        self.pezzi[p_on_to.indice()] &= !(1 << to); self.colori[us.indice()] &= !(1 << to);
        let original_piece = if m.promozione().is_some() { Pezzo::Pedone } else { p_on_to };
        self.pezzi[original_piece.indice()] |= 1 << from; self.colori[us.indice()] |= 1 << from;
        if let Some(cap_p) = state.captured_piece {
            let cap_sq = if m.flag() == MoveFlag::EnPassant { if us == Colore::Bianco { to - 8 } else { to + 8 } } else { to };
            self.pezzi[cap_p.indice()] |= 1 << cap_sq; self.colori[them.indice()] |= 1 << cap_sq;
        }
        if m.flag() == MoveFlag::CastleKingSide { let (rf, rt) = if us == Colore::Bianco { (7, 5) } else { (63, 61) }; self.move_rook_raw(rt, rf, us); }
        if m.flag() == MoveFlag::CastleQueenSide { let (rf, rt) = if us == Colore::Bianco { (0, 3) } else { (56, 59) }; self.move_rook_raw(rt, rf, us); }
    }

    pub fn esegui_mossa_nulla(&mut self) {
        self.history.push(GameState { en_passant: self.en_passant, diritti_arrocco: self.diritti_arrocco, captured_piece: None, last_move: Mossa { data: 0 }, hash: self.current_hash });
        self.current_hash ^= ZOBRIST.en_passant[self.en_passant.unwrap_or(64)]; self.en_passant = None; self.current_hash ^= ZOBRIST.en_passant[64];
        self.turno = self.turno.opposto(); self.current_hash ^= ZOBRIST.turno;
    }

    pub fn annulla_mossa_nulla(&mut self) {
        let state = self.history.pop().unwrap();
        self.turno = self.turno.opposto(); self.en_passant = state.en_passant; self.diritti_arrocco = state.diritti_arrocco; self.current_hash = state.hash;
    }

    fn toggle_piece(&mut self, col: Colore, p: Pezzo, sq: usize) {
        let bit = 1u64 << sq; self.pezzi[p.indice()] ^= bit; self.colori[col.indice()] ^= bit;
        self.current_hash ^= ZOBRIST.pezzi[col.indice()][p.indice()][sq];
    }
    fn get_piece_at(&self, sq: usize, col: Colore) -> Option<Pezzo> {
        let bit = 1u64 << sq; if (self.colori[col.indice()] & bit) == 0 { return None; }
        for p in 0..6 { if (self.pezzi[p] & bit) != 0 { return Some(match p {0=>Pezzo::Pedone, 1=>Pezzo::Cavallo, 2=>Pezzo::Alfiere, 3=>Pezzo::Torre, 4=>Pezzo::Regina, _=>Pezzo::Re}); } }
        None
    }
    fn get_piece_at_no_color(&self, sq: usize) -> Option<Pezzo> {
        let bit = 1u64 << sq; for p in 0..6 { if (self.pezzi[p] & bit) != 0 { return Some(match p {0=>Pezzo::Pedone, 1=>Pezzo::Cavallo, 2=>Pezzo::Alfiere, 3=>Pezzo::Torre, 4=>Pezzo::Regina, _=>Pezzo::Re}); } }
        None
    }
    fn move_rook_raw(&mut self, from: usize, to: usize, c: Colore) {
        let f = 1u64 << from; let t = 1u64 << to;
        self.pezzi[Pezzo::Torre.indice()] &= !f; self.pezzi[Pezzo::Torre.indice()] |= t;
        self.colori[c.indice()] &= !f; self.colori[c.indice()] |= t;
    }
    fn get_castle_mask(&self, sq: usize) -> u8 { match sq { 0=>2, 7=>1, 56=>8, 63=>4, 4=>3, 60=>12, _=>0 } }
    pub fn re_in_scacco(&self, col: Colore) -> bool {
        let k = self.bitboard_pezzo_colore(Pezzo::Re, col); if k == 0 { return true; }
        self.casella_attaccata(Casella(k.trailing_zeros() as usize), col.opposto())
    }
    pub fn casella_attaccata(&self, sq: Casella, da_colore: Colore) -> bool {
        let s = sq.indice(); let occ = self.occupazione();
        if (pawn_attacks(s, da_colore.opposto()) & self.bitboard_pezzo_colore(Pezzo::Pedone, da_colore)) != 0 { return true; }
        if (knight_attacks(s) & self.bitboard_pezzo_colore(Pezzo::Cavallo, da_colore)) != 0 { return true; }
        if (king_attacks(s) & self.bitboard_pezzo_colore(Pezzo::Re, da_colore)) != 0 { return true; }
        let sliders = self.bitboard_pezzo_colore(Pezzo::Regina, da_colore);
        if (bishop_attacks(s, occ) & (self.bitboard_pezzo_colore(Pezzo::Alfiere, da_colore) | sliders)) != 0 { return true; }
        if (rook_attacks(s, occ) & (self.bitboard_pezzo_colore(Pezzo::Torre, da_colore) | sliders)) != 0 { return true; }
        false
    }
    pub fn ha_pezzi_maggiori(&self, col: Colore) -> bool { (self.colori[col.indice()] & !self.pezzi[Pezzo::Pedone.indice()] & !self.pezzi[Pezzo::Re.indice()]) != 0 }
    pub fn diritti_arrocco_struct(&self) -> CastlingRights { CastlingRights { bianco_lato_re: (self.diritti_arrocco&1)!=0, bianco_lato_regina: (self.diritti_arrocco&2)!=0, nero_lato_re: (self.diritti_arrocco&4)!=0, nero_lato_regina: (self.diritti_arrocco&8)!=0 } }
}