use std::fmt;

pub type Bitboard = u64;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Colore { Bianco, Nero }
impl Colore {
    pub fn opposto(&self) -> Colore {
        match self { Colore::Bianco => Colore::Nero, Colore::Nero => Colore::Bianco }
    }
    pub fn indice(&self) -> usize {
        match self { Colore::Bianco => 0, Colore::Nero => 1 }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Pezzo { Pedone, Cavallo, Alfiere, Torre, Regina, Re }
impl Pezzo {
    pub fn indice(&self) -> usize {
        match self {
            Pezzo::Pedone => 0, Pezzo::Cavallo => 1, Pezzo::Alfiere => 2,
            Pezzo::Torre => 3, Pezzo::Regina => 4, Pezzo::Re => 5,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum MoveFlag { None, Capture, Castle, EnPassant, Promotion, DoublePawnPush }
impl MoveFlag {
    pub fn is_capture(&self) -> bool {
        matches!(self, MoveFlag::Capture | MoveFlag::EnPassant)
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Mossa { pub data: u16 }
impl Mossa {
    pub fn new(da: usize, a: usize, flag: MoveFlag) -> Self {
        let f = match flag {
            MoveFlag::None => 0, MoveFlag::Capture => 1, MoveFlag::Castle => 2,
            MoveFlag::EnPassant => 3, MoveFlag::Promotion => 4, MoveFlag::DoublePawnPush => 5,
        };
        Self { data: (da as u16) | ((a as u16) << 6) | (f << 12) }
    }
    pub fn da(&self) -> usize { (self.data & 0x3F) as usize }
    pub fn a(&self) -> usize { ((self.data >> 6) & 0x3F) as usize }
    pub fn move_flag(&self) -> MoveFlag {
        match (self.data >> 12) & 0x7 {
            1 => MoveFlag::Capture, 2 => MoveFlag::Castle,
            3 => MoveFlag::EnPassant, 4 => MoveFlag::Promotion,
            5 => MoveFlag::DoublePawnPush, _ => MoveFlag::None,
        }
    }
    pub fn is_promotion(&self) -> bool { self.move_flag() == MoveFlag::Promotion }
}

impl fmt::Display for Mossa {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sq = ["a1","b1","c1","d1","e1","f1","g1","h1","a2","b2","c2","d2","e2","f2","g2","h2","a3","b3","c3","d3","e3","f3","g3","h3","a4","b4","c4","d4","e4","f4","g4","h4","a5","b5","c5","d5","e5","f5","g5","h5","a6","b6","c6","d6","e6","f6","g6","h6","a7","b7","c7","d7","e7","f7","g7","h7","a8","b8","c8","d8","e8","f8","g8","h8"];
        let mut s = format!("{}{}", sq[self.da()], sq[self.a()]);
        if self.is_promotion() { s.push('q'); }
        write!(f, "{}", s)
    }
}

#[derive(Clone)]
pub struct Scacchiera {
    pub pezzi: [Bitboard; 6],
    pub colori: [Bitboard; 2],
    pub turno: Colore,
    pub ep_square: Option<usize>,
    pub diritti_arrocco: u8,
    pub pst_val: i32,
}

impl Scacchiera {
    pub fn nuova() -> Self {
        Self::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
    }

    pub fn from_fen(fen: &str) -> Self {
        let mut s = Self { pezzi: [0; 6], colori: [0; 2], turno: Colore::Bianco, ep_square: None, diritti_arrocco: 0, pst_val: 0 };
        let parts: Vec<&str> = fen.split_whitespace().collect();
        let mut row = 7; let mut col = 0;
        for c in parts[0].chars() {
            if c == '/' { row -= 1; col = 0; }
            else if c.is_digit(10) { col += c.to_digit(10).unwrap() as usize; }
            else {
                let sq = row * 8 + col;
                let (p, color) = match c {
                    'P' => (Pezzo::Pedone, Colore::Bianco), 'N' => (Pezzo::Cavallo, Colore::Bianco),
                    'B' => (Pezzo::Alfiere, Colore::Bianco), 'R' => (Pezzo::Torre, Colore::Bianco),
                    'Q' => (Pezzo::Regina, Colore::Bianco), 'K' => (Pezzo::Re, Colore::Bianco),
                    'p' => (Pezzo::Pedone, Colore::Nero), 'n' => (Pezzo::Cavallo, Colore::Nero),
                    'b' => (Pezzo::Alfiere, Colore::Nero), 'r' => (Pezzo::Torre, Colore::Nero),
                    'q' => (Pezzo::Regina, Colore::Nero), 'k' => (Pezzo::Re, Colore::Nero),
                    _ => continue,
                };
                s.add_piece(p, color, sq); col += 1;
            }
        }
        if parts.len() > 1 { s.turno = if parts[1] == "w" { Colore::Bianco } else { Colore::Nero }; }
        if parts.len() > 2 {
            for c in parts[2].chars() {
                match c { 'K'=>s.diritti_arrocco|=1, 'Q'=>s.diritti_arrocco|=2, 'k'=>s.diritti_arrocco|=4, 'q'=>s.diritti_arrocco|=8, _=>() }
            }
        }
        s
    }

    pub fn get_attacks_for_piece(&self, p: Pezzo, sq: usize, occ: Bitboard) -> Bitboard {
        match p {
            Pezzo::Torre => crate::attacks::rook_attacks(sq, occ),
            Pezzo::Alfiere => crate::attacks::bishop_attacks(sq, occ),
            Pezzo::Regina => crate::attacks::rook_attacks(sq, occ) | crate::attacks::bishop_attacks(sq, occ),
            Pezzo::Cavallo => crate::attacks::knight_attacks(sq),
            Pezzo::Re => crate::attacks::king_attacks(sq),
            _ => 0,
        }
    }

    pub fn add_piece(&mut self, p: Pezzo, c: Colore, sq: usize) {
        let bit = 1u64 << sq;
        self.pezzi[p.indice()] |= bit;
        self.colori[c.indice()] |= bit;
        let rel_sq = if c == Colore::Bianco { sq ^ 56 } else { sq };
        let val = crate::evaluation::get_pst(p.indice(), rel_sq, true);
        if c == Colore::Bianco { self.pst_val += val; } else { self.pst_val -= val; }
    }

    pub fn remove_piece(&mut self, p: Pezzo, c: Colore, sq: usize) {
        let bit = 1u64 << sq;
        self.pezzi[p.indice()] &= !bit;
        self.colori[c.indice()] &= !bit;
        let rel_sq = if c == Colore::Bianco { sq ^ 56 } else { sq };
        let val = crate::evaluation::get_pst(p.indice(), rel_sq, true);
        if c == Colore::Bianco { self.pst_val -= val; } else { self.pst_val += val; }
    }

    pub fn esegui_mossa(&mut self, m: &Mossa, _state: Option<()>) -> bool {
        let original = self.clone();
        let us = self.turno;
        let (from, to) = (m.da(), m.a());
        let piece = match self.get_piece_at(from) { Some(p) => p, None => return false };

        if piece == Pezzo::Re {
            if us == Colore::Bianco { self.diritti_arrocco &= !3; } else { self.diritti_arrocco &= !12; }
        }
        self.aggiorna_diritti_torre(from);
        self.aggiorna_diritti_torre(to);

        self.remove_piece(piece, us, from);
        if let Some(cap) = self.get_piece_at(to) { self.remove_piece(cap, us.opposto(), to); }
        
        if m.move_flag() == MoveFlag::EnPassant {
            let cap_sq = if us == Colore::Bianco { to - 8 } else { to + 8 };
            self.remove_piece(Pezzo::Pedone, us.opposto(), cap_sq);
        } else if m.move_flag() == MoveFlag::Castle {
            let (r_f, r_t) = match to { 6=>(7,5), 2=>(0,3), 62=>(63,61), 58=>(56,59), _=>(0,0) };
            self.remove_piece(Pezzo::Torre, us, r_f);
            self.add_piece(Pezzo::Torre, us, r_t);
        }
        
        self.add_piece(if m.is_promotion() { Pezzo::Regina } else { piece }, us, to);
        if self.re_in_scacco(us) { *self = original; return false; }
        self.turno = us.opposto();
        true
    }

    pub fn make_null_move(&mut self) {
        self.turno = self.turno.opposto();
        self.ep_square = None;
    }

    fn aggiorna_diritti_torre(&mut self, sq: usize) {
        match sq { 7=>self.diritti_arrocco&=!1, 0=>self.diritti_arrocco&=!2, 63=>self.diritti_arrocco&=!4, 56=>self.diritti_arrocco&=!8, _=>() }
    }

    pub fn get_piece_at(&self, sq: usize) -> Option<Pezzo> {
        let bit = 1u64 << sq;
        for p in 0..6 { if (self.pezzi[p] & bit) != 0 { return match p { 0=>Some(Pezzo::Pedone), 1=>Some(Pezzo::Cavallo), 2=>Some(Pezzo::Alfiere), 3=>Some(Pezzo::Torre), 4=>Some(Pezzo::Regina), _=>Some(Pezzo::Re) }; } }
        None
    }

    pub fn re_in_scacco(&self, c: Colore) -> bool {
        let k_bb = self.pezzi[5] & self.colori[c.indice()];
        if k_bb == 0 { return false; }
        self.casella_attaccata(k_bb.trailing_zeros() as usize, c.opposto())
    }

    pub fn casella_attaccata(&self, sq: usize, by_color: Colore) -> bool {
        let occ = self.occupazione();
        let attackers = self.colori[by_color.indice()];
        if (crate::attacks::rook_attacks(sq, occ) & (self.pezzi[3] | self.pezzi[4]) & attackers) != 0 { return true; }
        if (crate::attacks::bishop_attacks(sq, occ) & (self.pezzi[2] | self.pezzi[4]) & attackers) != 0 { return true; }
        if (crate::attacks::knight_attacks(sq) & self.pezzi[1] & attackers) != 0 { return true; }
        if (crate::attacks::king_attacks(sq) & self.pezzi[5] & attackers) != 0 { return true; }
        if (crate::attacks::pawn_attacks(sq, by_color.opposto()) & self.pezzi[0] & attackers) != 0 { return true; }
        false
    }

    pub fn occupazione(&self) -> Bitboard { self.colori[0] | self.colori[1] }
    pub fn genera_mosse(&self) -> Vec<Mossa> { crate::movegen::genera_mosse(self) }
    pub fn get_hash(&self, z: &crate::zobrist::ZobristKeys) -> u64 { z.hash(self) }
}