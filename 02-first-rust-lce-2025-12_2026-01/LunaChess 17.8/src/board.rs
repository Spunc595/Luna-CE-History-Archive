use std::fmt;
use crate::zobrist::ZobristKeys;

pub type Bitboard = u64;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Colore {
    Bianco = 0,
    Nero = 1,
}

impl Colore {
    pub fn opposto(&self) -> Colore {
        match self {
            Colore::Bianco => Colore::Nero,
            Colore::Nero => Colore::Bianco,
        }
    }
    
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
    pub fn is_capture(&self) -> bool {
        match self {
            MoveFlag::Capture | MoveFlag::EnPassant => true,
            // Promotion può essere cattura o no, qui semplifichiamo. 
            // La logica di movegen setta Capture se mangia promuovendo? 
            // Solitamente si gestisce a parte, ma per MVV-LVA basta sapere se mangia.
            _ => false,
        }
    }
    
    pub fn is_promotion(&self) -> bool {
        matches!(self, MoveFlag::Promotion)
    }
}

// Struttura Mossa compatta (16 bit)
// Bit 0-5: From, Bit 6-11: To, Bit 12-15: Flag
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Mossa {
    pub data: u16,
}

impl Mossa {
    pub fn new(from: usize, to: usize, flag: MoveFlag) -> Self {
        let flag_bits = match flag {
            MoveFlag::None => 0,
            MoveFlag::EnPassant => 1,
            MoveFlag::Castle => 2,
            MoveFlag::Promotion => 3,
            MoveFlag::Capture => 4,
            MoveFlag::DoublePawnPush => 5,
        };
        Mossa {
            data: (from as u16) | ((to as u16) << 6) | ((flag_bits as u16) << 12)
        }
    }

    pub fn da(&self) -> usize { (self.data & 0x3F) as usize }
    pub fn a(&self) -> usize { ((self.data >> 6) & 0x3F) as usize }
    
    pub fn move_flag(&self) -> MoveFlag {
        match (self.data >> 12) & 0xF {
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
        if self.move_flag() == MoveFlag::Promotion {
            write!(f, "q")?; // Default a regina
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct Scacchiera {
    pub pezzi: [Bitboard; 6], // P, N, B, R, Q, K
    pub colori: [Bitboard; 2], // Bianco, Nero
    pub turno: Colore,
    pub ep_square: Option<usize>,
    pub diritti_arrocco: u8, // Bitmask: WK=1, WQ=2, BK=4, BQ=8
    pub pst_val: i32, // Valutazione incrementale
    pub hash: u64,    // Zobrist Hash corrente
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
            if c == '/' {
                rank -= 1;
                file = 0;
            } else if c.is_digit(10) {
                file += c.to_digit(10).unwrap() as usize;
            } else {
                let sq = rank * 8 + file;
                let (color, piece_idx) = match c {
                    'P' => (Colore::Bianco, 0), 'N' => (Colore::Bianco, 1),
                    'B' => (Colore::Bianco, 2), 'R' => (Colore::Bianco, 3),
                    'Q' => (Colore::Bianco, 4), 'K' => (Colore::Bianco, 5),
                    'p' => (Colore::Nero, 0),   'n' => (Colore::Nero, 1),
                    'b' => (Colore::Nero, 2),   'r' => (Colore::Nero, 3),
                    'q' => (Colore::Nero, 4),   'k' => (Colore::Nero, 5),
                    _ => panic!("FEN invalido"),
                };
                
                let mask = 1u64 << sq;
                pezzi[piece_idx] |= mask;
                colori[color.indice()] |= mask;
                
                // Aggiorna PST iniziale
                let mg_pst = crate::evaluation::get_pst(piece_idx, sq, true);
                if color == Colore::Bianco { pst_val += mg_pst; } else { pst_val -= mg_pst; }

                file += 1;
            }
        }

        let turno = if parts[1] == "w" { Colore::Bianco } else { Colore::Nero };
        
        let mut diritti_arrocco = 0;
        if parts[2].contains('K') { diritti_arrocco |= 1; }
        if parts[2].contains('Q') { diritti_arrocco |= 2; }
        if parts[2].contains('k') { diritti_arrocco |= 4; }
        if parts[2].contains('q') { diritti_arrocco |= 8; }

        let ep_square = if parts[3] == "-" {
            None
        } else {
            let cs: Vec<char> = parts[3].chars().collect();
            let f = cs[0] as usize - 'a' as usize;
            let r = cs[1].to_digit(10).unwrap() as usize - 1;
            Some(r * 8 + f)
        };

        Scacchiera {
            pezzi,
            colori,
            turno,
            ep_square,
            diritti_arrocco,
            pst_val,
            hash: 0, 
        }
    }

    // --- METODO FONDAMENTALE PER LA SEARCH ---
    pub fn make_null_move(&mut self) {
        self.turno = self.turno.opposto();
        self.ep_square = None;
        // Nella null move non aggiorniamo l'hash completo per performance, 
        // poiché viene usata solo in una copia temporanea della board che viene scartata.
        // Se volessimo essere rigorosi: self.hash ^= z.side;
    }

    pub fn occupazione(&self) -> Bitboard {
        self.colori[0] | self.colori[1]
    }

    pub fn genera_mosse(&self) -> Vec<Mossa> {
        crate::movegen::genera_mosse(self)
    }

    // Hash corretto con i nomi italiani
    pub fn get_hash(&self, z: &ZobristKeys) -> u64 {
        let mut h = 0;
        
        // Usa z.turno invece di z.side
        if self.turno == Colore::Nero { h ^= z.turno; }
        
        for p in 0..6 {
            let mut bb = self.pezzi[p];
            while bb != 0 {
                let sq = bb.trailing_zeros() as usize;
                let c = if (self.colori[0] & (1 << sq)) != 0 { 0 } else { 1 };
                // Usa z.pezzi invece di z.pieces
                h ^= z.pezzi[c][p][sq];
                bb &= bb - 1;
            }
        }
        
        // Includi arrocco ed ep se vuoi precisione massima nel TT
        h ^= z.arrocco[self.diritti_arrocco as usize];
        if let Some(ep) = self.ep_square {
             // Assumiamo che Zobrist abbia un array ep_file
             // h ^= z.ep_file[ep % 8]; 
        }
        
        h
    }

    pub fn re_in_scacco(&self, c: Colore) -> bool {
        let k_bb = self.pezzi[5] & self.colori[c.indice()];
        if k_bb == 0 { return true; } 
        let k_sq = k_bb.trailing_zeros() as usize;
        self.casella_attaccata(k_sq, c.opposto())
    }

    pub fn casella_attaccata(&self, sq: usize, by: Colore) -> bool {
        let occ = self.occupazione();
        let nemici = self.colori[by.indice()];

        // 1. Pedoni (Reverse lookup: chi mi attacca come pedone?)
        if (crate::attacks::pawn_attacks(sq, by.opposto()) & self.pezzi[0] & nemici) != 0 { return true; }

        // 2. Cavalli
        if (crate::attacks::knight_attacks(sq) & self.pezzi[1] & nemici) != 0 { return true; }
        
        // 3. Re
        if (crate::attacks::king_attacks(sq) & self.pezzi[5] & nemici) != 0 { return true; }

        // 4. Alfieri / Regine
        let bq = (self.pezzi[2] | self.pezzi[4]) & nemici;
        if bq != 0 {
             if (crate::attacks::bishop_attacks(sq, occ) & bq) != 0 { return true; }
        }

        // 5. Torri / Regine
        let rq = (self.pezzi[3] | self.pezzi[4]) & nemici;
        if rq != 0 {
             if (crate::attacks::rook_attacks(sq, occ) & rq) != 0 { return true; }
        }

        false
    }
    
    // Helper per valutazione mobilità in evaluation.rs
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

    pub fn esegui_mossa(&mut self, m: &Mossa, _z: Option<&ZobristKeys>) -> bool {
        let us = self.turno;
        let them = us.opposto();
        let from = m.da();
        let to = m.a();
        let flag = m.move_flag();
        
        let mut piece_idx = 6;
        for p in 0..6 {
            if (self.pezzi[p] & self.colori[us.indice()] & (1u64 << from)) != 0 {
                piece_idx = p;
                break;
            }
        }
        if piece_idx == 6 { return false; }

        let mask_from = 1u64 << from;
        let mask_to = 1u64 << to;
        
        self.pezzi[piece_idx] ^= mask_from;
        self.colori[us.indice()] ^= mask_from;
        
        // Aggiorna PST (Rimuovi vecchio)
        let pst_old = crate::evaluation::get_pst(piece_idx, from, true);
        if us == Colore::Bianco { self.pst_val -= pst_old; } else { self.pst_val += pst_old; }

        // Gestione Cattura
        if flag.is_capture() {
            // Se EnPassant, la cattura è su una casa diversa
            let cap_sq = if flag == MoveFlag::EnPassant {
                if us == Colore::Bianco { to - 8 } else { to + 8 }
            } else {
                to
            };
            let mask_cap = 1u64 << cap_sq;

            for p in 0..6 {
                if (self.pezzi[p] & self.colori[them.indice()] & mask_cap) != 0 {
                    self.pezzi[p] ^= mask_cap;
                    self.colori[them.indice()] ^= mask_cap;
                    
                    let pst_cap = crate::evaluation::get_pst(p, cap_sq, true);
                    if them == Colore::Bianco { self.pst_val -= pst_cap; } else { self.pst_val += pst_cap; }
                    break;
                }
            }
        }

        // Muovi pezzo in to
        self.pezzi[piece_idx] |= mask_to;
        self.colori[us.indice()] |= mask_to;
        
        // Aggiorna PST (Aggiungi nuovo)
        let pst_new = crate::evaluation::get_pst(piece_idx, to, true);
        if us == Colore::Bianco { self.pst_val += pst_new; } else { self.pst_val -= pst_new; }

        // Gestione Arrocco (Muovere la Torre)
        if flag == MoveFlag::Castle {
            let (r_from, r_to) = match to {
                6 => (7, 5), // Bianco corto
                2 => (0, 3), // Bianco lungo
                62 => (63, 61), // Nero corto
                58 => (56, 59), // Nero lungo
                _ => (0, 0),
            };
            let mask_r_from = 1u64 << r_from;
            let mask_r_to = 1u64 << r_to;
            
            self.pezzi[3] ^= mask_r_from | mask_r_to;
            self.colori[us.indice()] ^= mask_r_from | mask_r_to;
            
            let pst_r_old = crate::evaluation::get_pst(3, r_from, true);
            let pst_r_new = crate::evaluation::get_pst(3, r_to, true);
            if us == Colore::Bianco { self.pst_val += pst_r_new - pst_r_old; } 
            else { self.pst_val -= pst_r_new - pst_r_old; }
        }

        // Gestione Promozione
        if flag.is_promotion() {
            self.pezzi[0] ^= mask_to; // Togli pedone
            self.pezzi[4] |= mask_to; // Metti Regina
            
            let pst_p = crate::evaluation::get_pst(0, to, true);
            let pst_q = crate::evaluation::get_pst(4, to, true);
            // Correzione logica: Abbiamo aggiunto PST pedone in 'to' poche righe sopra.
            // Ora dobbiamo rimuoverlo e aggiungere PST regina.
            if us == Colore::Bianco { self.pst_val += pst_q - pst_p; }
            else { self.pst_val -= pst_q - pst_p; }
        }

        self.ep_square = None;
        if flag == MoveFlag::DoublePawnPush {
            self.ep_square = Some(if us == Colore::Bianco { to - 8 } else { to + 8 });
        }

        // Aggiorna diritti arrocco
        let mask_corners = 0x8100000000000081u64;
        if piece_idx == 5 { 
            if us == Colore::Bianco { self.diritti_arrocco &= !3; } else { self.diritti_arrocco &= !12; }
        }
        if (mask_from & mask_corners) != 0 || (mask_to & mask_corners) != 0 {
             if from == 0 || to == 0 { self.diritti_arrocco &= !2; } 
             if from == 7 || to == 7 { self.diritti_arrocco &= !1; } 
             if from == 56 || to == 56 { self.diritti_arrocco &= !8; } 
             if from == 63 || to == 63 { self.diritti_arrocco &= !4; } 
        }

        self.turno = them;
        
        if self.re_in_scacco(us) {
            return false;
        }

        true
    }
}