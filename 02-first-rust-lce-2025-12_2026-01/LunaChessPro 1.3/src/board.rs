use std::fmt;
use crate::attacks::{pawn_attacks, knight_attacks, king_attacks, bishop_attacks, rook_attacks};
use lazy_static::lazy_static;

lazy_static! {
    static ref ZOBRIST: ZobristKeys = ZobristKeys::new();
}

struct ZobristKeys {
    pezzi: [[[u64; 64]; 6]; 2],
    turno: u64,
    arrocco: [u64; 16],
    en_passant: [u64; 65],
}

struct PolyGlotRng { seed: u64 }
impl PolyGlotRng {
    fn new() -> Self { PolyGlotRng { seed: 0x9D2C5680_25F71D35 } }
    fn next(&mut self) -> u64 {
        self.seed = self.seed.wrapping_mul(47900159953467).wrapping_add(1);
        self.seed
    }
}

impl ZobristKeys {
    fn new() -> Self {
        let mut rng = PolyGlotRng::new();
        let mut pezzi = [[[0u64; 64]; 6]; 2];
        for p_idx in 0..12 {
            let c = if p_idx < 6 { 0 } else { 1 };
            let p = p_idx % 6;
            for s in 0..64 { pezzi[c][p][s] = rng.next(); }
        }
        let mut arrocco = [0u64; 16];
        for i in 0..16 { arrocco[i] = rng.next(); }
        let mut en_passant = [0u64; 65];
        for i in 0..65 { en_passant[i] = rng.next(); }
        let turno = rng.next();
        ZobristKeys { pezzi, turno, arrocco, en_passant }
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
    pub fn nome(&self) -> String { format!("{}{}", (b'a' + (self.0 % 8) as u8) as char, (b'1' + (self.0 / 8) as u8) as char) }
    pub fn da_nome(nome: &str) -> Option<Self> {
        if nome.len() != 2 { return None; }
        let b = nome.as_bytes();
        let f = b[0].wrapping_sub(b'a'); let r = b[1].wrapping_sub(b'1');
        if f < 8 && r < 8 { Some(Casella((r*8+f) as usize)) } else { None }
    }
}

pub type Bitboard = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveFlag { None, EnPassant, CastleKingSide, CastleQueenSide, Capture, Promotion, PromotionCapture, DoublePawnPush }
impl MoveFlag { pub fn is_capture(&self) -> bool { matches!(self, MoveFlag::Capture | MoveFlag::PromotionCapture | MoveFlag::EnPassant) } }

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
                let b = if flag == MoveFlag::PromotionCapture { 12 } else { 8 };
                b + match prom { Some(Pezzo::Cavallo)=>0, Some(Pezzo::Alfiere)=>1, Some(Pezzo::Torre)=>2, Some(Pezzo::Regina)=>3, _=>0 }
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
    pub pezzi: [Bitboard; 6], pub colori: [Bitboard; 2], pub turno: Colore, pub en_passant: Option<usize>, pub diritti_arrocco: u8, pub current_hash: u64, pub history: Vec<GameState>
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
                    let p = Pezzo::da_char(c).ok_or("FEN Error")?;
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

    fn genera_hash_completo(&self) -> u64 {
        let mut h = 0u64;
        for c in 0..2 {
            for p in 0..6 {
                let mut bb = self.pezzi[p] & self.colori[c];
                while bb != 0 { h ^= ZOBRIST.pezzi[c][p][bb.trailing_zeros() as usize]; bb &= bb - 1; }
            }
        }
        if self.turno == Colore::Nero { h ^= ZOBRIST.turno; }
        h ^= ZOBRIST.arrocco[self.diritti_arrocco as usize];
        h ^= ZOBRIST.en_passant[self.en_passant.unwrap_or(64)];
        h
    }

    #[inline(always)] pub fn hash(&self) -> u64 { self.current_hash }
    #[inline(always)] pub fn occupazione(&self) -> Bitboard { self.colori[0] | self.colori[1] }
    #[inline(always)] pub fn bitboard_pezzo_colore(&self, p: Pezzo, c: Colore) -> Bitboard { self.pezzi[p.indice()] & self.colori[c.indice()] }
    #[inline(always)] pub fn bitboard_colore(&self, c: Colore) -> Bitboard { self.colori[c.indice()] }
    #[inline(always)] pub fn turno(&self) -> Colore { self.turno }

    pub fn pezzo_su_casella(&self, casella: Casella) -> Option<(Pezzo, Colore)> {
        let bit = 1u64 << casella.indice();
        if (self.occupazione() & bit) == 0 { return None; }
        let col = if (self.colori[0] & bit) != 0 { Colore::Bianco } else { Colore::Nero };
        for p in [Pezzo::Pedone, Pezzo::Cavallo, Pezzo::Alfiere, Pezzo::Torre, Pezzo::Regina, Pezzo::Re] {
            if (self.pezzi[p.indice()] & bit) != 0 { return Some((p, col)); }
        }
        None
    }

    // --- HELPER PER SEE (CORRETTO con '& occ') ---
    pub fn get_least_valuable_attacker(&self, sq: usize, by: Colore, occ: Bitboard) -> Option<(usize, Pezzo)> {
        // Pedone (Controlliamo solo i pedoni presenti in 'occ')
        let pawns = self.bitboard_pezzo_colore(Pezzo::Pedone, by) & occ; 
        let pawn_attacks = pawn_attacks(sq, by.opposto());
        if (pawns & pawn_attacks) != 0 {
            let from = (pawns & pawn_attacks).trailing_zeros() as usize;
            return Some((from, Pezzo::Pedone));
        }
        // Cavallo
        let knights = self.bitboard_pezzo_colore(Pezzo::Cavallo, by) & occ;
        let k_attacks = knight_attacks(sq);
        if (knights & k_attacks) != 0 {
             let from = (knights & k_attacks).trailing_zeros() as usize;
             return Some((from, Pezzo::Cavallo));
        }
        // Alfiere
        let bishops = self.bitboard_pezzo_colore(Pezzo::Alfiere, by) & occ;
        let b_attacks = bishop_attacks(sq, occ);
        if (bishops & b_attacks) != 0 {
             let from = (bishops & b_attacks).trailing_zeros() as usize;
             return Some((from, Pezzo::Alfiere));
        }
        // Torre
        let rooks = self.bitboard_pezzo_colore(Pezzo::Torre, by) & occ;
        let r_attacks = rook_attacks(sq, occ);
        if (rooks & r_attacks) != 0 {
             let from = (rooks & r_attacks).trailing_zeros() as usize;
             return Some((from, Pezzo::Torre));
        }
        // Regina
        let queens = self.bitboard_pezzo_colore(Pezzo::Regina, by) & occ;
        let q_attacks = bishop_attacks(sq, occ) | rook_attacks(sq, occ);
        if (queens & q_attacks) != 0 {
             let from = (queens & q_attacks).trailing_zeros() as usize;
             return Some((from, Pezzo::Regina));
        }
        // Re
        let king = self.bitboard_pezzo_colore(Pezzo::Re, by) & occ;
        let king_att = king_attacks(sq);
        if (king & king_att) != 0 {
             let from = (king & king_att).trailing_zeros() as usize;
             return Some((from, Pezzo::Re));
        }

        None
    }

    pub fn esegui_mossa(&mut self, m: &Mossa) -> bool {
        let us = self.turno; let them = us.opposto(); let from = m.da().indice(); let to = m.a().indice();
        let piece = match self.get_piece_at_no_color(from) { Some(p)=>p, None=>return false };
        let cap = if m.flag().is_capture() { if m.flag()==MoveFlag::EnPassant {Some(Pezzo::Pedone)} else {self.get_piece_at_no_color(to)} } else {None};

        self.history.push(GameState { en_passant: self.en_passant, diritti_arrocco: self.diritti_arrocco, captured_piece: cap, last_move: *m, hash: self.current_hash });
        self.toggle_piece(us, piece, from);
        if let Some(cp) = cap {
            let cs = if m.flag() == MoveFlag::EnPassant { if us == Colore::Bianco { to - 8 } else { to + 8 } } else { to };
            self.toggle_piece(them, cp, cs);
        }
        let fp = m.promozione().unwrap_or(piece);
        self.toggle_piece(us, fp, to);
        if m.flag() == MoveFlag::CastleKingSide { let (rf, rt) = if us==Colore::Bianco {(7,5)} else {(63,61)}; self.toggle_piece(us, Pezzo::Torre, rf); self.toggle_piece(us, Pezzo::Torre, rt); }
        else if m.flag() == MoveFlag::CastleQueenSide { let (rf, rt) = if us==Colore::Bianco {(0,3)} else {(56,59)}; self.toggle_piece(us, Pezzo::Torre, rf); self.toggle_piece(us, Pezzo::Torre, rt); }

        self.current_hash ^= ZOBRIST.arrocco[self.diritti_arrocco as usize];
        if piece == Pezzo::Re { if us == Colore::Bianco { self.diritti_arrocco &= !3; } else { self.diritti_arrocco &= !12; } }
        self.diritti_arrocco &= !self.get_castle_mask(from); self.diritti_arrocco &= !self.get_castle_mask(to);
        self.current_hash ^= ZOBRIST.arrocco[self.diritti_arrocco as usize];
        self.current_hash ^= ZOBRIST.en_passant[self.en_passant.unwrap_or(64)];
        self.en_passant = if m.flag() == MoveFlag::DoublePawnPush { Some(if us == Colore::Bianco { to - 8 } else { to + 8 }) } else { None };
        self.current_hash ^= ZOBRIST.en_passant[self.en_passant.unwrap_or(64)];
        self.turno = them; self.current_hash ^= ZOBRIST.turno;
        true
    }

    pub fn annulla_mossa(&mut self) {
        let s = self.history.pop().expect("No history");
        self.turno = self.turno.opposto(); self.en_passant = s.en_passant; self.diritti_arrocco = s.diritti_arrocco; self.current_hash = s.hash;
        let m = s.last_move; let us = self.turno; let from = m.da().indice(); let to = m.a().indice();
        let mp = if m.promozione().is_some() { Pezzo::Pedone } else { self.get_piece_at_no_color(to).unwrap() };
        let pot = m.promozione().unwrap_or(mp);
        self.pezzi[pot.indice()] &= !(1 << to); self.colori[us.indice()] &= !(1 << to);
        self.pezzi[mp.indice()] |= 1 << from; self.colori[us.indice()] |= 1 << from;
        if let Some(cp) = s.captured_piece {
            let cs = if m.flag() == MoveFlag::EnPassant { if us == Colore::Bianco { to - 8 } else { to + 8 } } else { to };
            self.pezzi[cp.indice()] |= 1 << cs; self.colori[self.turno.opposto().indice()] |= 1 << cs;
        }
        if m.flag() == MoveFlag::CastleKingSide { let (rf, rt) = if us==Colore::Bianco {(7,5)} else {(63,61)}; self.move_rook_raw(rt, rf, us); }
        else if m.flag() == MoveFlag::CastleQueenSide { let (rf, rt) = if us==Colore::Bianco {(0,3)} else {(56,59)}; self.move_rook_raw(rt, rf, us); }
    }

    pub fn esegui_mossa_nulla(&mut self) {
        self.history.push(GameState { en_passant: self.en_passant, diritti_arrocco: self.diritti_arrocco, captured_piece: None, last_move: Mossa { data: 0 }, hash: self.current_hash });
        self.en_passant = None; self.turno = self.turno.opposto(); self.current_hash ^= ZOBRIST.turno;
    }
    pub fn annulla_mossa_nulla(&mut self) { let s = self.history.pop().unwrap(); self.turno = self.turno.opposto(); self.en_passant = s.en_passant; self.current_hash = s.hash; }

    fn toggle_piece(&mut self, c: Colore, p: Pezzo, s: usize) { let b = 1u64 << s; self.pezzi[p.indice()] ^= b; self.colori[c.indice()] ^= b; self.current_hash ^= ZOBRIST.pezzi[c.indice()][p.indice()][s]; }
    fn get_piece_at_no_color(&self, s: usize) -> Option<Pezzo> { let b = 1u64 << s; for p in 0..6 { if (self.pezzi[p] & b) != 0 { return Some(match p {0=>Pezzo::Pedone, 1=>Pezzo::Cavallo, 2=>Pezzo::Alfiere, 3=>Pezzo::Torre, 4=>Pezzo::Regina, _=>Pezzo::Re}); } } None }
    fn move_rook_raw(&mut self, f: usize, t: usize, c: Colore) { self.pezzi[3] &= !(1<<f); self.pezzi[3] |= 1<<t; self.colori[c.indice()] &= !(1<<f); self.colori[c.indice()] |= 1<<t; }
    fn get_castle_mask(&self, s: usize) -> u8 { match s { 0=>2, 7=>1, 56=>8, 63=>4, 4=>3, 60=>12, _=>0 } }
    pub fn re_in_scacco(&self, c: Colore) -> bool { let k = self.bitboard_pezzo_colore(Pezzo::Re, c); if k == 0 { return true; } self.casella_attaccata(Casella(k.trailing_zeros() as usize), c.opposto()) }
    pub fn casella_attaccata(&self, sq: Casella, by: Colore) -> bool {
        let s = sq.indice(); let occ = self.occupazione();
        if (pawn_attacks(s, by.opposto()) & self.bitboard_pezzo_colore(Pezzo::Pedone, by)) != 0 { return true; }
        if (knight_attacks(s) & self.bitboard_pezzo_colore(Pezzo::Cavallo, by)) != 0 { return true; }
        if (king_attacks(s) & self.bitboard_pezzo_colore(Pezzo::Re, by)) != 0 { return true; }
        let sl = self.bitboard_pezzo_colore(Pezzo::Regina, by);
        if (bishop_attacks(s, occ) & (self.bitboard_pezzo_colore(Pezzo::Alfiere, by) | sl)) != 0 { return true; }
        if (rook_attacks(s, occ) & (self.bitboard_pezzo_colore(Pezzo::Torre, by) | sl)) != 0 { return true; }
        false
    }
    pub fn ha_pezzi_maggiori(&self, c: Colore) -> bool { (self.colori[c.indice()] & !self.pezzi[0] & !self.pezzi[5]) != 0 }
    pub fn può_arroccare(&self, c: Colore, r: bool) -> bool { match c { Colore::Bianco => if r { (self.diritti_arrocco & 1) != 0 } else { (self.diritti_arrocco & 2) != 0 }, Colore::Nero => if r { (self.diritti_arrocco & 4) != 0 } else { (self.diritti_arrocco & 8) != 0 } } }
    pub fn diritti_arrocco_struct(&self) -> crate::board::CastlingRights { crate::board::CastlingRights { bianco_lato_re: (self.diritti_arrocco & 1)!=0, bianco_lato_regina: (self.diritti_arrocco & 2)!=0, nero_lato_re: (self.diritti_arrocco & 4)!=0, nero_lato_regina: (self.diritti_arrocco & 8)!=0 } }
}

#[derive(Clone, Copy)] pub struct CastlingRights { pub bianco_lato_re: bool, pub bianco_lato_regina: bool, pub nero_lato_re: bool, pub nero_lato_regina: bool }