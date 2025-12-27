// src/board.rs

use std::fmt;
use crate::attacks::{pawn_attacks, knight_attacks, king_attacks, bishop_attacks, rook_attacks};

// ===== TIPI BASE =====

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Colore {
    Bianco = 0,
    Nero = 1,
}

impl Colore {
    pub fn opposto(&self) -> Self {
        match self {
            Colore::Bianco => Colore::Nero,
            Colore::Nero => Colore::Bianco,
        }
    }

    pub fn indice(&self) -> usize {
        *self as usize
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pezzo {
    Pedone = 0,
    Cavallo = 1,
    Alfiere = 2,
    Torre = 3,
    Regina = 4,
    Re = 5,
}

impl Pezzo {
    pub fn indice(&self) -> usize {
        *self as usize
    }

    pub fn valore(&self) -> i32 {
        match self {
            Pezzo::Pedone => 100,
            Pezzo::Cavallo => 320,
            Pezzo::Alfiere => 330,
            Pezzo::Torre => 500,
            Pezzo::Regina => 900,
            Pezzo::Re => 20000,
        }
    }
    
    pub fn da_char(c: char) -> Option<Self> {
        match c.to_ascii_lowercase() {
            'p' => Some(Pezzo::Pedone),
            'n' => Some(Pezzo::Cavallo),
            'b' => Some(Pezzo::Alfiere),
            'r' => Some(Pezzo::Torre),
            'q' => Some(Pezzo::Regina),
            'k' => Some(Pezzo::Re),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Casella(usize);

impl Casella {
    pub fn da_indice(idx: usize) -> Option<Self> {
        if idx < 64 { Some(Casella(idx)) } else { None }
    }

    pub fn indice(&self) -> usize {
        self.0
    }
    
    pub fn rank(&self) -> u8 {
        (self.0 / 8) as u8
    }
    
    pub fn da_nome(nome: &str) -> Option<Self> {
        if nome.len() != 2 { return None; }
        let bytes = nome.as_bytes();
        let file = bytes[0].wrapping_sub(b'a');
        let rank = bytes[1].wrapping_sub(b'1');
        if file < 8 && rank < 8 {
            Some(Casella((rank * 8 + file) as usize))
        } else {
            None
        }
    }
    
    pub fn nome(&self) -> String {
        let file = (self.0 % 8) as u8;
        let rank = (self.0 / 8) as u8;
        format!("{}{}", (b'a' + file) as char, (b'1' + rank) as char)
    }
}

pub type Bitboard = u64;

// ===== MOSSA =====

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveFlag {
    None,
    EnPassant,
    CastleKingSide,
    CastleQueenSide,
    Capture,
    Promotion,
    PromotionCapture,
    DoublePawnPush, 
}

impl MoveFlag {
    pub fn is_capture(&self) -> bool {
        matches!(self, MoveFlag::Capture | MoveFlag::PromotionCapture | MoveFlag::EnPassant)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Mossa {
    data: u32, // Bitfield: 6 from, 6 to, 3 promo, 4 flag
}

impl fmt::Display for Mossa {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let from = self.da().nome();
        let to = self.a().nome();
        let promo = match self.promozione() {
            Some(Pezzo::Regina) => "q",
            Some(Pezzo::Torre) => "r",
            Some(Pezzo::Alfiere) => "b",
            Some(Pezzo::Cavallo) => "n",
            _ => "",
        };
        write!(f, "{}{}{}", from, to, promo)
    }
}

impl Mossa {
    pub fn new(da: Casella, a: Casella, promozione: Option<Pezzo>) -> Self {
        Self::new_with_flag(da, a, promozione, MoveFlag::None)
    }

    pub fn new_with_flag(da: Casella, a: Casella, promozione: Option<Pezzo>, flag: MoveFlag) -> Self {
        let from_bits = da.indice() as u32;
        let to_bits = a.indice() as u32;
        let promo_bits = match promozione {
            None => 0,
            Some(Pezzo::Regina) => 1,
            Some(Pezzo::Torre) => 2,
            Some(Pezzo::Alfiere) => 3,
            Some(Pezzo::Cavallo) => 4,
            _ => 0,
        };
        let flag_bits = match flag {
            MoveFlag::None => 0,
            MoveFlag::EnPassant => 1,
            MoveFlag::CastleKingSide => 2,
            MoveFlag::CastleQueenSide => 3,
            MoveFlag::Capture => 4,
            MoveFlag::Promotion => 5,
            MoveFlag::PromotionCapture => 6,
            MoveFlag::DoublePawnPush => 7,
        };

        let data = (flag_bits << 15) | (promo_bits << 12) | (to_bits << 6) | from_bits;
        Mossa { data }
    }
    
    pub fn da(&self) -> Casella {
        Casella::da_indice((self.data & 0x3F) as usize).unwrap()
    }

    pub fn a(&self) -> Casella {
        Casella::da_indice(((self.data >> 6) & 0x3F) as usize).unwrap()
    }

    pub fn promozione(&self) -> Option<Pezzo> {
        match (self.data >> 12) & 0x7 {
            1 => Some(Pezzo::Regina),
            2 => Some(Pezzo::Torre),
            3 => Some(Pezzo::Alfiere),
            4 => Some(Pezzo::Cavallo),
            _ => None,
        }
    }

    pub fn flag(&self) -> MoveFlag {
        match (self.data >> 15) & 0xF {
            1 => MoveFlag::EnPassant,
            2 => MoveFlag::CastleKingSide,
            3 => MoveFlag::CastleQueenSide,
            4 => MoveFlag::Capture,
            5 => MoveFlag::Promotion,
            6 => MoveFlag::PromotionCapture,
            7 => MoveFlag::DoublePawnPush,
            _ => MoveFlag::None,
        }
    }
    
    pub fn is_promotion(&self) -> bool {
        self.promozione().is_some()
    }
    
    pub fn to_u32(&self) -> u32 { self.data }
    pub fn from_u32(val: u32) -> Option<Self> { Some(Mossa { data: val }) }

    pub fn from_uci(uci: &str) -> Option<Self> {
        if uci.len() < 4 { return None; }
        let from = Casella::da_nome(&uci[0..2])?;
        let to = Casella::da_nome(&uci[2..4])?;
        let promo = if uci.len() > 4 {
            Pezzo::da_char(uci.chars().nth(4)?)
        } else {
            None
        };
        Some(Self::new(from, to, promo))
    }
}

// ===== SCACCHIERA E STATO =====

#[derive(Clone, Copy)]
pub struct CastlingRights {
    pub bianco_lato_re: bool,
    pub bianco_lato_regina: bool,
    pub nero_lato_re: bool,
    pub nero_lato_regina: bool,
}

#[derive(Clone, Copy)]
pub struct GameState {
    pub en_passant: Option<usize>,
    pub diritti_arrocco: u8,
    pub captured_piece: Option<Pezzo>,
    pub fifty_move: u8,
    pub hash: u64,
}

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
        for rank in (0..8).rev() {
            s.push_str(&format!("{} | ", rank + 1));
            for file in 0..8 {
                let sq = rank * 8 + file;
                let mut c = '.';
                if let Some((p, col)) = self.pezzo_su_casella(Casella(sq)) {
                    c = match p {
                        Pezzo::Pedone => 'p', Pezzo::Cavallo => 'n', Pezzo::Alfiere => 'b',
                        Pezzo::Torre => 'r', Pezzo::Regina => 'q', Pezzo::Re => 'k',
                    };
                    if col == Colore::Bianco { c = c.to_ascii_uppercase(); }
                }
                s.push(c);
                s.push(' ');
            }
            s.push_str("|\n");
        }
        s.push_str("    a b c d e f g h\n");
        write!(f, "{}", s)
    }
}

impl Scacchiera {
    pub fn nuova() -> Self {
        Self::da_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap()
    }

    pub fn da_fen(fen: &str) -> Result<Self, String> {
        let parts: Vec<&str> = fen.split_whitespace().collect();
        if parts.len() < 4 { return Err("FEN invalido".to_string()); }

        let mut pezzi = [0; 6];
        let mut colori = [0; 2];

        let rows: Vec<&str> = parts[0].split('/').collect();
        for (r, row) in rows.iter().enumerate() {
            let rank = 7 - r;
            let mut file = 0;
            for c in row.chars() {
                if let Some(digit) = c.to_digit(10) {
                    file += digit as usize;
                } else {
                    let p = Pezzo::da_char(c).ok_or("Pezzo invalido")?;
                    let col = if c.is_uppercase() { Colore::Bianco } else { Colore::Nero };
                    let sq = rank * 8 + file;
                    let bit = 1u64 << sq;
                    pezzi[p.indice()] |= bit;
                    colori[col.indice()] |= bit;
                    file += 1;
                }
            }
        }

        let turno = if parts[1] == "w" { Colore::Bianco } else { Colore::Nero };
        let mut castling = 0;
        if parts[2].contains('K') { castling |= 1; }
        if parts[2].contains('Q') { castling |= 2; }
        if parts[2].contains('k') { castling |= 4; }
        if parts[2].contains('q') { castling |= 8; }

        let ep = if parts[3] == "-" { None } else { Casella::da_nome(parts[3]).map(|c| c.indice()) };

        Ok(Scacchiera {
            pezzi,
            colori,
            turno,
            en_passant: ep,
            diritti_arrocco: castling,
            history: Vec::with_capacity(256),
        })
    }
    
    pub fn a_fen(&self) -> String { "FEN_DEBUG".to_string() }

    #[inline] pub fn turno(&self) -> Colore { self.turno }
    #[inline] pub fn bitboard_colore(&self, c: Colore) -> Bitboard { self.colori[c.indice()] }
    #[inline] pub fn bitboard_pezzo(&self, p: Pezzo) -> Bitboard { self.pezzi[p.indice()] }
    #[inline] pub fn bitboard_pezzo_colore(&self, p: Pezzo, c: Colore) -> Bitboard {
        self.pezzi[p.indice()] & self.colori[c.indice()]
    }
    #[inline] pub fn occupazione(&self) -> Bitboard { self.colori[0] | self.colori[1] }
    #[inline] pub fn en_passant_sq(&self) -> Option<usize> { self.en_passant } 
    #[inline] pub fn diritti_arrocco_struct(&self) -> CastlingRights {
        CastlingRights {
            bianco_lato_re: (self.diritti_arrocco & 1) != 0,
            bianco_lato_regina: (self.diritti_arrocco & 2) != 0,
            nero_lato_re: (self.diritti_arrocco & 4) != 0,
            nero_lato_regina: (self.diritti_arrocco & 8) != 0,
        }
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

    pub fn colore_attivo(&self) -> Colore { self.turno }
    
    pub fn hash(&self) -> u64 {
        let mut h = 0;
        h ^= self.colori[0];
        h ^= self.colori[1].wrapping_mul(31);
        h ^= (self.turno as u64).wrapping_mul(12345);
        if let Some(ep) = self.en_passant { h ^= (ep as u64).wrapping_mul(777); }
        h ^= (self.diritti_arrocco as u64).wrapping_mul(999);
        h
    }

    pub fn valore_materiale_totale(&self) -> i32 {
        let mut score = 0;
        for p in [Pezzo::Pedone, Pezzo::Cavallo, Pezzo::Alfiere, Pezzo::Torre, Pezzo::Regina] {
            let count_w = (self.bitboard_pezzo_colore(p, Colore::Bianco)).count_ones() as i32;
            let count_b = (self.bitboard_pezzo_colore(p, Colore::Nero)).count_ones() as i32;
            score += (count_w - count_b) * p.valore();
        }
        if self.turno == Colore::Nero { -score } else { score }
    }

    // ===== GESTIONE ATTACCHI E SCACCHI (Implementati qui per evitare dipendenze circolari) =====

    pub fn casella_attaccata(&self, sq: Casella, da_colore: Colore) -> bool {
        let s = sq.indice();
        let occ = self.occupazione();

        // Pedoni: Nota che passiamo da_colore.opposto() a pawn_attacks perché
        // pawn_attacks(sq, colore) restituisce dove il pedone in 'sq' attacca.
        // Se un pedone avversario in X attacca 'sq', allora 'sq' attaccherebbe X se fosse un pedone del colore opposto.
        let pawns = self.bitboard_pezzo_colore(Pezzo::Pedone, da_colore);
        if (pawn_attacks(s, da_colore.opposto()) & pawns) != 0 { return true; }

        let knights = self.bitboard_pezzo_colore(Pezzo::Cavallo, da_colore);
        if (knight_attacks(s) & knights) != 0 { return true; }

        let kings = self.bitboard_pezzo_colore(Pezzo::Re, da_colore);
        if (king_attacks(s) & kings) != 0 { return true; }

        let rooks = self.bitboard_pezzo_colore(Pezzo::Torre, da_colore);
        let bishops = self.bitboard_pezzo_colore(Pezzo::Alfiere, da_colore);
        let queens = self.bitboard_pezzo_colore(Pezzo::Regina, da_colore);

        if (rook_attacks(s, occ) & (rooks | queens)) != 0 { return true; }
        if (bishop_attacks(s, occ) & (bishops | queens)) != 0 { return true; }

        false
    }

    pub fn re_in_scacco(&self, colore: Colore) -> bool {
        let k_bb = self.bitboard_pezzo_colore(Pezzo::Re, colore);
        if k_bb == 0 { return true; } // Should not happen
        let sq = k_bb.trailing_zeros() as usize;
        self.casella_attaccata(Casella(sq), colore.opposto())
    }

    pub fn sotto_scacco(&self, colore: Colore) -> bool {
        self.re_in_scacco(colore)
    }

    pub fn attacchi_alfiere(&self, sq: Casella, occ: Bitboard) -> Bitboard {
        bishop_attacks(sq.indice(), occ)
    }

    pub fn attacchi_torre(&self, sq: Casella, occ: Bitboard) -> Bitboard {
        rook_attacks(sq.indice(), occ)
    }
    
    // ===== ESECUZIONE MOSSA =====

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

        // Salva stato
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

        let state = GameState {
            en_passant: self.en_passant,
            diritti_arrocco: self.diritti_arrocco,
            captured_piece: captured,
            fifty_move: 0,
            hash: self.hash(),
        };
        self.history.push(state);

        // Rimuovi da from
        self.pezzi[piece.indice()] &= !from_bit;
        self.colori[us.indice()] &= !from_bit;

        // Gestione Cattura
        if mossa.flag().is_capture() {
            if mossa.flag() == MoveFlag::EnPassant {
                let cap_sq = if us == Colore::Bianco { to - 8 } else { to + 8 };
                let cap_bit = 1u64 << cap_sq;
                self.pezzi[Pezzo::Pedone.indice()] &= !cap_bit;
                self.colori[them.indice()] &= !cap_bit;
            } else {
                if let Some(cap_p) = captured {
                    self.pezzi[cap_p.indice()] &= !to_bit;
                    self.colori[them.indice()] &= !to_bit;
                }
            }
        }

        // Aggiungi a to
        if let Some(promo) = mossa.promozione() {
            self.pezzi[promo.indice()] |= to_bit;
        } else {
            self.pezzi[piece.indice()] |= to_bit;
        }
        self.colori[us.indice()] |= to_bit;

        // Arrocco
        if mossa.flag() == MoveFlag::CastleKingSide {
            let (r_from, r_to) = if us == Colore::Bianco { (7, 5) } else { (63, 61) };
            self.move_rook_raw(r_from, r_to, us);
        } else if mossa.flag() == MoveFlag::CastleQueenSide {
            let (r_from, r_to) = if us == Colore::Bianco { (0, 3) } else { (56, 59) };
            self.move_rook_raw(r_from, r_to, us);
        }

        // En Passant update
        if mossa.flag() == MoveFlag::DoublePawnPush {
            let ep_sq = if us == Colore::Bianco { to - 8 } else { to + 8 };
            self.en_passant = Some(ep_sq);
        } else {
            self.en_passant = None;
        }

        // Arrocco rights
        if piece == Pezzo::Re {
            if us == Colore::Bianco { self.diritti_arrocco &= !3; } else { self.diritti_arrocco &= !12; }
        }
        
        let corners = [0, 7, 56, 63];
        if from == 0 || to == 0 { self.diritti_arrocco &= !2; } // WQ
        if from == 7 || to == 7 { self.diritti_arrocco &= !1; } // WK
        if from == 56 || to == 56 { self.diritti_arrocco &= !8; } // BQ
        if from == 63 || to == 63 { self.diritti_arrocco &= !4; } // BK

        self.turno = them;
        true
    }

    fn move_rook_raw(&mut self, from: usize, to: usize, col: Colore) {
        let f_bit = 1u64 << from;
        let t_bit = 1u64 << to;
        self.pezzi[Pezzo::Torre.indice()] &= !f_bit;
        self.pezzi[Pezzo::Torre.indice()] |= t_bit;
        self.colori[col.indice()] &= !f_bit;
        self.colori[col.indice()] |= t_bit;
    }

    pub fn annulla_mossa(&mut self) {
        // Safe panic: se chiamato, avvisa che search deve usare clone
        panic!("annulla_mossa called! Use clone() strategy in search.rs");
    }
}