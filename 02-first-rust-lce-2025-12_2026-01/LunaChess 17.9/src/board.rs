use std::fmt;
use crate::zobrist::ZobristKeys;

pub type Bitboard = u64;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Colore {
    Bianco = 0,
    Nero = 1,
}

impl Colore {
    #[inline(always)]
    pub fn opposto(&self) -> Colore {
        match self {
            Colore::Bianco => Colore::Nero,
            Colore::Nero => Colore::Bianco,
        }
    }
    
    #[inline(always)]
    pub fn indice(&self) -> usize {
        *self as usize
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

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum MoveFlag {
    None = 0,
    EnPassant = 1,
    Castle = 2,
    Promotion = 3,
    Capture = 4,
    DoublePawnPush = 5,
}

impl MoveFlag {
    #[inline(always)]
    pub fn is_capture(&self) -> bool {
        matches!(self, MoveFlag::Capture | MoveFlag::EnPassant)
    }
    
    #[inline(always)]
    pub fn is_promotion(&self) -> bool {
        matches!(self, MoveFlag::Promotion)
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Mossa {
    pub data: u16,
}

impl Mossa {
    pub fn new(from: usize, to: usize, flag: MoveFlag) -> Self {
        Mossa {
            data: (from as u16) | ((to as u16) << 6) | ((flag as u16) << 12)
        }
    }

    #[inline(always)]
    pub fn da(&self) -> usize { (self.data & 0x3F) as usize }
    #[inline(always)]
    pub fn a(&self) -> usize { ((self.data >> 6) & 0x3F) as usize }
    
    #[inline(always)]
    pub fn move_flag(&self) -> MoveFlag {
        match (self.data >> 12) & 0x7 {
            1 => MoveFlag::EnPassant,
            2 => MoveFlag::Castle,
            3 => MoveFlag::Promotion,
            4 => MoveFlag::Capture,
            5 => MoveFlag::DoublePawnPush,
            _ => MoveFlag::None,
        }
    }
}

impl fmt::Display for Mossa {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let files = ['a','b','c','d','e','f','g','h'];
        let ranks = ['1','2','3','4','5','6','7','8'];
        let from = self.da();
        let to = self.a();
        write!(f, "{}{}{}{}", files[from%8], ranks[from/8], files[to%8], ranks[to/8])?;
        if self.move_flag() == MoveFlag::Promotion { write!(f, "q")?; }
        Ok(())
    }
}

const CASTLING_RIGHTS_UPDATE: [u8; 64] = {
    let mut r = [15u8; 64];
    r[0] = 13; r[7] = 14; r[56] = 7; r[63] = 11;
    r[4] = 12; r[60] = 3;
    r
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
        Self::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
    }

    pub fn from_fen(fen: &str) -> Self {
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
                let sq = rank * 8 + file;
                let (color, p_idx) = match c {
                    'P' => (0, 0), 'N' => (0, 1), 'B' => (0, 2), 'R' => (0, 3), 'Q' => (0, 4), 'K' => (0, 5),
                    'p' => (1, 0), 'n' => (1, 1), 'b' => (1, 2), 'r' => (1, 3), 'q' => (1, 4), 'k' => (1, 5),
                    _ => panic!("FEN invalido"),
                };
                let mask = 1u64 << sq;
                pezzi[p_idx] |= mask;
                colori[color] |= mask;
                let val = crate::evaluation::get_pst(p_idx, sq, true);
                pst_val += if color == 0 { val } else { -val };
                file += 1;
            }
        }

        let turno = if parts[1] == "w" { Colore::Bianco } else { Colore::Nero };
        let mut diritti = 0;
        if parts[2].contains('K') { diritti |= 1; }
        if parts[2].contains('Q') { diritti |= 2; }
        if parts[2].contains('k') { diritti |= 4; }
        if parts[2].contains('q') { diritti |= 8; }

        let ep_square = if parts[3] == "-" { None } else {
            let f = parts[3].as_bytes()[0] as usize - b'a' as usize;
            let r = parts[3].as_bytes()[1] as usize - b'1' as usize;
            Some(r * 8 + f)
        };

        let mut s = Scacchiera { pezzi, colori, turno, ep_square, diritti_arrocco: diritti, pst_val, hash: 0 };
        // Calcoliamo l'hash iniziale usando una chiave temporanea se necessario, 
        // o lo faremo gestire all'UCI quando carica la posizione.
        s
    }

    #[inline(always)]
    pub fn make_null_move(&mut self) {
        self.turno = self.turno.opposto();
        self.ep_square = None;
    }

    #[inline(always)]
    pub fn occupazione(&self) -> Bitboard {
        self.colori[0] | self.colori[1]
    }

    // PUBBLICO per search.rs e uci.rs
    pub fn genera_mosse(&self) -> Vec<Mossa> {
        crate::movegen::genera_mosse(self)
    }

    // PUBBLICO per search.rs e uci.rs
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
        if let Some(ep) = self.ep_square { h ^= z.ep_file[ep % 8]; }
        h
    }

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

    #[inline(always)]
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

    // Cambiato Option<&ZobristKeys> in &ZobristKeys per coerenza con la search
    pub fn esegui_mossa(&mut self, m: &Mossa, z: &ZobristKeys) -> bool {
        let us = self.turno;
        let them = us.opposto();
        let (from, to) = (m.da(), m.a());
        let flag = m.move_flag();
        
        let mut piece_idx = 6;
        for p in 0..6 {
            if (self.pezzi[p] & self.colori[us.indice()] & (1 << from)) != 0 {
                piece_idx = p; break;
            }
        }
        if piece_idx == 6 { return false; }

        // --- Aggiornamento Incrementale Hash (Parte 1) ---
        self.hash ^= z.pezzi[us.indice()][piece_idx][from];
        if let Some(ep) = self.ep_square { self.hash ^= z.ep_file[ep % 8]; }
        self.hash ^= z.arrocco[self.diritti_arrocco as usize];

        // Rimuovi pezzo da 'from'
        self.pezzi[piece_idx] &= !(1 << from);
        self.colori[us.indice()] &= !(1 << from);
        self.pst_val -= if us == Colore::Bianco { crate::evaluation::get_pst(piece_idx, from, true) } else { -crate::evaluation::get_pst(piece_idx, from, true) };

        // Cattura
        if flag.is_capture() {
            let cap_sq = if flag == MoveFlag::EnPassant { if us == Colore::Bianco { to - 8 } else { to + 8 } } else { to };
            for p in 0..6 {
                if (self.pezzi[p] & self.colori[them.indice()] & (1 << cap_sq)) != 0 {
                    self.pezzi[p] &= !(1 << cap_sq);
                    self.colori[them.indice()] &= !(1 << cap_sq);
                    self.hash ^= z.pezzi[them.indice()][p][cap_sq];
                    let v = crate::evaluation::get_pst(p, cap_sq, true);
                    self.pst_val -= if them == Colore::Bianco { v } else { -v };
                    break;
                }
            }
        }

        // Destinazione e Promozione
        let mut final_piece = piece_idx;
        if flag == MoveFlag::Promotion { final_piece = 4; }
        self.pezzi[final_piece] |= 1 << to;
        self.colori[us.indice()] |= 1 << to;
        let v_to = crate::evaluation::get_pst(final_piece, to, true);
        self.pst_val += if us == Colore::Bianco { v_to } else { -v_to };

        // Arrocco
        if flag == MoveFlag::Castle {
            let (rf, rt) = match to { 6 => (7, 5), 2 => (0, 3), 62 => (63, 61), 58 => (56, 59), _ => (0, 0) };
            self.pezzi[3] ^= (1 << rf) | (1 << rt);
            self.colori[us.indice()] ^= (1 << rf) | (1 << rt);
            self.hash ^= z.pezzi[us.indice()][3][rf] ^ z.pezzi[us.indice()][3][rt];
        }

        // Aggiorna diritti arrocco ed ep_square
        self.diritti_arrocco &= CASTLING_RIGHTS_UPDATE[from] & CASTLING_RIGHTS_UPDATE[to];
        self.ep_square = if flag == MoveFlag::DoublePawnPush { Some(if us == Colore::Bianco { to - 8 } else { to + 8 }) } else { None };
        
        // --- Aggiornamento Incrementale Hash (Parte 2) ---
        self.hash ^= z.pezzi[us.indice()][final_piece][to];
        if let Some(ep) = self.ep_square { self.hash ^= z.ep_file[ep % 8]; }
        self.hash ^= z.arrocco[self.diritti_arrocco as usize] ^ z.turno;

        self.turno = them;
        if self.re_in_scacco(us) { return false; }
        true
    }
}