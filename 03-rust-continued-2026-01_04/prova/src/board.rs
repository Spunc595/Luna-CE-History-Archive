use std::fmt;
use crate::zobrist::ZobristKeys;
use crate::nnue::{LunaNNUE, Accumulator};

pub type Bitboard = u64;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Colore { Bianco = 0, Nero = 1 }

impl Colore {
    #[inline(always)] pub fn opposto(&self) -> Colore { match self { Colore::Bianco => Colore::Nero, Colore::Nero => Colore::Bianco } }
    #[inline(always)] pub fn indice(&self) -> usize { *self as usize }
    pub fn from_index(i: usize) -> Self { if i == 0 { Colore::Bianco } else { Colore::Nero } }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Pezzo { Pedone = 0, Cavallo = 1, Alfiere = 2, Torre = 3, Regina = 4, Re = 5 }

impl Pezzo {
    #[inline(always)] pub fn indice(&self) -> usize { *self as usize }
    #[inline(always)] pub fn valore(&self) -> i32 {
        match self { Pezzo::Pedone => 100, Pezzo::Cavallo => 320, Pezzo::Alfiere => 330, Pezzo::Torre => 500, Pezzo::Regina => 900, Pezzo::Re => 20000 }
    }
    pub fn from_index(i: usize) -> Pezzo {
        match i { 0 => Pezzo::Pedone, 1 => Pezzo::Cavallo, 2 => Pezzo::Alfiere, 3 => Pezzo::Torre, 4 => Pezzo::Regina, 5 => Pezzo::Re, _ => Pezzo::Pedone }
    }
    pub fn from_char(c: char) -> Option<(Colore, Pezzo)> {
        match c {
            'P' => Some((Colore::Bianco, Pezzo::Pedone)), 'N' => Some((Colore::Bianco, Pezzo::Cavallo)), 'B' => Some((Colore::Bianco, Pezzo::Alfiere)),
            'R' => Some((Colore::Bianco, Pezzo::Torre)), 'Q' => Some((Colore::Bianco, Pezzo::Regina)), 'K' => Some((Colore::Bianco, Pezzo::Re)),
            'p' => Some((Colore::Nero, Pezzo::Pedone)), 'n' => Some((Colore::Nero, Pezzo::Cavallo)), 'b' => Some((Colore::Nero, Pezzo::Alfiere)),
            'r' => Some((Colore::Nero, Pezzo::Torre)), 'q' => Some((Colore::Nero, Pezzo::Regina)), 'k' => Some((Colore::Nero, Pezzo::Re)), _ => None
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum MoveFlag { None = 0, EnPassant = 1, Castle = 2, Promotion = 3, Capture = 4, DoublePawnPush = 5, PromotionCapture = 6 }

impl MoveFlag { 
    #[inline(always)] pub fn is_capture(&self) -> bool { matches!(self, MoveFlag::Capture | MoveFlag::EnPassant | MoveFlag::PromotionCapture) } 
    #[inline(always)] pub fn is_promotion(&self) -> bool { matches!(self, MoveFlag::Promotion | MoveFlag::PromotionCapture) }
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Mossa { pub data: u16, pub promozione: u8 }

impl Mossa {
    pub fn new(from: usize, to: usize, flag: MoveFlag, promo: Option<Pezzo>) -> Self {
        let pv = promo.map(|p| p.indice() as u8).unwrap_or(6);
        Mossa { data: (from as u16) | ((to as u16) << 6) | ((flag as u16) << 12), promozione: pv }
    }
    #[inline(always)] pub fn da(&self) -> usize { (self.data & 0x3F) as usize }
    #[inline(always)] pub fn a(&self) -> usize { ((self.data >> 6) & 0x3F) as usize }
    #[inline(always)] pub fn move_flag(&self) -> MoveFlag { unsafe { std::mem::transmute((self.data >> 12) as u8) } }
    #[inline(always)] pub fn is_cattura(&self) -> bool { self.move_flag().is_capture() }
    #[inline(always)] pub fn is_promozione(&self) -> bool { self.move_flag().is_promotion() }
    pub fn pezzo_promosso(&self) -> Option<Pezzo> { if self.promozione < 6 { Some(Pezzo::from_index(self.promozione as usize)) } else { None } }
    pub fn to_uci(&self) -> String {
        let from = self.da(); let to = self.a();
        let mut s = format!("{}{}{}{}", (b'a'+(from%8)as u8)as char, (b'1'+(from/8)as u8)as char, (b'a'+(to%8)as u8)as char, (b'1'+(to/8)as u8)as char);
        if let Some(p) = self.pezzo_promosso() { s.push(match p { Pezzo::Cavallo => 'n', Pezzo::Alfiere => 'b', Pezzo::Torre => 'r', _ => 'q' }); }
        s
    }
    pub fn null() -> Self { Mossa { data: 0, promozione: 6 } }
    pub fn is_null(&self) -> bool { self.data == 0 }
    pub fn from_data(data: u16) -> Self { Mossa { data, promozione: 6 } }
}

#[derive(Clone, Debug)]
pub struct UndoData { pub hash: u64, pub ep_square: Option<usize>, pub diritti_arrocco: u8, pub mezze_mosse: u32, pub cattura_p: Option<usize>, pub accumulator: Accumulator }

const CR_UPDATE: [u8; 64] = [13, 15, 15, 15, 12, 15, 15, 14, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 7, 15, 15, 15, 3, 15, 15, 11];

#[derive(Clone, Debug)]
pub struct Scacchiera {
    pub pezzi: [Bitboard; 6], pub colori: [Bitboard; 2], pub turno: Colore, pub ep_square: Option<usize>,
    pub diritti_arrocco: u8, pub hash: u64, pub mezze_mosse: u32, pub history: Vec<UndoData>,
    pub ply: u32, pub rule_50: u32, pub accumulator: Accumulator,
}

impl Scacchiera {
    pub fn from_fen(fen: &str, z: &ZobristKeys) -> Self {
        let mut p_bb = [0; 6]; let mut c_bb = [0; 2];
        let parts: Vec<&str> = fen.split_whitespace().collect();
        let (mut r, mut f) = (7i32, 0usize);
        for c in parts[0].chars() {
            if c == '/' { r -= 1; f = 0; }
            else if let Some(d) = c.to_digit(10) { f += d as usize; }
            else if let Some((col, p)) = Pezzo::from_char(c) {
                let sq = (r * 8) as usize + f;
                p_bb[p.indice()] |= 1 << sq; c_bb[col.indice()] |= 1 << sq; f += 1;
            }
        }
        let t = if parts.len() > 1 && parts[1] == "b" { Colore::Nero } else { Colore::Bianco };
        let mut board = Scacchiera { pezzi: p_bb, colori: c_bb, turno: t, ep_square: None, diritti_arrocco: 15, hash: 0, mezze_mosse: 0, history: Vec::with_capacity(128), ply: 0, rule_50: 0, accumulator: Accumulator::default() };
        board.hash = board.get_hash(z); board
    }

    pub fn new_iniziale(z: &ZobristKeys) -> Self { Self::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", z) }
    pub fn fase_gioco(&self) -> i32 { (self.pezzi[1].count_ones() + self.pezzi[2].count_ones() + self.pezzi[3].count_ones() + self.pezzi[4].count_ones()) as i32 }
    pub fn check_accumulator(&mut self, n: Option<&LunaNNUE>) { if let Some(net) = n { self.accumulator = net.refresh_accumulator(self); } }
    #[inline(always)] pub fn occupazione(&self) -> Bitboard { self.colori[0] | self.colori[1] }
    
    pub fn get_hash(&self, z: &ZobristKeys) -> u64 {
        let mut h = 0;
        for c in 0..2 { for p in 0..6 {
            let mut bb = self.pezzi[p] & self.colori[c];
            while bb != 0 { h ^= z.pezzi[c][p][bb.trailing_zeros() as usize]; bb &= bb - 1; }
        } }
        if self.turno == Colore::Nero { h ^= z.turno; } h
    }

    #[inline(always)] pub fn pezzo_in(&self, sq: usize) -> Option<usize> {
        let m = 1 << sq; if (self.occupazione() & m) == 0 { return None; }
        for p in 0..6 { if (self.pezzi[p] & m) != 0 { return Some(p); } } None
    }

    pub fn pezzo_e_colore_in(&self, sq: usize) -> Option<(Colore, Pezzo)> {
        if let Some(p_idx) = self.pezzo_in(sq) {
            let c = if (self.colori[0] & (1 << sq)) != 0 { Colore::Bianco } else { Colore::Nero };
            return Some((c, Pezzo::from_index(p_idx)));
        } None
    }

    pub fn re_in_scacco(&self, c: Colore) -> bool {
        let k = self.pezzi[5] & self.colori[c.indice()];
        if k == 0 { return false; }
        crate::attacks::square_attacked(self, k.trailing_zeros() as usize, c.opposto())
    }

    pub fn esegui_mossa(&mut self, m: &Mossa, z: &ZobristKeys, nnue: Option<&LunaNNUE>) -> bool {
        let (from, to) = (m.da(), m.a());
        let flag = m.move_flag();
        let us = self.turno.indice();
        let them = 1 - us;

        // Validazione mossa TT sporca
        if (self.colori[us] & (1 << from)) == 0 || (self.colori[us] & (1 << to)) != 0 { return false; }

        let mut moved_p = 0;
        for p in 0..6 { if (self.pezzi[p] & (1 << from)) != 0 { moved_p = p; break; } }
        let cap_p = if flag == MoveFlag::EnPassant { Some(0) } else { self.pezzo_in(to) };

        let undo = UndoData { hash: self.hash, ep_square: self.ep_square, diritti_arrocco: self.diritti_arrocco, mezze_mosse: self.mezze_mosse, cattura_p: cap_p, accumulator: self.accumulator.clone() };

        // Aggiornamento Incrementale NNUE
        if let Some(net) = nnue { self.accumulator = net.update_accumulator(&self.accumulator, m, self); }

        // Movimento Pezzi
        self.pezzi[moved_p] &= !(1 << from); self.colori[us] &= !(1 << from);
        if flag == MoveFlag::EnPassant {
            let cs = if us == 0 { to - 8 } else { to + 8 };
            self.pezzi[0] &= !(1 << cs); self.colori[them] &= !(1 << cs);
        } else if let Some(p) = cap_p {
            self.pezzi[p] &= !(1 << to); self.colori[them] &= !(1 << to);
        }

        let mut final_p = moved_p;
        if flag.is_promotion() { final_p = m.pezzo_promosso().unwrap_or(Pezzo::Regina).indice(); }
        self.pezzi[final_p] |= 1 << to; self.colori[us] |= 1 << to;

        if flag == MoveFlag::Castle {
            let (rf, rt) = match to { 6 => (7, 5), 2 => (0, 3), 62 => (63, 61), 58 => (56, 59), _ => (0,0) };
            self.pezzi[3] ^= (1 << rf) | (1 << rt); self.colori[us] ^= (1 << rf) | (1 << rt);
        }

        self.history.push(undo);
        self.ep_square = if flag == MoveFlag::DoublePawnPush { Some(if us == 0 { to - 8 } else { to + 8 }) } else { None };
        self.diritti_arrocco &= CR_UPDATE[from] & CR_UPDATE[to];
        self.turno = self.turno.opposto(); self.hash = self.get_hash(z);

        if self.re_in_scacco(Colore::from_index(us)) { self.annulla_mossa(m, z); return false; }
        self.ply += 1; true
    }

    pub fn annulla_mossa(&mut self, m: &Mossa, _z: &ZobristKeys) {
        let u = if let Some(undo) = self.history.pop() { undo } else { return };
        let us = self.turno.opposto().indice();
        let them = 1 - us;
        let from = m.da(); let to = m.a();
        let flag = m.move_flag();

        self.accumulator = u.accumulator; // Ripristino istantaneo!

        let mut current_p = 0;
        for p in 0..6 { if (self.pezzi[p] & (1 << to)) != 0 { current_p = p; break; } }

        self.pezzi[current_p] &= !(1 << to); self.colori[us] &= !(1 << to);
        let moved_p = if flag.is_promotion() { 0 } else { current_p };
        self.pezzi[moved_p] |= 1 << from; self.colori[us] |= 1 << from;

        if flag == MoveFlag::EnPassant {
            let cs = if us == 0 { to - 8 } else { to + 8 };
            self.pezzi[0] |= 1 << cs; self.colori[them] |= 1 << cs;
        } else if let Some(p) = u.cattura_p {
            self.pezzi[p] |= 1 << to; self.colori[them] |= 1 << to;
        }

        if flag == MoveFlag::Castle {
            let (rf, rt) = match to { 6 => (7, 5), 2 => (0, 3), 62 => (63, 61), 58 => (56, 59), _ => (0,0) };
            self.pezzi[3] ^= (1 << rf) | (1 << rt); self.colori[us] ^= (1 << rf) | (1 << rt);
        }

        self.turno = Colore::from_index(us);
        self.hash = u.hash; self.ep_square = u.ep_square; self.diritti_arrocco = u.diritti_arrocco; self.mezze_mosse = u.half;
        self.ply -= 1;
    }

    pub fn fai_mossa_nulla(&mut self, z: &ZobristKeys) -> UndoData {
        let undo = UndoData { hash: self.hash, ep_square: self.ep_square, diritti_arrocco: self.diritti_arrocco, mezze_mosse: self.mezze_mosse, cattura_p: None, accumulator: self.accumulator.clone() };
        self.turno = self.turno.opposto(); self.hash ^= z.turno; self.history.push(undo.clone()); self.ply += 1; undo
    }

    pub fn annulla_mossa_nulla(&mut self, undo: UndoData, _z: &ZobristKeys) {
        self.hash = undo.hash; self.ep_square = undo.ep_square; self.diritti_arrocco = undo.diritti_arrocco; self.mezze_mosse = undo.half;
        self.accumulator = undo.accumulator; self.turno = self.turno.opposto(); self.history.pop(); self.ply -= 1;
    }

    pub fn genera_mosse_legali(&self, z: &ZobristKeys) -> Vec<Mossa> {
        let mut legali = Vec::new();
        for m in crate::movegen::genera_mosse(self) {
            let mut c = self.clone(); if c.esegui_mossa(&m, z, None) { legali.push(m); }
        }
        legali
    }

    pub fn to_fen(&self) -> String {
        let mut f = String::new();
        for r in (0..8).rev() {
            let mut e = 0;
            for file in 0..8 {
                let sq = r * 8 + file;
                if let Some((c, p)) = self.pezzo_e_colore_in(sq) {
                    if e > 0 { f.push_str(&e.to_string()); e = 0; }
                    let mut ch = match p { Pezzo::Pedone=>'p', Pezzo::Cavallo=>'n', Pezzo::Alfiere=>'b', Pezzo::Torre=>'r', Pezzo::Regina=>'q', Pezzo::Re=>'k' };
                    if c == Colore::Bianco { ch = ch.to_ascii_uppercase(); } f.push(ch);
                } else { e += 1; }
            }
            if e > 0 { f.push_str(&e.to_string()); } if r > 0 { f.push('/'); }
        }
        f.push_str(if self.turno == Colore::Bianco { " w " } else { " b " }); f
    }
}

impl fmt::Display for Scacchiera { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}", self.to_fen()) } }