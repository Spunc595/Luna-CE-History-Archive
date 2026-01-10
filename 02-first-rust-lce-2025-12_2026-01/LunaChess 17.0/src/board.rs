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
}

impl Scacchiera {
    pub fn nuova() -> Self {
        let mut s = Self { pezzi: [0; 6], colori: [0; 2], turno: Colore::Bianco, ep_square: None, diritti_arrocco: 15 };
        s.pezzi[0] = 0x00FF00000000FF00; s.pezzi[1] = 0x4200000000000042; s.pezzi[2] = 0x2400000000000024;
        s.pezzi[3] = 0x8100000000000081; s.pezzi[4] = 0x0800000000000008; s.pezzi[5] = 0x1000000000000010;
        s.colori[0] = 0x000000000000FFFF; s.colori[1] = 0xFFFF000000000000;
        s
    }

    pub fn occupazione(&self) -> Bitboard { self.colori[0] | self.colori[1] }

    pub fn genera_mosse(&self) -> Vec<Mossa> { crate::movegen::genera_mosse(self) }

    pub fn get_piece_at(&self, sq: usize) -> Option<Pezzo> {
        let bit = 1u64 << sq;
        for p in 0..6 { if (self.pezzi[p] & bit) != 0 { return match p { 0=>Some(Pezzo::Pedone), 1=>Some(Pezzo::Cavallo), 2=>Some(Pezzo::Alfiere), 3=>Some(Pezzo::Torre), 4=>Some(Pezzo::Regina), _=>Some(Pezzo::Re) }; } }
        None
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

    pub fn esegui_mossa(&mut self, m: &Mossa, _state: Option<()>) -> bool {
        let original_state = self.clone(); // Per rollback se mossa illegale
        let us = self.turno;
        let (from, to) = (m.da(), m.a());
        let piece = match self.get_piece_at(from) { Some(p) => p, None => return false };
        
        self.remove_piece(piece, us, from);
        if let Some(cap) = self.get_piece_at(to) { self.remove_piece(cap, us.opposto(), to); }
        
        if m.move_flag() == MoveFlag::EnPassant {
            let cap_sq = if us == Colore::Bianco { to - 8 } else { to + 8 };
            self.remove_piece(Pezzo::Pedone, us.opposto(), cap_sq);
        } else if m.move_flag() == MoveFlag::Castle {
            let (r_f, r_t) = match to { 6=>(7,5), 2=>(0,3), 62=>(63,61), 58=>(56,59), _=>(0,0) };
            self.remove_piece(Pezzo::Torre, us, r_f); self.add_piece(Pezzo::Torre, us, r_t);
        }

        self.add_piece(if m.is_promotion() { Pezzo::Regina } else { piece }, us, to);
        
        if self.re_in_scacco(us) { 
            *self = original_state; // Rollback
            return false; 
        }
        
        self.aggiorna_diritti_arrocco(from, to);
        self.ep_square = if m.move_flag() == MoveFlag::DoublePawnPush { Some(if us == Colore::Bianco {from+8} else {from-8}) } else { None };
        self.turno = us.opposto();
        true
    }

    /// Fondamentale per il Null Move Pruning
    pub fn make_null_move(&mut self) {
        self.ep_square = None;
        self.turno = self.turno.opposto();
    }

    fn remove_piece(&mut self, p: Pezzo, c: Colore, sq: usize) {
        let b = !(1u64 << sq); 
        self.pezzi[p.indice()] &= b; 
        self.colori[c.indice()] &= b;
    }
    
    fn add_piece(&mut self, p: Pezzo, c: Colore, sq: usize) {
        let b = 1u64 << sq; 
        self.pezzi[p.indice()] |= b; 
        self.colori[c.indice()] |= b;
    }

    fn aggiorna_diritti_arrocco(&mut self, f: usize, t: usize) {
        if f == 4 || t == 4 { self.diritti_arrocco &= !3; }
        if f == 60 || t == 60 { self.diritti_arrocco &= !12; }
        if f == 0 || t == 0 { self.diritti_arrocco &= !2; }
        if f == 7 || t == 7 { self.diritti_arrocco &= !1; }
        if f == 56 || t == 56 { self.diritti_arrocco &= !8; }
        if f == 63 || t == 63 { self.diritti_arrocco &= !4; }
    }
}