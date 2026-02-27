use std::fmt;
use crate::zobrist::ZobristKeys;
use crate::nnue::{Accumulator, LunaNNUE};

pub type Bitboard = u64;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Colore { Bianco = 0, Nero = 1 }

impl Colore {
    #[inline(always)]
    pub fn opposto(&self) -> Colore {
        match self {
            Colore::Bianco => Colore::Nero,
            Colore::Nero => Colore::Bianco,
        }
    }

    #[inline(always)]
    pub fn indice(&self) -> usize { *self as usize }

    pub fn from_index(i: usize) -> Self {
        if i == 0 { Colore::Bianco } else { Colore::Nero }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Pezzo {
    Pedone = 0,
    Cavallo = 1,
    Alfiere = 2,
    Torre = 3,
    Regina = 4,
    Re = 5,
}

impl Pezzo {
    #[inline(always)]
    pub fn indice(&self) -> usize { *self as usize }

    #[inline(always)]
    pub fn valore(&self) -> i32 {
        match self {
            Pezzo::Pedone   => 100,
            Pezzo::Cavallo  => 320,
            Pezzo::Alfiere  => 330,
            Pezzo::Torre    => 500,
            Pezzo::Regina   => 900,
            Pezzo::Re       => 20000,
        }
    }

    pub fn from_index(i: usize) -> Pezzo {
        match i {
            0 => Pezzo::Pedone,
            1 => Pezzo::Cavallo,
            2 => Pezzo::Alfiere,
            3 => Pezzo::Torre,
            4 => Pezzo::Regina,
            5 => Pezzo::Re,
            _ => Pezzo::Pedone,
        }
    }

    pub fn from_char(c: char) -> Option<(Colore, Pezzo)> {
        match c {
            'P' => Some((Colore::Bianco, Pezzo::Pedone)),
            'N' => Some((Colore::Bianco, Pezzo::Cavallo)),
            'B' => Some((Colore::Bianco, Pezzo::Alfiere)),
            'R' => Some((Colore::Bianco, Pezzo::Torre)),
            'Q' => Some((Colore::Bianco, Pezzo::Regina)),
            'K' => Some((Colore::Bianco, Pezzo::Re)),
            'p' => Some((Colore::Nero, Pezzo::Pedone)),
            'n' => Some((Colore::Nero, Pezzo::Cavallo)),
            'b' => Some((Colore::Nero, Pezzo::Alfiere)),
            'r' => Some((Colore::Nero, Pezzo::Torre)),
            'q' => Some((Colore::Nero, Pezzo::Regina)),
            'k' => Some((Colore::Nero, Pezzo::Re)),
            _ => None,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum MoveFlag {
    None = 0,
    EnPassant = 1,
    Castle = 2,
    Promotion = 3,
    Capture = 4,
    DoublePawnPush = 5,
    PromotionCapture = 6,
}

impl MoveFlag {
    #[inline(always)]
    pub fn is_capture(&self) -> bool {
        matches!(self, MoveFlag::Capture | MoveFlag::EnPassant | MoveFlag::PromotionCapture)
    }

    #[inline(always)]
    pub fn is_promotion(&self) -> bool {
        matches!(self, MoveFlag::Promotion | MoveFlag::PromotionCapture)
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Mossa {
    pub data: u16,
    pub promozione: u8,
}

impl Mossa {
    pub fn new(from: usize, to: usize, flag: MoveFlag, promo_piece: Option<Pezzo>) -> Self {
        let promo_val = promo_piece.map(|p| p.indice() as u8).unwrap_or(6);
        Mossa {
            data: (from as u16) | ((to as u16) << 6) | ((flag as u8 as u16) << 12),
            promozione: promo_val,
        }
    }

    #[inline(always)] pub fn da(&self) -> usize { (self.data & 0x3F) as usize }
    #[inline(always)] pub fn a(&self) -> usize  { ((self.data >> 6) & 0x3F) as usize }

    #[inline(always)]
    pub fn move_flag(&self) -> MoveFlag {
        match (self.data >> 12) as u8 {
            1 => MoveFlag::EnPassant,
            2 => MoveFlag::Castle,
            3 => MoveFlag::Promotion,
            4 => MoveFlag::Capture,
            5 => MoveFlag::DoublePawnPush,
            6 => MoveFlag::PromotionCapture,
            _ => MoveFlag::None,
        }
    }

    #[inline(always)] pub fn is_cattura(&self) -> bool { self.move_flag().is_capture() }
    #[inline(always)] pub fn is_promozione(&self) -> bool { self.move_flag().is_promotion() }

    pub fn pezzo_promosso(&self) -> Option<Pezzo> {
        if self.promozione < 6 {
            Some(Pezzo::from_index(self.promozione as usize))
        } else { None }
    }

    pub fn null() -> Self {
        Mossa { data: 0, promozione: 6 }
    }

    pub fn is_null(&self) -> bool { self.data == 0 }

    pub fn from_data(data: u16) -> Self {
        Mossa { data, promozione: 6 }
    }

    pub fn to_uci(&self) -> String {
        if self.is_null() {
            return "0000".to_string();
        }
        let f = self.da();
        let t = self.a();
        let mut s = format!(
            "{}{}{}{}",
            (b'a' + (f % 8) as u8) as char,
            (b'1' + (f / 8) as u8) as char,
            (b'a' + (t % 8) as u8) as char,
            (b'1' + (t / 8) as u8) as char
        );
        if let Some(p) = self.pezzo_promosso() {
            s.push(match p {
                Pezzo::Cavallo => 'n',
                Pezzo::Alfiere => 'b',
                Pezzo::Torre   => 'r',
                _ => 'q',
            });
        }
        s
    }
}

#[derive(Clone)]
pub struct UndoData {
    pub hash: u64,
    pub ep_square: Option<usize>,
    pub castling: u8,
    pub mezze_mosse: u32,
    pub cattura_p: Option<usize>,
    pub acc: Accumulator, 
}

const CASTLING_UPDATE: [u8; 64] = [
    13,15,15,15,12,15,15,14,15,15,15,15,15,15,15,15,
    15,15,15,15,15,15,15,15,15,15,15,15,15,15,15,15,
    15,15,15,15,15,15,15,15,15,15,15,15,15,15,15,15,
    15,15,15,15,15,15,15,15, 7,15,15,15, 3,15,15,11,
];

#[derive(Clone)]
pub struct Scacchiera {
    pub pezzi: [Bitboard; 6],
    pub colori: [Bitboard; 2],
    pub turno: Colore,
    pub ep_square: Option<usize>,
    pub castling: u8,
    pub hash: u64,
    pub mezze_mosse: u32,
    pub ply: u32,
    pub history: Vec<UndoData>,
    pub accumulator: Accumulator, 
}

impl Scacchiera {
    pub fn from_fen(fen: &str, z: &ZobristKeys) -> Self {
        let mut pezzi = [0; 6];
        let mut colori = [0; 2];

        let parts: Vec<&str> = fen.split_whitespace().collect();
        let mut rank = 7;
        let mut file = 0;

        for c in parts[0].chars() {
            if c == '/' {
                rank -= 1;
                file = 0;
            }
            else if let Some(d) = c.to_digit(10) {
                file += d as usize;
            }
            else if let Some((col, p)) = Pezzo::from_char(c) {
                let sq = rank * 8 + file;
                pezzi[p.indice()] |= 1 << sq;
                colori[col.indice()] |= 1 << sq;
                file += 1;
            }
        }

        let turno = if parts[1] == "b" { Colore::Nero } else { Colore::Bianco };

        let mut cast = 0;
        if parts.len() > 2 {
            if parts[2].contains('K') { cast |= 1; }
            if parts[2].contains('Q') { cast |= 2; }
            if parts[2].contains('k') { cast |= 4; }
            if parts[2].contains('q') { cast |= 8; }
        }

        let ep_square = if parts.len() > 3 && parts[3] != "-" {
            let b = parts[3].as_bytes();
            Some(((b[1] - b'1') * 8 + (b[0] - b'a')) as usize)
        } else { None };

        let mezze = if parts.len() > 4 {
            parts[4].parse::<u32>().unwrap_or(0)
        } else { 0 };

        let mut board = Scacchiera {
            pezzi,
            colori,
            turno,
            ep_square,
            castling: cast,
            hash: 0,
            mezze_mosse: mezze,
            ply: 0,
            history: Vec::with_capacity(256),
            accumulator: Accumulator::new(),
        };

        board.hash = board.compute_hash(z);
        board
    }

    pub fn new_iniziale(z: &ZobristKeys) -> Self {
        Self::from_fen(
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            z,
        )
    }
    
    pub fn inizializza_nnue(&mut self, nnue: &LunaNNUE) {
        self.accumulator = Accumulator::new();
        for sq in 0..64 {
            if let Some((colore, pezzo)) = self.pezzo_e_colore_in(sq) {
                let idx = LunaNNUE::get_feature_index(colore, pezzo, sq);
                nnue.update_add(&mut self.accumulator, idx);
            }
        }
    }

    pub fn is_repetition(&self) -> bool {
        let mut count = 0;
        for undo in self.history.iter().rev().take(self.mezze_mosse as usize) {
            if undo.hash == self.hash {
                count += 1;
                if count >= 2 {
                    return true;
                }
            }
        }
        false
    }

    // ---------------------------
    //  HASHING COMPLETO INIZIALE
    // ---------------------------
    pub fn compute_hash(&self, z: &ZobristKeys) -> u64 {
        let mut h = 0;

        for col in 0..2 {
            for p in 0..6 {
                let mut bb = self.pezzi[p] & self.colori[col];
                while bb != 0 {
                    let sq = bb.trailing_zeros() as usize;
                    h ^= z.pezzi[col][p][sq];
                    bb &= bb - 1;
                }
            }
        }

        if self.turno == Colore::Nero {
            h ^= z.turno;
        }

        h ^= z.castling[self.castling as usize];

        if let Some(sq) = self.ep_square {
            h ^= z.ep[sq % 8];
        }

        h
    }

    #[inline(always)]
    pub fn occupazione(&self) -> Bitboard {
        self.colori[0] | self.colori[1]
    }

    #[inline(always)]
    pub fn pezzo_in(&self, sq: usize) -> Option<usize> {
        let mask = 1 << sq;
        if (self.occupazione() & mask) == 0 {
            return None;
        }
        for p in 0..6 {
            if (self.pezzi[p] & mask) != 0 {
                return Some(p);
            }
        }
        None
    }

    #[inline(always)]
    pub fn colore_in(&self, sq: usize) -> Option<Colore> {
        if (self.colori[Colore::Bianco.indice()] & (1 << sq)) != 0 {
            Some(Colore::Bianco)
        } else if (self.colori[Colore::Nero.indice()] & (1 << sq)) != 0 {
            Some(Colore::Nero)
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn pezzo_e_colore_in(&self, sq: usize) -> Option<(Colore, Pezzo)> {
        let c = self.colore_in(sq)?;
        let p = self.pezzo_in(sq)?;
        Some((c, Pezzo::from_index(p)))
    }

    // --------------------------
    //  RE IN SCACCO E LEGALITÀ
    // --------------------------
    
    pub fn in_scacco(&self) -> bool {
        self.re_in_scacco(self.turno)
    }

    #[inline(always)]
    pub fn mossa_precedente_legale(&self) -> bool {
        !self.re_in_scacco(self.turno.opposto())
    }

    pub fn re_in_scacco(&self, col: Colore) -> bool {
        let bb = self.pezzi[Pezzo::Re.indice()] & self.colori[col.indice()];
        if bb == 0 {
            return false;
        }
        let sq = bb.trailing_zeros() as usize;
        crate::attacks::square_attacked(self, sq, col.opposto())
    }

    // -----------------------------------------
    //                 MAKE MOVE
    // -----------------------------------------
    pub fn esegui_mossa(&mut self, m: &Mossa, z: &ZobristKeys, nnue_opt: Option<&LunaNNUE>) -> UndoData {
        let from = m.da();
        let to = m.a();
        let flag = m.move_flag();

        let us = self.turno.indice();
        let them = 1 - us;
        let colore_mosso = self.turno;

        let moved_p = self.pezzo_in(from).unwrap_or(0);
        let moved_pezzo_enum = Pezzo::from_index(moved_p);

        let cattura_p = Self::cattura_piece(self, m);

        let undo = UndoData {
            hash: self.hash,
            ep_square: self.ep_square,
            castling: self.castling,
            mezze_mosse: self.mezze_mosse,
            cattura_p,
            acc: self.accumulator.clone(),
        };

        if let Some(nnue) = nnue_opt {
            let idx_from = LunaNNUE::get_feature_index(colore_mosso, moved_pezzo_enum, from);
            nnue.update_remove(&mut self.accumulator, idx_from);
            
            if let Some(cap) = cattura_p {
                let cap_sq = if flag == MoveFlag::EnPassant { if us == 0 { to - 8 } else { to + 8 } } else { to };
                let idx_cap = LunaNNUE::get_feature_index(colore_mosso.opposto(), Pezzo::from_index(cap), cap_sq);
                nnue.update_remove(&mut self.accumulator, idx_cap);
            }
        }

        if let Some(sq) = self.ep_square {
            self.hash ^= z.ep[sq % 8];
        }
        self.ep_square = None;

        self.pezzi[moved_p] &= !(1 << from);
        self.colori[us] &= !(1 << from);
        self.hash ^= z.pezzi[us][moved_p][from];

        if flag == MoveFlag::EnPassant {
            let cap_sq = if us == 0 { to - 8 } else { to + 8 };
            self.pezzi[Pezzo::Pedone.indice()] &= !(1 << cap_sq);
            self.colori[them] &= !(1 << cap_sq);
            self.hash ^= z.pezzi[them][Pezzo::Pedone.indice()][cap_sq];
        }
        else if let Some(cap) = undo.cattura_p {
            self.pezzi[cap] &= !(1 << to);
            self.colori[them] &= !(1 << to);
            self.hash ^= z.pezzi[them][cap][to];
        }

        let final_p = if flag.is_promotion() {
            m.pezzo_promosso().unwrap().indice()
        } else {
            moved_p
        };

        self.pezzi[final_p] |= 1 << to;
        self.colori[us] |= 1 << to;
        self.hash ^= z.pezzi[us][final_p][to];

        if let Some(nnue) = nnue_opt {
            let final_pezzo_enum = Pezzo::from_index(final_p);
            let idx_to = LunaNNUE::get_feature_index(colore_mosso, final_pezzo_enum, to);
            nnue.update_add(&mut self.accumulator, idx_to);
        }

        if flag == MoveFlag::Castle {
            let (r_from, r_to) = match to {
                6   => (7, 5),
                2   => (0, 3),
                62  => (63, 61),
                58  => (56, 59),
                _ => (0, 0),
            };
            self.pezzi[Pezzo::Torre.indice()] ^= (1 << r_from) | (1 << r_to);
            self.colori[us] ^= (1 << r_from) | (1 << r_to);
            self.hash ^= z.pezzi[us][Pezzo::Torre.indice()][r_from]
                       ^ z.pezzi[us][Pezzo::Torre.indice()][r_to];
            
            if let Some(nnue) = nnue_opt {
                let idx_t_from = LunaNNUE::get_feature_index(colore_mosso, Pezzo::Torre, r_from);
                let idx_t_to = LunaNNUE::get_feature_index(colore_mosso, Pezzo::Torre, r_to);
                nnue.update_remove(&mut self.accumulator, idx_t_from);
                nnue.update_add(&mut self.accumulator, idx_t_to);
            }
        }

        if flag == MoveFlag::DoublePawnPush {
            let ep = if us == 0 { to - 8 } else { to + 8 };
            self.ep_square = Some(ep);
            self.hash ^= z.ep[ep % 8];
        }

        self.hash ^= z.castling[self.castling as usize];
        self.castling &= CASTLING_UPDATE[from] & CASTLING_UPDATE[to];
        self.hash ^= z.castling[self.castling as usize];

        self.hash ^= z.turno;
        self.turno = self.turno.opposto();

        if moved_p == Pezzo::Pedone.indice() || flag.is_capture() {
            self.mezze_mosse = 0;
        } else {
            self.mezze_mosse += 1;
        }

        self.ply += 1;
        self.history.push(undo.clone());
        undo
    }

    #[inline(always)]
    fn cattura_piece(&self, m: &Mossa) -> Option<usize> {
        let to = m.a();
        if m.move_flag().is_capture() {
            if m.move_flag() == MoveFlag::EnPassant {
                return Some(Pezzo::Pedone.indice());
            }
            return self.pezzo_in(to);
        }
        None
    }

    // -----------------------------------------
    //                 UNDO MOVE
    // -----------------------------------------
    pub fn annulla_mossa(&mut self, m: &Mossa, undo: UndoData, z: &ZobristKeys) {
        let from = m.da();
        let to = m.a();
        let flag = m.move_flag();

        let them = self.turno.indice();
        let us = 1 - them;

        self.history.pop();
        self.ply -= 1;

        self.hash ^= z.turno;
        self.turno = Colore::from_index(us);

        if let Some(sq) = self.ep_square {
            self.hash ^= z.ep[sq % 8];
        }
        self.ep_square = undo.ep_square;
        if let Some(sq) = undo.ep_square {
            self.hash ^= z.ep[sq % 8];
        }

        self.hash ^= z.castling[self.castling as usize];
        self.castling = undo.castling;
        self.hash ^= z.castling[self.castling as usize];

        self.mezze_mosse = undo.mezze_mosse;

        let final_p = if flag.is_promotion() {
            m.pezzo_promosso().unwrap().indice()
        } else {
            self.pezzo_in(to).unwrap_or(0)
        };

        self.pezzi[final_p] &= !(1 << to);
        self.colori[us] &= !(1 << to);
        self.hash ^= z.pezzi[us][final_p][to];

        let moved_p = self.pezzo_in(to).unwrap_or(final_p);
        let original_p = if flag.is_promotion() { Pezzo::Pedone.indice() } else { moved_p };

        self.pezzi[original_p] |= 1 << from;
        self.colori[us] |= 1 << from;
        self.hash ^= z.pezzi[us][original_p][from];

        if flag == MoveFlag::EnPassant {
            let cap_sq = if us == 0 { to - 8 } else { to + 8 };
            self.pezzi[Pezzo::Pedone.indice()] |= 1 << cap_sq;
            self.colori[them] |= 1 << cap_sq;
            self.hash ^= z.pezzi[them][Pezzo::Pedone.indice()][cap_sq];
        }
        else if let Some(cap) = undo.cattura_p {
            self.pezzi[cap] |= 1 << to;
            self.colori[them] |= 1 << to;
            self.hash ^= z.pezzi[them][cap][to];
        }

        if flag == MoveFlag::Castle {
            let (r_from, r_to) = match to {
                6   => (7, 5),
                2   => (0, 3),
                62  => (63, 61),
                58  => (56, 59),
                _ => (0, 0),
            };
            self.pezzi[Pezzo::Torre.indice()] ^= (1 << r_from) | (1 << r_to);
            self.colori[us] ^= (1 << r_from) | (1 << r_to);
            self.hash ^= z.pezzi[us][Pezzo::Torre.indice()][r_from]
                       ^ z.pezzi[us][Pezzo::Torre.indice()][r_to];
        }

        self.hash = undo.hash;
        self.accumulator = undo.acc; 
    }

    // -----------------------------------------
    //           NULL MOVE PRUNING SUPPORT
    // -----------------------------------------
    pub fn fai_mossa_nulla(&mut self, z: &ZobristKeys) -> UndoData {
        let undo = UndoData {
            hash: self.hash,
            ep_square: self.ep_square,
            castling: self.castling,
            mezze_mosse: self.mezze_mosse,
            cattura_p: None,
            acc: self.accumulator.clone(),
        };

        if let Some(sq) = self.ep_square {
            self.hash ^= z.ep[sq % 8];
        }
        self.ep_square = None;

        self.hash ^= z.turno;
        self.turno = self.turno.opposto();

        self.history.push(undo.clone());
        self.ply += 1;

        undo
    }

    pub fn annulla_mossa_nulla(&mut self, undo: UndoData, z: &ZobristKeys) {
        self.history.pop();
        self.ply -= 1;

        self.hash ^= z.turno;
        self.turno = self.turno.opposto();

        if let Some(sq) = self.ep_square {
            self.hash ^= z.ep[sq % 8];
        }
        self.ep_square = undo.ep_square;
        if let Some(sq) = undo.ep_square {
            self.hash ^= z.ep[sq % 8];
        }

        self.castling = undo.castling;
        self.mezze_mosse = undo.mezze_mosse;

        self.hash = undo.hash;
        self.accumulator = undo.acc;
    }

    // ----------------------------
    //   MOVE GENERATION HELPERS
    // ----------------------------
    pub fn genera_mosse(&self) -> Vec<Mossa> {
        crate::movegen::genera_mosse(self)
    }

    pub fn genera_mosse_legali(&mut self, z: &ZobristKeys) -> Vec<Mossa> {
        let mosse = crate::movegen::genera_mosse(self);
        let mut legali = Vec::with_capacity(mosse.len());
        for m in mosse {
            let undo = self.esegui_mossa(&m, z, None);
            if !self.re_in_scacco(self.turno.opposto()) {
                legali.push(m);
            }
            self.annulla_mossa(&m, undo, z);
        }
        legali
    }

    pub fn genera_catture_legali(&mut self, z: &ZobristKeys) -> Vec<Mossa> {
        let mosse = crate::movegen::genera_catture(self);
        let mut legali = Vec::with_capacity(mosse.len());
        for m in mosse {
            let undo = self.esegui_mossa(&m, z, None);
            if !self.re_in_scacco(self.turno.opposto()) {
                legali.push(m);
            }
            self.annulla_mossa(&m, undo, z);
        }
        legali
    }

    pub fn genera_silenziose_legali(&mut self, z: &ZobristKeys) -> Vec<Mossa> {
        let mosse = crate::movegen::genera_silenziose(self);
        let mut legali = Vec::with_capacity(mosse.len());
        for m in mosse {
            let undo = self.esegui_mossa(&m, z, None);
            if !self.re_in_scacco(self.turno.opposto()) {
                legali.push(m);
            }
            self.annulla_mossa(&m, undo, z);
        }
        legali
    }

    // ----------------------------
    //           FEN
    // ----------------------------
    pub fn to_fen(&self) -> String {
        let mut fen = String::new();

        for rank in (0..8).rev() {
            let mut empty = 0;
            for file in 0..8 {
                let sq = rank * 8 + file;
                if let Some((col, p)) = self.pezzo_e_colore_in(sq) {
                    if empty > 0 {
                        fen.push_str(&empty.to_string());
                        empty = 0;
                    }
                    let mut c = match p {
                        Pezzo::Pedone => 'p',
                        Pezzo::Cavallo => 'n',
                        Pezzo::Alfiere => 'b',
                        Pezzo::Torre => 'r',
                        Pezzo::Regina => 'q',
                        Pezzo::Re => 'k',
                    };
                    if col == Colore::Bianco {
                        c = c.to_ascii_uppercase();
                    }
                    fen.push(c);
                } else {
                    empty += 1;
                }
            }
            if empty > 0 {
                fen.push_str(&empty.to_string());
            }
            if rank > 0 {
                fen.push('/');
            }
        }

        fen.push(' ');
        fen.push(if self.turno == Colore::Bianco { 'w' } else { 'b' });
        fen.push(' ');

        if self.castling == 0 {
            fen.push('-');
        } else {
            if (self.castling & 1) != 0 { fen.push('K'); }
            if (self.castling & 2) != 0 { fen.push('Q'); }
            if (self.castling & 4) != 0 { fen.push('k'); }
            if (self.castling & 8) != 0 { fen.push('q'); }
        }

        fen.push(' ');
        if let Some(sq) = self.ep_square {
            let f = (sq % 8) as u8;
            let r = (sq / 8) as u8;
            fen.push((b'a' + f) as char);
            fen.push((b'1' + r) as char);
        } else {
            fen.push('-');
        }

        fen.push_str(&format!(" {} {}", self.mezze_mosse, (self.ply / 2) + 1));
        fen
    }
}

impl fmt::Display for Scacchiera {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_fen())
    }
}