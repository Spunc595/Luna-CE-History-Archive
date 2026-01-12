use std::fmt;
use crate::zobrist::ZobristKeys;

pub type Bitboard = u64;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Colore { Bianco = 0, Nero = 1 }

impl Colore {
    #[inline(always)]
    pub fn opposto(&self) -> Colore {
        match self { Colore::Bianco => Colore::Nero, Colore::Nero => Colore::Bianco }
    }
    #[inline(always)]
    pub fn indice(&self) -> usize { *self as usize }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Pezzo { Pedone = 0, Cavallo = 1, Alfiere = 2, Torre = 3, Regina = 4, Re = 5 }

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum MoveFlag { None = 0, EnPassant = 1, Castle = 2, Promotion = 3, Capture = 4, DoublePawnPush = 5 }

impl MoveFlag {
    #[inline(always)]
    pub fn is_capture(&self) -> bool { matches!(self, MoveFlag::Capture | MoveFlag::EnPassant) }
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Mossa { pub data: u16 }

impl Mossa {
    pub fn new(from: usize, to: usize, flag: MoveFlag) -> Self {
        Mossa { data: (from as u16) | ((to as u16) << 6) | ((flag as u16) << 12) }
    }
    #[inline(always)] pub fn da(&self) -> usize { (self.data & 0x3F) as usize }
    #[inline(always)] pub fn a(&self) -> usize { ((self.data >> 6) & 0x3F) as usize }
    #[inline(always)] pub fn move_flag(&self) -> MoveFlag {
        match (self.data >> 12) & 0x7 {
            1 => MoveFlag::EnPassant, 2 => MoveFlag::Castle, 3 => MoveFlag::Promotion,
            4 => MoveFlag::Capture, 5 => MoveFlag::DoublePawnPush, _ => MoveFlag::None,
        }
    }

    pub fn to_uci(&self) -> String {
        let files = ['a','b','c','d','e','f','g','h'];
        let ranks = ['1','2','3','4','5','6','7','8'];
        let from = self.da();
        let to = self.a();
        let mut s = format!("{}{}{}{}", files[from % 8], ranks[from / 8], files[to % 8], ranks[to / 8]);
        if self.move_flag() == MoveFlag::Promotion { s.push('q'); }
        s
    }
}

impl fmt::Display for Mossa {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "{}", self.to_uci()) }
}

const CASTLING_RIGHTS_UPDATE: [u8; 64] = {
    let mut r = [15u8; 64];
    r[0] = 13; r[7] = 14; r[56] = 7; r[63] = 11; r[4] = 12; r[60] = 3; r
};

#[derive(Clone)]
pub struct Scacchiera {
    pub pezzi: [Bitboard; 6],
    pub colori: [Bitboard; 2],
    pub turno: Colore,
    pub ep_square: Option<usize>,
    pub diritti_arrocco: u8,
    pub pst_val: i32,
    pub hash: u64,
}

impl Scacchiera {
    pub fn new() -> Self {
        let z = ZobristKeys::init();
        Self::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", &z)
    }

    pub fn from_fen(fen: &str, z: &ZobristKeys) -> Self {
        let mut pezzi = [0; 6];
        let mut colori = [0; 2];
        let mut pst_val = 0;
        let parts: Vec<&str> = fen.split_whitespace().collect();
        let board_str = parts[0];
        let mut rank = 7;
        let mut file = 0;

        for c in board_str.chars() {
            if c == '/' { rank -= 1; file = 0; }
            else if let Some(d) = c.to_digit(10) { file += d as usize; }
            else {
                let sq = (rank * 8 + file) as usize;
                let (color, p_idx) = match c {
                    'P'=> (0,0), 'N'=>(0,1), 'B'=>(0,2), 'R'=>(0,3), 'Q'=>(0,4), 'K'=>(0,5),
                    'p'=>(1,0), 'n'=>(1,1), 'b'=>(1,2), 'r'=>(1,3), 'q'=>(1,4), 'k'=>(1,5),
                    _ => continue,
                };
                pezzi[p_idx] |= 1 << sq;
                colori[color] |= 1 << sq;
                let v = crate::evaluation::get_pst(p_idx, sq, color == 0);
                pst_val += if color == 0 { v } else { -v };
                file += 1;
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

        let ep_sq = if parts.len() > 3 && parts[3] != "-" {
            let f = (parts[3].as_bytes()[0] - b'a') as usize;
            let r = (parts[3].as_bytes()[1] - b'1') as usize;
            Some(r * 8 + f)
        } else { None };

        let mut s = Scacchiera { pezzi, colori, turno, ep_square: ep_sq, diritti_arrocco: diritti, pst_val, hash: 0 };
        s.hash = s.get_hash(z);
        s
    }

    #[inline(always)]
    pub fn occupazione(&self) -> Bitboard { self.colori[0] | self.colori[1] }

    #[inline(always)]
    pub fn get_hash(&self, z: &ZobristKeys) -> u64 {
        let mut h = if self.turno == Colore::Nero { z.turno } else { 0 };
        for p in 0..6 {
            let mut bb = self.pezzi[p];
            while bb != 0 {
                let sq = bb.trailing_zeros() as usize;
                let color = if (self.colori[0] & (1 << sq)) != 0 { 0 } else { 1 };
                h ^= z.pezzi[color][p][sq];
                bb &= bb - 1;
            }
        }
        h ^= z.arrocco[self.diritti_arrocco as usize];
        if let Some(ep) = self.ep_square { h ^= z.ep_file[ep % 8]; }
        h
    }

    #[inline(always)]
    pub fn genera_mosse(&self) -> Vec<Mossa> { crate::movegen::genera_mosse(self) }

    pub fn re_in_scacco(&self, c: Colore) -> bool {
        let k_bb = self.pezzi[5] & self.colori[c.indice()];
        if k_bb == 0 { return false; }
        self.casella_attaccata(k_bb.trailing_zeros() as usize, c.opposto())
    }

    pub fn casella_attaccata(&self, sq: usize, by: Colore) -> bool {
        let occ = self.occupazione();
        let nemici = self.colori[by.indice()];
        if (crate::attacks::pawn_attacks(sq, by.opposto()) & self.pezzi[0] & nemici) != 0 { return true; }
        if (crate::attacks::knight_attacks(sq) & self.pezzi[1] & nemici) != 0 { return true; }
        if (crate::attacks::king_attacks(sq) & self.pezzi[5] & nemici) != 0 { return true; }
        let bq = (self.pezzi[2] | self.pezzi[4]) & nemici;
        if bq != 0 && (crate::attacks::bishop_attacks(sq, occ) & bq) != 0 { return true; }
        let rq = (self.pezzi[3] | self.pezzi[4]) & nemici;
        if rq != 0 && (crate::attacks::rook_attacks(sq, occ) & rq) != 0 { return true; }
        false
    }

    pub fn get_attacks_for_piece(&self, p: Pezzo, sq: usize, occ: Bitboard) -> Bitboard {
        match p {
            Pezzo::Cavallo => crate::attacks::knight_attacks(sq),
            Pezzo::Re => crate::attacks::king_attacks(sq),
            Pezzo::Alfiere => crate::attacks::bishop_attacks(sq, occ),
            Pezzo::Torre => crate::attacks::rook_attacks(sq, occ),
            Pezzo::Regina => crate::attacks::bishop_attacks(sq, occ) | crate::attacks::rook_attacks(sq, occ),
            _ => 0,
        }
    }

    #[inline(always)]
    pub fn esegui_mossa(&mut self, m: &Mossa, z: &ZobristKeys) -> bool {
        let us = self.turno.indice();
        let them = self.turno.opposto().indice();
        let from = m.da();
        let to = m.a();
        let flag = m.move_flag();
        
        let mut moved_piece = 6;
        for p in 0..6 { if (self.pezzi[p] & (1 << from)) != 0 { moved_piece = p; break; } }
        if moved_piece == 6 { return false; }

        self.hash ^= z.pezzi[us][moved_piece][from];
        if let Some(ep) = self.ep_square { self.hash ^= z.ep_file[ep % 8]; }
        self.hash ^= z.arrocco[self.diritti_arrocco as usize];

        self.pezzi[moved_piece] &= !(1 << from);
        self.colori[us] &= !(1 << from);
        self.pst_val -= if us == 0 { crate::evaluation::get_pst(moved_piece, from, true) } 
                        else { -crate::evaluation::get_pst(moved_piece, from, false) };

        if flag.is_capture() {
            let cap_sq = if flag == MoveFlag::EnPassant { if us == 0 { to - 8 } else { to + 8 } } else { to };
            for p in 0..6 {
                if (self.pezzi[p] & (1 << cap_sq)) != 0 {
                    self.pezzi[p] &= !(1 << cap_sq);
                    self.colori[them] &= !(1 << cap_sq);
                    self.hash ^= z.pezzi[them][p][cap_sq];
                    let v = crate::evaluation::get_pst(p, cap_sq, them == 0);
                    self.pst_val -= if them == 0 { v } else { -v };
                    break;
                }
            }
        }

        let mut final_p = moved_piece;
        if flag == MoveFlag::Promotion { final_p = 4; } 
        self.pezzi[final_p] |= 1 << to;
        self.colori[us] |= 1 << to;
        let v_to = crate::evaluation::get_pst(final_p, to, us == 0);
        self.pst_val += if us == 0 { v_to } else { -v_to };

        if flag == MoveFlag::Castle {
            let (rf, rt) = match to { 6=>(7,5), 2=>(0,3), 62=>(63,61), 58=>(56,59), _=>(0,0) };
            self.pezzi[3] ^= (1 << rf) | (1 << rt);
            self.colori[us] ^= (1 << rf) | (1 << rt);
            self.hash ^= z.pezzi[us][3][rf] ^ z.pezzi[us][3][rt];
        }

        self.diritti_arrocco &= CASTLING_RIGHTS_UPDATE[from] & CASTLING_RIGHTS_UPDATE[to];
        self.ep_square = if flag == MoveFlag::DoublePawnPush { Some(if us == 0 { to - 8 } else { to + 8 }) } else { None };

        self.hash ^= z.pezzi[us][final_p][to];
        if let Some(ep) = self.ep_square { self.hash ^= z.ep_file[ep % 8]; }
        self.hash ^= z.arrocco[self.diritti_arrocco as usize] ^ z.turno;
        
        let moved_color = self.turno;
        self.turno = self.turno.opposto();

        if self.re_in_scacco(moved_color) { return false; }
        true
    }

    #[inline(always)]
    pub fn make_null_move_hash(&mut self, z: &ZobristKeys) {
        if let Some(ep) = self.ep_square { self.hash ^= z.ep_file[ep % 8]; }
        self.ep_square = None;
        self.hash ^= z.turno;
        self.turno = self.turno.opposto();
    }
}