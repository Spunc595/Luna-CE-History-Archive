use std::fmt;
use crate::zobrist::ZobristKeys;

// Tipi base
pub type Bitboard = u64;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Colore { Bianco = 0, Nero = 1 }

impl Colore {
    #[inline(always)] pub fn opposto(&self) -> Colore { match self { Colore::Bianco => Colore::Nero, Colore::Nero => Colore::Bianco } }
    #[inline(always)] pub fn indice(&self) -> usize { *self as usize }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Pezzo { 
    Pedone = 0, 
    Cavallo = 1, 
    Alfiere = 2, 
    Torre = 3, 
    Regina = 4, 
    Re = 5 
}

impl Pezzo {
    #[inline(always)] pub fn indice(&self) -> usize { *self as usize }
    #[inline(always)] pub fn valore(&self) -> i32 {
        match self { 
            Pezzo::Pedone => 100, 
            Pezzo::Cavallo => 320, 
            Pezzo::Alfiere => 330, 
            Pezzo::Torre => 500, 
            Pezzo::Regina => 900, 
            Pezzo::Re => 20000 
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(u16)]
pub enum MoveFlag { None = 0, EnPassant = 1, Castle = 2, Promotion = 3, Capture = 4, DoublePawnPush = 5 }

impl MoveFlag { 
    #[inline(always)] pub fn is_capture(&self) -> bool { matches!(self, MoveFlag::Capture | MoveFlag::EnPassant) } 
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
            1 => MoveFlag::EnPassant, 
            2 => MoveFlag::Castle, 
            3 => MoveFlag::Promotion, 
            4 => MoveFlag::Capture, 
            5 => MoveFlag::DoublePawnPush, 
            _ => MoveFlag::None 
        }
    }

    pub fn is_cattura(&self) -> bool { self.move_flag().is_capture() }
    pub fn is_promozione(&self) -> bool { self.move_flag() == MoveFlag::Promotion }

    pub fn to_uci(&self) -> String {
        let files = ['a','b','c','d','e','f','g','h'];
        let ranks = ['1','2','3','4','5','6','7','8'];
        let mut s = format!("{}{}{}{}", files[self.da() % 8], ranks[self.da() / 8], files[self.a() % 8], ranks[self.a() / 8]);
        if self.move_flag() == MoveFlag::Promotion { s.push('q'); }
        s
    }

    pub fn priority(&self, board: &Scacchiera) -> i32 {
        let mut score = 0;
        if self.is_cattura() {
            let victim_val = board.pezzo_in(self.a()).map(|p| Pezzo::from_index(p).valore()).unwrap_or(100);
            let attacker_val = board.get_piece_type(self.da()).map(|p| p.valore()).unwrap_or(0);
            score = 10000 + victim_val * 10 - attacker_val;
        } 
        if self.move_flag() == MoveFlag::Promotion { score += 9000; }
        score
    }
}

impl Pezzo {
    pub fn from_index(i: usize) -> Pezzo {
        match i {
            0 => Pezzo::Pedone, 1 => Pezzo::Cavallo, 2 => Pezzo::Alfiere,
            3 => Pezzo::Torre, 4 => Pezzo::Regina, _ => Pezzo::Re,
        }
    }
}

impl fmt::Display for Mossa {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}", self.to_uci()) }
}

#[derive(Clone, Copy)]
pub struct UndoData {
    pub hash: u64,
    pub ep_square: Option<usize>,
    pub diritti_arrocco: u8,
    pub mezze_mosse: u32,
    pub pst_val: i32,
}

#[derive(Clone)]
pub struct Scacchiera {
    pub pezzi: [Bitboard; 6],    // P, N, B, R, Q, K (Indici 0-5 come in train.py)
    pub colori: [Bitboard; 2],   // Bianco, Nero
    pub turno: Colore,
    pub ep_square: Option<usize>,
    pub diritti_arrocco: u8,     
    pub pst_val: i32,            
    pub hash: u64,               
    pub mezze_mosse: u32,        
    pub history: Vec<UndoData>,
}

const CASTLING_RIGHTS_UPDATE: [u8; 64] = {
    let mut r = [15u8; 64];
    r[0] = 13; r[7] = 14; r[56] = 7; r[63] = 11;
    r[4] = 12; r[60] = 3;
    r
};

impl Scacchiera {
    pub fn from_fen(fen: &str, z: &ZobristKeys) -> Self {
        let mut pezzi = [0; 6];
        let mut colori = [0; 2];
        let mut pst_val = 0;
        let parts: Vec<&str> = fen.split_whitespace().collect();
        let mut rank = 7; let mut file = 0;

        for c in parts[0].chars() {
            if c == '/' { rank -= 1; file = 0; }
            else if let Some(d) = c.to_digit(10) { file += d as usize; }
            else {
                let sq = (rank * 8 + file) as usize;
                let (c_idx, p_idx) = match c { 
                    'P'=>(0,0),'N'=>(0,1),'B'=>(0,2),'R'=>(0,3),'Q'=>(0,4),'K'=>(0,5),
                    'p'=>(1,0),'n'=>(1,1),'b'=>(1,2),'r'=>(1,3),'q'=>(1,4),'k'=>(1,5), _=>continue 
                };
                pezzi[p_idx] |= 1 << sq; 
                colori[c_idx] |= 1 << sq;
                let val = crate::evaluation::get_pst(p_idx, sq, true);
                pst_val += if c_idx == 0 { val as i32 } else { -(val as i32) };
                file += 1;
            }
        }
        
        let turno = if parts.len() > 1 && parts[1] == "b" { Colore::Nero } else { Colore::Bianco };
        let mut diritti = 0;
        if parts.len() > 2 {
            if parts[2].contains('K') { diritti |= 1; } if parts[2].contains('Q') { diritti |= 2; }
            if parts[2].contains('k') { diritti |= 4; } if parts[2].contains('q') { diritti |= 8; }
        }
        
        let ep_sq = if parts.len() > 3 && parts[3] != "-" {
            let f = (parts[3].as_bytes()[0] - b'a') as usize; 
            let r = (parts[3].as_bytes()[1] - b'1') as usize; 
            Some(r * 8 + f)
        } else { None };

        let mut s = Scacchiera { pezzi, colori, turno, ep_square: ep_sq, diritti_arrocco: diritti, pst_val, hash: 0, mezze_mosse: 0, history: Vec::with_capacity(128) };
        s.hash = s.get_hash(z);
        s
    }

    #[inline(always)] pub fn occupazione(&self) -> Bitboard { self.colori[0] | self.colori[1] }
    
    pub fn get_hash(&self, z: &ZobristKeys) -> u64 {
        let mut h = if self.turno == Colore::Nero { z.turno } else { 0 };
        for p in 0..6 {
            let mut bb = self.pezzi[p];
            while bb != 0 {
                let sq = bb.trailing_zeros() as usize;
                let c = if (self.colori[0] & (1 << sq)) != 0 { 0 } else { 1 };
                h ^= z.pezzi[c][p][sq];
                bb &= bb - 1;
            }
        }
        h ^= z.arrocco[self.diritti_arrocco as usize];
        if let Some(sq) = self.ep_square { h ^= z.ep_file[sq % 8]; }
        h
    }

    pub fn pezzo_in(&self, sq: usize) -> Option<usize> {
        let mask = 1 << sq;
        for p in 0..6 { if (self.pezzi[p] & mask) != 0 { return Some(p); } }
        None
    }

    pub fn get_piece_type(&self, sq: usize) -> Option<Pezzo> {
        self.pezzo_in(sq).map(Pezzo::from_index)
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

    pub fn re_in_scacco(&self, c: Colore) -> bool {
        let k_bb = self.pezzi[5] & self.colori[c.indice()];
        if k_bb == 0 { return false; }
        self.casella_attaccata(k_bb.trailing_zeros() as usize, c.opposto())
    }

    pub fn esegui_mossa(&mut self, m: &Mossa, z: &ZobristKeys) -> bool {
        let undo = UndoData { hash: self.hash, ep_square: self.ep_square, diritti_arrocco: self.diritti_arrocco, mezze_mosse: self.mezze_mosse, pst_val: self.pst_val };
        let us = self.turno.indice(); 
        let them = 1 - us;
        let from = m.da(); 
        let to = m.a(); 
        let flag = m.move_flag();
        
        let moved_p = self.pezzo_in(from).unwrap_or(5);
        
        self.hash ^= z.pezzi[us][moved_p][from] ^ z.arrocco[self.diritti_arrocco as usize];
        if let Some(sq) = self.ep_square { self.hash ^= z.ep_file[sq % 8]; }

        let pst_remove = crate::evaluation::get_pst(moved_p, from, true) as i32;
        self.pst_val -= if us == 0 { pst_remove } else { -pst_remove };

        self.pezzi[moved_p] &= !(1 << from); 
        self.colori[us] &= !(1 << from);

        if flag.is_capture() {
            let cap_sq = if flag == MoveFlag::EnPassant { if us == 0 { to - 8 } else { to + 8 } } else { to };
            if let Some(p) = self.pezzo_in(cap_sq) {
                self.hash ^= z.pezzi[them][p][cap_sq];
                self.pezzi[p] &= !(1 << cap_sq); 
                self.colori[them] &= !(1 << cap_sq);
                let pst_cap = crate::evaluation::get_pst(p, cap_sq, true) as i32;
                self.pst_val -= if them == 0 { pst_cap } else { -pst_cap };
            }
        }
        
        let final_p = if flag == MoveFlag::Promotion { 4 } else { moved_p };

        self.pezzi[final_p] |= 1 << to; 
        self.colori[us] |= 1 << to;
        self.hash ^= z.pezzi[us][final_p][to];

        let pst_add = crate::evaluation::get_pst(final_p, to, true) as i32;
        self.pst_val += if us == 0 { pst_add } else { -pst_add };

        if flag == MoveFlag::Castle {
            let (rf, rt) = match to { 6=>(7,5), 2=>(0,3), 62=>(63,61), 58=>(56,59), _=>(0,0) };
            self.pezzi[3] ^= (1 << rf) | (1 << rt); 
            self.colori[us] ^= (1 << rf) | (1 << rt);
            self.hash ^= z.pezzi[us][3][rf] ^ z.pezzi[us][3][rt];
            let delta = -(crate::evaluation::get_pst(3, rf, true) as i32) + (crate::evaluation::get_pst(3, rt, true) as i32);
            self.pst_val += if us == 0 { delta } else { -delta };
        }

        self.diritti_arrocco &= CASTLING_RIGHTS_UPDATE[from] & CASTLING_RIGHTS_UPDATE[to];
        self.ep_square = if flag == MoveFlag::DoublePawnPush { Some(if us == 0 { to - 8 } else { to + 8 }) } else { None };
        if let Some(sq) = self.ep_square { self.hash ^= z.ep_file[sq % 8]; }
        
        self.hash ^= z.arrocco[self.diritti_arrocco as usize] ^ z.turno;
        self.turno = self.turno.opposto();

        if self.re_in_scacco(self.turno.opposto()) { 
            self.colori[us] |= 1 << from;
            self.pezzi[moved_p] |= 1 << from;
            self.annulla_mossa_veloce(undo); 
            return false; 
        }
        
        self.history.push(undo);
        true
    }

    fn annulla_mossa_veloce(&mut self, undo: UndoData) {
        self.turno = self.turno.opposto(); 
        self.hash = undo.hash; 
        self.ep_square = undo.ep_square;
        self.diritti_arrocco = undo.diritti_arrocco; 
        self.mezze_mosse = undo.mezze_mosse; 
        self.pst_val = undo.pst_val;
    }

    pub fn genera_mosse(&self) -> Vec<Mossa> { crate::movegen::genera_mosse(self) }
}