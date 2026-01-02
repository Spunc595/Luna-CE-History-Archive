use std::fmt;
use crate::attacks::{pawn_attacks, knight_attacks, king_attacks, bishop_attacks, rook_attacks, queen_attacks};

pub type Bitboard = u64;
pub const NNUE_LAYER1_SIZE: usize = 1536;

#[derive(Clone, Copy)]
pub struct NNUEAccumulator { pub v: [i16; NNUE_LAYER1_SIZE] }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Colore { Bianco = 0, Nero = 1 }
impl Colore {
    pub fn opposto(&self) -> Self { if *self == Colore::Bianco { Colore::Nero } else { Colore::Bianco } }
    pub fn indice(&self) -> usize { *self as usize }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pezzo { Pedone=0, Cavallo=1, Alfiere=2, Torre=3, Regina=4, Re=5 }
impl Pezzo { pub fn indice(&self) -> usize { *self as usize } }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveFlag { None, EnPassant, CastleKingSide, CastleQueenSide, Capture, Promotion, PromotionCapture, DoublePawnPush }

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Casella(pub usize);
impl Casella {
    pub fn indice(&self) -> usize { self.0 }
    pub fn nome(&self) -> String { format!("{}{}", (b'a' + (self.0 % 8) as u8) as char, (b'1' + (self.0 / 8) as u8) as char) }
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
        match (self.data >> 12) & 0xF { 0..=5 | 8..=15 => unsafe { std::mem::transmute(((self.data >> 12) & 0xF) as u8) }, _ => MoveFlag::None }
    }
    pub fn promozione(&self) -> Option<Pezzo> {
        let f = (self.data >> 12) & 0xF; if f < 8 { return None; }
        match f & 3 { 0=>Some(Pezzo::Cavallo), 1=>Some(Pezzo::Alfiere), 2=>Some(Pezzo::Torre), 3=>Some(Pezzo::Regina), _=>None }
    }
    pub fn to_u16(&self) -> u16 { self.data }
}

impl fmt::Display for Mossa {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let p = match self.promozione() { Some(Pezzo::Regina)=>"q", Some(Pezzo::Torre)=>"r", Some(Pezzo::Alfiere)=>"b", Some(Pezzo::Cavallo)=>"n", _=>"" };
        write!(f, "{}{}{}", self.da().nome(), self.a().nome(), p)
    }
}

#[derive(Clone)]
pub struct GameState { pub en_passant: Option<usize>, pub diritti_arrocco: u8, pub captured_piece: Option<Pezzo>, pub last_move: Mossa, pub acc: [NNUEAccumulator; 2] }

#[derive(Clone)]
pub struct Scacchiera { pub pezzi: [Bitboard; 6], pub colori: [Bitboard; 2], pub turno: Colore, pub en_passant: Option<usize>, pub diritti_arrocco: u8, pub current_acc: [NNUEAccumulator; 2], pub history: Vec<GameState> }

impl Scacchiera {
    pub fn nuova() -> Self { Self::da_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap() }

    pub fn da_fen(fen: &str) -> Result<Self, String> {
        let parts: Vec<&str> = fen.split_whitespace().collect();
        let mut pezzi = [0u64; 6]; let mut colori = [0u64; 2];
        let mut row = 7; let mut col = 0;
        for c in parts[0].chars() {
            if c == '/' { row -= 1; col = 0; }
            else if let Some(d) = c.to_digit(10) { col += d as usize; }
            else {
                let color = if c.is_uppercase() { Colore::Bianco } else { Colore::Nero };
                let piece = match c.to_ascii_lowercase() { 'p'=>Pezzo::Pedone, 'n'=>Pezzo::Cavallo, 'b'=>Pezzo::Alfiere, 'r'=>Pezzo::Torre, 'q'=>Pezzo::Regina, 'k'=>Pezzo::Re, _=>return Err("FEN error".into()) };
                let bit = 1u64 << (row * 8 + col);
                pezzi[piece.indice()] |= bit; colori[color.indice()] |= bit; col += 1;
            }
        }
        let turno = if parts[1] == "w" { Colore::Bianco } else { Colore::Nero };
        Ok(Scacchiera { pezzi, colori, turno, en_passant: None, diritti_arrocco: 0xF, current_acc: [NNUEAccumulator { v: [0; NNUE_LAYER1_SIZE] }; 2], history: Vec::with_capacity(256) })
    }

    pub fn esegui_mossa(&mut self, m: &Mossa, net: &crate::nnue::Network) -> bool {
        let us = self.turno; let from = m.da().indice(); let to = m.a().indice();
        let piece = self.get_piece_at(from).unwrap();
        let cap = if m.flag() == MoveFlag::EnPassant { Some(Pezzo::Pedone) } else { self.get_piece_at(to) };
        self.history.push(GameState { en_passant: self.en_passant, diritti_arrocco: self.diritti_arrocco, captured_piece: cap, last_move: *m, acc: self.current_acc.clone() });
        self.aggiorna_nnue_rimuovi(piece, us, from, net);
        if let Some(cp) = cap { self.aggiorna_nnue_rimuovi(cp, us.opposto(), to, net); }
        let promo = m.promozione().unwrap_or(piece);
        self.aggiorna_nnue_aggiungi(promo, us, to, net);
        self.toggle_piece(us, piece, from);
        if let Some(cp) = cap { let bit = 1 << to; self.pezzi[cp.indice()] &= !bit; self.colori[us.opposto().indice()] &= !bit; }
        self.toggle_piece(us, promo, to);
        self.turno = us.opposto();
        true
    }

    pub fn annulla_mossa(&mut self) {
        let s = self.history.pop().expect("No moves");
        self.turno = self.turno.opposto();
        self.current_acc = s.acc;
        let m = s.last_move; let to = m.a().indice(); let from = m.da().indice();
        let p_to = self.get_piece_at(to).unwrap();
        let p_from = if m.promozione().is_some() { Pezzo::Pedone } else { p_to };
        self.toggle_piece(self.turno, p_to, to);
        self.toggle_piece(self.turno, p_from, from);
        if let Some(cp) = s.captured_piece { let bit = 1 << to; self.pezzi[cp.indice()] |= bit; self.colori[self.turno.opposto().indice()] |= bit; }
    }

    pub fn inizializza_acc_nnue(&mut self, net: &crate::nnue::Network) {
        self.current_acc[0].v.copy_from_slice(&net.feature_bias);
        self.current_acc[1].v.copy_from_slice(&net.feature_bias);
        for sq in 0..64 { if let Some(p) = self.get_piece_at(sq) { self.aggiorna_nnue_aggiungi(p, self.get_color_at(sq).unwrap(), sq, net); } }
    }

    pub fn aggiorna_nnue_aggiungi(&mut self, p: Pezzo, c: Colore, sq: usize, net: &crate::nnue::Network) {
        let idx_w = crate::nnue::get_feature_index(p, c, sq);
        let idx_b = crate::nnue::get_feature_index(p, c, sq ^ 56);
        let w_w = &net.feature_weights[idx_w * NNUE_LAYER1_SIZE..(idx_w + 1) * NNUE_LAYER1_SIZE];
        let w_b = &net.feature_weights[idx_b * NNUE_LAYER1_SIZE..(idx_b + 1) * NNUE_LAYER1_SIZE];
        for i in 0..NNUE_LAYER1_SIZE { self.current_acc[0].v[i] += w_w[i]; self.current_acc[1].v[i] += w_b[i]; }
    }

    pub fn aggiorna_nnue_rimuovi(&mut self, p: Pezzo, c: Colore, sq: usize, net: &crate::nnue::Network) {
        let idx_w = crate::nnue::get_feature_index(p, c, sq);
        let idx_b = crate::nnue::get_feature_index(p, c, sq ^ 56);
        let w_w = &net.feature_weights[idx_w * NNUE_LAYER1_SIZE..(idx_w + 1) * NNUE_LAYER1_SIZE];
        let w_b = &net.feature_weights[idx_b * NNUE_LAYER1_SIZE..(idx_b + 1) * NNUE_LAYER1_SIZE];
        for i in 0..NNUE_LAYER1_SIZE { self.current_acc[0].v[i] -= w_w[i]; self.current_acc[1].v[i] -= w_b[i]; }
    }

    pub fn get_piece_at(&self, sq: usize) -> Option<Pezzo> {
        let bit = 1u64 << sq;
        for p in 0..6 { if (self.pezzi[p] & bit) != 0 { return unsafe { Some(std::mem::transmute(p as u8)) }; } }
        None
    }
    pub fn get_color_at(&self, sq: usize) -> Option<Colore> {
        let bit = 1u64 << sq;
        if (self.colori[0] & bit) != 0 { Some(Colore::Bianco) } else if (self.colori[1] & bit) != 0 { Some(Colore::Nero) } else { None }
    }
    pub fn bitboard_colore(&self, c: Colore) -> Bitboard { self.colori[c.indice()] }
    pub fn bitboard_pezzo_colore(&self, p: Pezzo, c: Colore) -> Bitboard { self.pezzi[p.indice()] & self.colori[c.indice()] }
    pub fn get_attacks_for_piece(&self, p: Pezzo, sq: usize, occ: Bitboard) -> Bitboard {
        match p { Pezzo::Pedone => pawn_attacks(sq, self.get_color_at(sq).unwrap_or(Colore::Bianco)), Pezzo::Cavallo => knight_attacks(sq), Pezzo::Alfiere => bishop_attacks(sq, occ), Pezzo::Torre => rook_attacks(sq, occ), Pezzo::Regina => queen_attacks(sq, occ), Pezzo::Re => king_attacks(sq) }
    }
    pub fn diritti_arrocco_struct(&self) -> crate::nnue::CastlingRights {
        crate::nnue::CastlingRights { bianco_lato_re: (self.diritti_arrocco & 1) != 0, bianco_lato_regina: (self.diritti_arrocco & 2) != 0, nero_lato_re: (self.diritti_arrocco & 4) != 0, nero_lato_regina: (self.diritti_arrocco & 8) != 0 }
    }
    pub fn turno(&self) -> Colore { self.turno }
    pub fn occupazione(&self) -> Bitboard { self.colori[0] | self.colori[1] }
    pub fn re_in_scacco(&self, c: Colore) -> bool {
        let k = self.pezzi[Pezzo::Re.indice()] & self.colori[c.indice()];
        if k == 0 { return false; } self.casella_attaccata(Casella(k.trailing_zeros() as usize), c.opposto())
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
    fn toggle_piece(&mut self, c: Colore, p: Pezzo, s: usize) { let b = 1u64 << s; self.pezzi[p.indice()] ^= b; self.colori[c.indice()] ^= b; }
    pub fn hash(&self) -> u64 { 0 }
    pub fn ha_pezzi_maggiori(&self, c: Colore) -> bool { (self.pezzi[1]|self.pezzi[2]|self.pezzi[3]|self.pezzi[4]) & self.colori[c.indice()] != 0 }
    pub fn history_mosse_str(&self) -> Vec<String> { self.history.iter().map(|h| h.last_move.to_string()).collect() }
}

struct ZobristKeys { pezzi: [[[u64; 64]; 6]; 2], turno: u64, arrocco: [u64; 16], en_passant: [u64; 8] }
impl ZobristKeys { fn new() -> Self { unsafe { std::mem::zeroed() } } }