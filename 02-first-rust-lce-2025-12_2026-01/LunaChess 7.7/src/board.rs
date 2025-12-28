// src/board.rs

use std::fmt;
use crate::attacks::{pawn_attacks, knight_attacks, king_attacks, bishop_attacks, rook_attacks};

// ===== TIPI BASE =====

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
pub struct Casella(usize);
impl Casella {
    pub fn da_indice(idx: usize) -> Option<Self> { if idx < 64 { Some(Casella(idx)) } else { None } }
    pub fn indice(&self) -> usize { self.0 }
    pub fn rank(&self) -> u8 { (self.0 / 8) as u8 }
    pub fn da_nome(nome: &str) -> Option<Self> {
        if nome.len() != 2 { return None; }
        let b = nome.as_bytes();
        let f = b[0].wrapping_sub(b'a'); let r = b[1].wrapping_sub(b'1');
        if f < 8 && r < 8 { Some(Casella((r*8+f) as usize)) } else { None }
    }
    pub fn nome(&self) -> String {
        let f = (self.0 % 8) as u8; let r = (self.0 / 8) as u8;
        format!("{}{}", (b'a'+f) as char, (b'1'+r) as char)
    }
}
pub type Bitboard = u64;

// ===== MOSSA =====

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveFlag { None, EnPassant, CastleKingSide, CastleQueenSide, Capture, Promotion, PromotionCapture, DoublePawnPush }
impl MoveFlag {
    pub fn is_capture(&self) -> bool { matches!(self, MoveFlag::Capture | MoveFlag::PromotionCapture | MoveFlag::EnPassant) }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Mossa { pub data: u32 } // Pub data per creare mosse nulle manuali
impl fmt::Display for Mossa {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Se è una mossa nulla (data == 0), stampiamo "0000"
        if self.data == 0 { return write!(f, "0000"); }
        
        write!(f, "{}{}{}", self.da().nome(), self.a().nome(), match self.promozione() { Some(Pezzo::Regina)=>"q", Some(Pezzo::Torre)=>"r", Some(Pezzo::Alfiere)=>"b", Some(Pezzo::Cavallo)=>"n", _=>"" })
    }
}
impl Mossa {
    pub fn new(da: Casella, a: Casella, prom: Option<Pezzo>) -> Self { Self::new_with_flag(da, a, prom, MoveFlag::None) }
    pub fn new_with_flag(da: Casella, a: Casella, prom: Option<Pezzo>, flag: MoveFlag) -> Self {
        let p_bits = match prom { None=>0, Some(Pezzo::Regina)=>1, Some(Pezzo::Torre)=>2, Some(Pezzo::Alfiere)=>3, Some(Pezzo::Cavallo)=>4, _=>0 };
        let f_bits = match flag { MoveFlag::None=>0, MoveFlag::EnPassant=>1, MoveFlag::CastleKingSide=>2, MoveFlag::CastleQueenSide=>3, MoveFlag::Capture=>4, MoveFlag::Promotion=>5, MoveFlag::PromotionCapture=>6, MoveFlag::DoublePawnPush=>7 };
        Mossa { data: (f_bits << 15) | (p_bits << 12) | ((a.indice() as u32) << 6) | (da.indice() as u32) }
    }
    pub fn da(&self) -> Casella { Casella::da_indice((self.data & 0x3F) as usize).unwrap() }
    pub fn a(&self) -> Casella { Casella::da_indice(((self.data >> 6) & 0x3F) as usize).unwrap() }
    pub fn promozione(&self) -> Option<Pezzo> { match (self.data >> 12) & 0x7 { 1=>Some(Pezzo::Regina), 2=>Some(Pezzo::Torre), 3=>Some(Pezzo::Alfiere), 4=>Some(Pezzo::Cavallo), _=>None } }
    pub fn flag(&self) -> MoveFlag { match (self.data >> 15) & 0xF { 1=>MoveFlag::EnPassant, 2=>MoveFlag::CastleKingSide, 3=>MoveFlag::CastleQueenSide, 4=>MoveFlag::Capture, 5=>MoveFlag::Promotion, 6=>MoveFlag::PromotionCapture, 7=>MoveFlag::DoublePawnPush, _=>MoveFlag::None } }
    pub fn is_promotion(&self) -> bool { self.promozione().is_some() }
    pub fn to_u32(&self) -> u32 { self.data }
    pub fn from_u32(val: u32) -> Option<Self> { Some(Mossa { data: val }) }
    pub fn from_uci(uci: &str) -> Option<Self> {
        if uci == "0000" { return Some(Mossa{data:0}); }
        if uci.len() < 4 { return None; }
        let from = Casella::da_nome(&uci[0..2])?; let to = Casella::da_nome(&uci[2..4])?;
        let promo = if uci.len() > 4 { Pezzo::da_char(uci.chars().nth(4)?) } else { None };
        Some(Self::new(from, to, promo))
    }
}

// ===== SCACCHIERA E STATO =====

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
    pub history: Vec<GameState>,
}

impl fmt::Display for Scacchiera {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = String::new();
        s.push_str("\n");
        for r in (0..8).rev() {
            s.push_str(&format!("{} | ", r+1));
            for c in 0..8 {
                let sq = Casella(r*8+c);
                let mut ch = '.';
                if let Some((p, col)) = self.pezzo_su_casella(sq) {
                    ch = match p { Pezzo::Pedone=>'p', Pezzo::Cavallo=>'n', Pezzo::Alfiere=>'b', Pezzo::Torre=>'r', Pezzo::Regina=>'q', Pezzo::Re=>'k' };
                    if col == Colore::Bianco { ch = ch.to_ascii_uppercase(); }
                }
                s.push(ch); s.push(' ');
            }
            s.push_str("|\n");
        }
        s.push_str("    a b c d e f g h\n");
        write!(f, "{}", s)
    }
}

impl Scacchiera {
    pub fn nuova() -> Self { Self::da_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap() }
    
    pub fn da_fen(fen: &str) -> Result<Self, String> {
        let parts: Vec<&str> = fen.split_whitespace().collect();
        if parts.len() < 4 { return Err("FEN errato".to_string()); }
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
        
        Ok(Scacchiera { pezzi, colori, turno, en_passant: ep, diritti_arrocco: cast, history: Vec::with_capacity(256) })
    }
    
    pub fn a_fen(&self) -> String { String::from("todo") }

    #[inline] pub fn turno(&self) -> Colore { self.turno }
    #[inline] pub fn occupazione(&self) -> Bitboard { self.colori[0] | self.colori[1] }
    #[inline] pub fn bitboard_colore(&self, c: Colore) -> Bitboard { self.colori[c.indice()] }
    #[inline] pub fn bitboard_pezzo(&self, p: Pezzo) -> Bitboard { self.pezzi[p.indice()] }
    #[inline] pub fn bitboard_pezzo_colore(&self, p: Pezzo, c: Colore) -> Bitboard { self.pezzi[p.indice()] & self.colori[c.indice()] }
    #[inline] pub fn en_passant_sq(&self) -> Option<usize> { self.en_passant }
    #[inline] pub fn diritti_arrocco_struct(&self) -> CastlingRights {
        CastlingRights { bianco_lato_re: (self.diritti_arrocco&1)!=0, bianco_lato_regina: (self.diritti_arrocco&2)!=0, nero_lato_re: (self.diritti_arrocco&4)!=0, nero_lato_regina: (self.diritti_arrocco&8)!=0 }
    }

    pub fn pezzo_su_casella(&self, casella: Casella) -> Option<(Pezzo, Colore)> {
        let bit = 1u64 << casella.indice();
        if (self.occupazione() & bit) == 0 { return None; }
        let col = if (self.colori[0] & bit) != 0 { Colore::Bianco } else { Colore::Nero };
        for p in [Pezzo::Pedone, Pezzo::Cavallo, Pezzo::Alfiere, Pezzo::Torre, Pezzo::Regina, Pezzo::Re] {
            if (self.pezzi[p.indice()] & bit) != 0 { return Some((p, col)); }
        }
        None
    }

    pub fn hash(&self) -> u64 {
        let mut h = self.colori[0];
        h ^= self.colori[1].wrapping_mul(0x9E3779B97F4A7C15);
        h ^= (self.turno as u64).wrapping_mul(0xBF58476D1CE4E5B9);
        h ^= (self.diritti_arrocco as u64).wrapping_mul(0x94D049BB133111EB);
        if let Some(ep) = self.en_passant { h ^= (ep as u64).wrapping_mul(0xC6A4A7935BD1E995); }
        h
    }

    pub fn valore_materiale_totale(&self) -> i32 {
        let mut s = 0;
        for p in [Pezzo::Pedone, Pezzo::Cavallo, Pezzo::Alfiere, Pezzo::Torre, Pezzo::Regina] {
            s += (self.bitboard_pezzo_colore(p, Colore::Bianco).count_ones() as i32 - self.bitboard_pezzo_colore(p, Colore::Nero).count_ones() as i32) * p.valore();
        }
        if self.turno == Colore::Nero { -s } else { s }
    }

    // --- ATTACCHI ---
    pub fn casella_attaccata(&self, sq: Casella, da_colore: Colore) -> bool {
        let s = sq.indice();
        let occ = self.occupazione();
        if (pawn_attacks(s, da_colore.opposto()) & self.bitboard_pezzo_colore(Pezzo::Pedone, da_colore)) != 0 { return true; }
        if (knight_attacks(s) & self.bitboard_pezzo_colore(Pezzo::Cavallo, da_colore)) != 0 { return true; }
        if (king_attacks(s) & self.bitboard_pezzo_colore(Pezzo::Re, da_colore)) != 0 { return true; }
        if (bishop_attacks(s, occ) & (self.bitboard_pezzo_colore(Pezzo::Alfiere, da_colore) | self.bitboard_pezzo_colore(Pezzo::Regina, da_colore))) != 0 { return true; }
        if (rook_attacks(s, occ) & (self.bitboard_pezzo_colore(Pezzo::Torre, da_colore) | self.bitboard_pezzo_colore(Pezzo::Regina, da_colore))) != 0 { return true; }
        false
    }

    pub fn re_in_scacco(&self, colore: Colore) -> bool {
        let k = self.bitboard_pezzo_colore(Pezzo::Re, colore);
        if k == 0 { return true; }
        self.casella_attaccata(Casella(k.trailing_zeros() as usize), colore.opposto())
    }

    pub fn attacchi_alfiere(&self, sq: Casella, occ: Bitboard) -> Bitboard { bishop_attacks(sq.indice(), occ) }
    pub fn attacchi_torre(&self, sq: Casella, occ: Bitboard) -> Bitboard { rook_attacks(sq.indice(), occ) }

    // ===========================================
    // MAKE / UNMAKE
    // ===========================================

    pub fn esegui_mossa(&mut self, mossa: &Mossa) -> bool {
        let us = self.turno;
        let them = us.opposto();
        let from = mossa.da().indice();
        let to = mossa.a().indice();
        let from_bit = 1u64 << from;
        let to_bit = 1u64 << to;

        let piece = match self.pezzo_su_casella(Casella(from)) {
            Some((p, c)) if c == us => p,
            _ => return false,
        };

        let mut captured = None;
        if mossa.flag().is_capture() {
            if mossa.flag() == MoveFlag::EnPassant {
                captured = Some(Pezzo::Pedone);
            } else {
                if let Some((p, c)) = self.pezzo_su_casella(Casella(to)) {
                    if c == them { captured = Some(p); }
                }
            }
        }

        self.history.push(GameState {
            en_passant: self.en_passant,
            diritti_arrocco: self.diritti_arrocco,
            captured_piece: captured,
            last_move: *mossa,
            hash: self.hash(),
        });

        self.pezzi[piece.indice()] &= !from_bit;
        self.colori[us.indice()] &= !from_bit;

        if let Some(cap_p) = captured {
            let cap_sq_idx = if mossa.flag() == MoveFlag::EnPassant {
                if us == Colore::Bianco { to - 8 } else { to + 8 }
            } else { to };
            let cap_bit = 1u64 << cap_sq_idx;
            self.pezzi[cap_p.indice()] &= !cap_bit;
            self.colori[them.indice()] &= !cap_bit;
        }

        let placed_piece = mossa.promozione().unwrap_or(piece);
        self.pezzi[placed_piece.indice()] |= to_bit;
        self.colori[us.indice()] |= to_bit;

        if mossa.flag() == MoveFlag::CastleKingSide {
            let (r_from, r_to) = if us == Colore::Bianco { (7, 5) } else { (63, 61) };
            self.move_rook(r_from, r_to, us);
        } else if mossa.flag() == MoveFlag::CastleQueenSide {
            let (r_from, r_to) = if us == Colore::Bianco { (0, 3) } else { (56, 59) };
            self.move_rook(r_from, r_to, us);
        }

        if piece == Pezzo::Re {
            if us == Colore::Bianco { self.diritti_arrocco &= !3; } else { self.diritti_arrocco &= !12; }
        }
        self.diritti_arrocco &= !self.get_castle_mask(from);
        self.diritti_arrocco &= !self.get_castle_mask(to);

        if mossa.flag() == MoveFlag::DoublePawnPush {
            self.en_passant = Some(if us == Colore::Bianco { to - 8 } else { to + 8 });
        } else {
            self.en_passant = None;
        }

        self.turno = them;
        true
    }

    pub fn annulla_mossa(&mut self) {
        let state = match self.history.pop() {
            Some(s) => s,
            None => return,
        };

        self.turno = self.turno.opposto();
        let us = self.turno;
        let them = us.opposto();
        self.en_passant = state.en_passant;
        self.diritti_arrocco = state.diritti_arrocco;

        let m = state.last_move;
        let from = m.da().indice();
        let to = m.a().indice();
        let from_bit = 1u64 << from;
        let to_bit = 1u64 << to;

        let moved_piece = if m.is_promotion() { Pezzo::Pedone } else {
            self.get_piece_at_no_color(to).unwrap()
        };

        let piece_on_dest = if m.is_promotion() { m.promozione().unwrap() } else { moved_piece };
        self.pezzi[piece_on_dest.indice()] &= !to_bit;
        self.colori[us.indice()] &= !to_bit;

        self.pezzi[moved_piece.indice()] |= from_bit;
        self.colori[us.indice()] |= from_bit;

        if let Some(cap_p) = state.captured_piece {
            let cap_sq_idx = if m.flag() == MoveFlag::EnPassant {
                if us == Colore::Bianco { to - 8 } else { to + 8 }
            } else { to };
            let cap_bit = 1u64 << cap_sq_idx;
            self.pezzi[cap_p.indice()] |= cap_bit;
            self.colori[them.indice()] |= cap_bit;
        }

        if m.flag() == MoveFlag::CastleKingSide {
            let (r_from, r_to) = if us == Colore::Bianco { (7, 5) } else { (63, 61) };
            self.move_rook(r_to, r_from, us);
        } else if m.flag() == MoveFlag::CastleQueenSide {
            let (r_from, r_to) = if us == Colore::Bianco { (0, 3) } else { (56, 59) };
            self.move_rook(r_to, r_from, us);
        }
    }

    // ===========================================
    // GESTIONE NULL MOVE (Passare il turno)
    // ===========================================

    pub fn esegui_mossa_nulla(&mut self) {
        // Salva stato
        let null_move = Mossa { data: 0 }; 
        self.history.push(GameState {
            en_passant: self.en_passant,
            diritti_arrocco: self.diritti_arrocco,
            captured_piece: None,
            last_move: null_move,
            hash: self.hash(),
        });

        // Applica effetti
        self.en_passant = None;
        self.turno = self.turno.opposto();
    }

    pub fn annulla_mossa_nulla(&mut self) {
        let state = match self.history.pop() {
            Some(s) => s,
            None => return,
        };
        self.turno = self.turno.opposto();
        self.en_passant = state.en_passant;
        self.diritti_arrocco = state.diritti_arrocco;
    }
    
    // Controlla se abbiamo pezzi oltre a Re e Pedoni (per evitare Zugzwang in NMP)
    pub fn ha_pezzi_maggiori(&self, colore: Colore) -> bool {
        let occ = self.colori[colore.indice()];
        let kings = self.pezzi[Pezzo::Re.indice()];
        let pawns = self.pezzi[Pezzo::Pedone.indice()];
        (occ & !kings & !pawns) != 0
    }

    // Helpers
    fn move_rook(&mut self, from: usize, to: usize, c: Colore) {
        let f = 1u64 << from; let t = 1u64 << to;
        self.pezzi[Pezzo::Torre.indice()] &= !f; self.pezzi[Pezzo::Torre.indice()] |= t;
        self.colori[c.indice()] &= !f; self.colori[c.indice()] |= t;
    }

    fn get_piece_at_no_color(&self, sq: usize) -> Option<Pezzo> {
        let bit = 1u64 << sq;
        for p in [Pezzo::Pedone, Pezzo::Cavallo, Pezzo::Alfiere, Pezzo::Torre, Pezzo::Regina, Pezzo::Re] {
            if (self.pezzi[p.indice()] & bit) != 0 { return Some(p); }
        }
        None
    }

    fn get_castle_mask(&self, sq: usize) -> u8 {
        match sq { 0=>2, 7=>1, 56=>8, 63=>4, 4=>3, 60=>12, _=>0 }
    }
}