use std::fmt;
use crate::attacks::{pawn_attacks, knight_attacks, king_attacks, bishop_attacks, rook_attacks, queen_attacks};

pub type Bitboard = u64;
pub const NNUE_LAYER1_SIZE: usize = 1536;

#[derive(Clone, Copy)]
pub struct NNUEAccumulator { pub v: [i16; NNUE_LAYER1_SIZE] }

lazy_static::lazy_static! {
    static ref ZOBRIST_TABLE: ZobristKeys = ZobristKeys::new();
}

struct ZobristKeys {
    pezzi: [[[u64; 64]; 6]; 2],
    turno: u64,
    arrocco: [u64; 16],
    ep_file: [u64; 8],
}

impl ZobristKeys {
    fn new() -> Self {
        use rand::{Rng, SeedableRng};
        let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(42);
        let mut pezzi = [[[0u64; 64]; 6]; 2];
        for c in 0..2 { for p in 0..6 { for s in 0..64 { pezzi[c][p][s] = rng.gen(); } } }
        let mut arrocco = [0u64; 16];
        for i in 0..16 { arrocco[i] = rng.gen(); }
        let mut ep_file = [0u64; 8];
        for i in 0..8 { ep_file[i] = rng.gen(); }
        
        Self { pezzi, turno: rng.gen(), arrocco, ep_file }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Colore { Bianco = 0, Nero = 1 }
impl Colore {
    pub fn indice(&self) -> usize { *self as usize }
    pub fn opposto(&self) -> Self { match self { Self::Bianco => Self::Nero, Self::Nero => Self::Bianco } }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pezzo { Pedone=0, Cavallo=1, Alfiere=2, Torre=3, Regina=4, Re=5 }
impl Pezzo { pub fn indice(&self) -> usize { *self as usize } }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveFlag { None, DoublePawnPush, Capture, EnPassant, Promotion, Castle }

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Mossa { pub data: u16 }

impl Mossa {
    pub fn new(da: usize, a: usize, flag: MoveFlag) -> Self {
        let f = match flag {
            MoveFlag::None => 0, MoveFlag::DoublePawnPush => 1,
            MoveFlag::Capture => 4, MoveFlag::EnPassant => 5, MoveFlag::Promotion => 6, MoveFlag::Castle => 7,
        };
        Mossa { data: (da as u16) | ((a as u16) << 6) | (f << 12) }
    }
    pub fn da(&self) -> usize { (self.data & 0x3F) as usize }
    pub fn a(&self) -> usize { ((self.data >> 6) & 0x3F) as usize }
    pub fn flag(&self) -> u16 { self.data >> 12 }
    
    pub fn move_flag(&self) -> MoveFlag {
        match self.flag() {
            1 => MoveFlag::DoublePawnPush,
            4 => MoveFlag::Capture,
            5 => MoveFlag::EnPassant,
            6 => MoveFlag::Promotion,
            7 => MoveFlag::Castle,
            _ => MoveFlag::None
        }
    }
}

impl fmt::Display for Mossa {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let da = self.da(); let a = self.a();
        let mut s = format!("{}{}{}{}", 
            (b'a' + (da % 8) as u8) as char, (b'1' + (da / 8) as u8) as char,
            (b'a' + (a % 8) as u8) as char, (b'1' + (a / 8) as u8) as char);
        if self.flag() == 6 { s.push('q'); }
        write!(f, "{}", s)
    }
}

#[derive(Clone)]
pub struct Scacchiera {
    pub pezzi: [Bitboard; 6],
    pub colori: [Bitboard; 2],
    pub turno: Colore,
    pub hash: u64,
    pub diritti_arrocco: u8,
    pub ep_square: Option<u8>,
    pub history: Vec<(u8, Option<Pezzo>, u64, Option<u8>)>,
    pub current_acc: [NNUEAccumulator; 2],
}

impl Scacchiera {
    pub fn nuova() -> Self {
        Self::da_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap()
    }

    pub fn da_fen(fen: &str) -> Result<Self, String> {
        let mut pezzi = [0u64; 6];
        let mut colori = [0u64; 2];
        let parts: Vec<&str> = fen.split_whitespace().collect();
        let ranks: Vec<&str> = parts[0].split('/').collect();
        
        for (r, rank_str) in ranks.iter().enumerate() {
            let mut file = 0; let rank = 7 - r;
            for c in rank_str.chars() {
                if let Some(digit) = c.to_digit(10) { file += digit as usize; }
                else {
                    let color = if c.is_uppercase() { Colore::Bianco } else { Colore::Nero };
                    let p = match c.to_ascii_lowercase() {
                        'p'=>Pezzo::Pedone, 'n'=>Pezzo::Cavallo, 'b'=>Pezzo::Alfiere,
                        'r'=>Pezzo::Torre, 'q'=>Pezzo::Regina, 'k'=>Pezzo::Re, _ => Pezzo::Pedone
                    };
                    pezzi[p.indice()] |= 1u64 << (rank * 8 + file);
                    colori[color.indice()] |= 1u64 << (rank * 8 + file);
                    file += 1;
                }
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

        let ep_square = if parts.len() > 3 && parts[3] != "-" {
            let s = parts[3];
            let f = s.chars().nth(0).unwrap() as u8 - b'a';
            let r = s.chars().nth(1).unwrap() as u8 - b'1';
            Some(r * 8 + f)
        } else {
            None
        };

        let mut s = Scacchiera { 
            pezzi, colori, turno, hash: 0, 
            diritti_arrocco: diritti, 
            ep_square,
            history: Vec::new(), 
            current_acc: [NNUEAccumulator { v: [0; 1536] }; 2] 
        };
        s.hash = s.genera_hash_completo();
        Ok(s)
    }

    pub fn genera_hash_completo(&self) -> u64 {
        let mut h = 0;
        for c in 0..2 { for p in 0..6 {
            let mut bb = self.pezzi[p] & self.colori[c];
            while bb != 0 { 
                h ^= ZOBRIST_TABLE.pezzi[c][p][bb.trailing_zeros() as usize]; 
                bb &= bb - 1; 
            }
        }}
        if self.turno == Colore::Nero { h ^= ZOBRIST_TABLE.turno; }
        h ^= ZOBRIST_TABLE.arrocco[self.diritti_arrocco as usize];
        if let Some(ep) = self.ep_square {
            h ^= ZOBRIST_TABLE.ep_file[(ep % 8) as usize];
        }
        h
    }

    pub fn genera_mosse(&self) -> Vec<Mossa> {
        crate::movegen::genera_mosse(self)
    }

    // Inizializza l'accumulatore da zero (usato se ricarichiamo la rete)
    pub fn inizializza_acc_nnue(&mut self, net: &crate::nnue::Network) {
        self.current_acc[0].v.copy_from_slice(&net.feature_bias);
        self.current_acc[1].v.copy_from_slice(&net.feature_bias);

        for sq in 0..64 {
            if let Some(p) = self.get_piece_at(sq) {
                let color = if (self.colori[0] & (1u64 << sq)) != 0 { 
                    Colore::Bianco 
                } else { 
                    Colore::Nero 
                };
                // Qui passiamo Some(net) perché l'abbiamo sicuramente
                self.aggiorna_nnue_aggiungi(p, color, sq, Some(net));
            }
        }
    }

    // --- NUOVO METODO PER UCI ---
    // Esegue una mossa senza aggiornare la NNUE (utile per il setup posizionale)
    pub fn esegui_mossa_raw(&mut self, m: &Mossa) -> bool {
        self.esegui_mossa(m, None)
    }

    // Firma modificata: accetta Option<&Network>
    pub fn esegui_mossa(&mut self, m: &Mossa, net: Option<&crate::nnue::Network>) -> bool {
        let (us, from, to) = (self.turno, m.da(), m.a());
        let piece = match self.get_piece_at(from) { Some(p) => p, None => return false };
        let mut cap = self.get_piece_at(to);
        let flag = m.move_flag();

        if flag == MoveFlag::EnPassant {
            cap = Some(Pezzo::Pedone);
        }

        self.history.push((self.diritti_arrocco, cap, self.hash, self.ep_square));

        self.aggiorna_nnue_rimuovi(piece, us, from, net);
        if let Some(cp) = cap { 
            let cap_sq = if flag == MoveFlag::EnPassant {
                if us == Colore::Bianco { to - 8 } else { to + 8 }
            } else { 
                to 
            };
            self.aggiorna_nnue_rimuovi(cp, us.opposto(), cap_sq, net);
        }

        if let Some(cp) = cap {
            let cap_sq = if flag == MoveFlag::EnPassant {
                 if us == Colore::Bianco { to - 8 } else { to + 8 }
            } else { to };

            self.pezzi[cp.indice()] &= !(1u64 << cap_sq);
            self.colori[us.opposto().indice()] &= !(1u64 << cap_sq);
            self.hash ^= ZOBRIST_TABLE.pezzi[us.opposto().indice()][cp.indice()][cap_sq];
        }

        self.toggle_piece(us, piece, from);
        let piece_on_dest = if flag == MoveFlag::Promotion { Pezzo::Regina } else { piece };
        self.toggle_piece(us, piece_on_dest, to);

        self.hash ^= ZOBRIST_TABLE.pezzi[us.indice()][piece.indice()][from];
        self.hash ^= ZOBRIST_TABLE.pezzi[us.indice()][piece_on_dest.indice()][to];

        if flag == MoveFlag::Castle {
            let (rook_from, rook_to) = match to {
                6 => (7, 5),   
                2 => (0, 3),   
                62 => (63, 61),
                58 => (56, 59),
                _ => (0, 0) 
            };
            self.aggiorna_nnue_rimuovi(Pezzo::Torre, us, rook_from, net);
            self.aggiorna_nnue_aggiungi(Pezzo::Torre, us, rook_to, net);
            self.toggle_piece(us, Pezzo::Torre, rook_from);
            self.toggle_piece(us, Pezzo::Torre, rook_to);
            self.hash ^= ZOBRIST_TABLE.pezzi[us.indice()][Pezzo::Torre.indice()][rook_from];
            self.hash ^= ZOBRIST_TABLE.pezzi[us.indice()][Pezzo::Torre.indice()][rook_to];
        }

        self.aggiorna_nnue_aggiungi(piece_on_dest, us, to, net);

        self.hash ^= ZOBRIST_TABLE.arrocco[self.diritti_arrocco as usize];
        self.aggiorna_diritti(from, to);
        self.hash ^= ZOBRIST_TABLE.arrocco[self.diritti_arrocco as usize];

        if let Some(ep) = self.ep_square {
             self.hash ^= ZOBRIST_TABLE.ep_file[(ep % 8) as usize];
        }
        
        if flag == MoveFlag::DoublePawnPush {
            let ep_sq = (from + to) / 2; 
            self.ep_square = Some(ep_sq as u8);
            self.hash ^= ZOBRIST_TABLE.ep_file[(ep_sq % 8)];
        } else {
            self.ep_square = None;
        }

        self.hash ^= ZOBRIST_TABLE.turno;
        self.turno = us.opposto();
        
        true
    }

    // Firma modificata: accetta Option<&Network>
    pub fn annulla_mossa(&mut self, m: &Mossa, _cap_unused: Option<Pezzo>, net: Option<&crate::nnue::Network>) {
        if let Some((dir, cap, h, ep_old)) = self.history.pop() {
            let us = self.turno.opposto();
            let (from, to) = (m.da(), m.a());
            let flag = m.move_flag();

            let piece_on_board = if flag == MoveFlag::Promotion { Pezzo::Regina } else { 
                self.get_piece_at(to).unwrap()
            };
            
            let piece_moved = if flag == MoveFlag::Promotion { Pezzo::Pedone } else { piece_on_board };

            self.aggiorna_nnue_rimuovi(piece_on_board, us, to, net);
            self.toggle_piece(us, piece_on_board, to);

            if flag == MoveFlag::Castle {
                let (rook_from, rook_to) = match to {
                    6 => (7, 5), 2 => (0, 3), 62 => (63, 61), 58 => (56, 59), _ => (0,0)
                };
                self.aggiorna_nnue_rimuovi(Pezzo::Torre, us, rook_to, net);
                self.toggle_piece(us, Pezzo::Torre, rook_to); 
                
                self.aggiorna_nnue_aggiungi(Pezzo::Torre, us, rook_from, net);
                self.toggle_piece(us, Pezzo::Torre, rook_from); 
            }

            self.aggiorna_nnue_aggiungi(piece_moved, us, from, net);
            self.toggle_piece(us, piece_moved, from);

            if let Some(cp) = cap {
                let cap_sq = if flag == MoveFlag::EnPassant {
                     if us == Colore::Bianco { to - 8 } else { to + 8 }
                } else { to };
                
                self.aggiorna_nnue_aggiungi(cp, us.opposto(), cap_sq, net);
                self.pezzi[cp.indice()] |= 1u64 << cap_sq;
                self.colori[us.opposto().indice()] |= 1u64 << cap_sq;
            }

            self.diritti_arrocco = dir;
            self.hash = h;
            self.ep_square = ep_old;
            self.turno = us;
        }
    }

    pub fn get_piece_at(&self, sq: usize) -> Option<Pezzo> {
        let bit = 1u64 << sq;
        for p in 0..6 { if (self.pezzi[p] & bit) != 0 { return Some(unsafe { std::mem::transmute(p as u8) }); } }
        None
    }

    pub fn toggle_piece(&mut self, c: Colore, p: Pezzo, s: usize) {
        let b = 1u64 << s; 
        self.pezzi[p.indice()] ^= b; 
        self.colori[c.indice()] ^= b;
    }

    pub fn occupazione(&self) -> Bitboard { self.colori[0] | self.colori[1] }
    
    pub fn re_in_scacco(&self, c: Colore) -> bool {
        let k = self.pezzi[5] & self.colori[c.indice()];
        if k == 0 { return false; }
        self.casella_attaccata(k.trailing_zeros() as usize, c.opposto())
    }

    pub fn casella_attaccata(&self, s: usize, by: Colore) -> bool {
        let occ = self.occupazione();
        let enemy_pieces = self.colori[by.indice()];
        
        if (pawn_attacks(s, by.opposto()) & (self.pezzi[0] & enemy_pieces)) != 0 { return true; }
        if (knight_attacks(s) & (self.pezzi[1] & enemy_pieces)) != 0 { return true; }
        if (king_attacks(s) & (self.pezzi[5] & enemy_pieces)) != 0 { return true; }
        
        let queens = self.pezzi[4] & enemy_pieces;
        let bishops = self.pezzi[2] & enemy_pieces;
        let rooks = self.pezzi[3] & enemy_pieces;

        if (bishop_attacks(s, occ) & (bishops | queens)) != 0 { return true; }
        if (rook_attacks(s, occ) & (rooks | queens)) != 0 { return true; }
        
        false
    }

    pub fn get_attacks_for_piece(&self, p: Pezzo, sq: usize, occ: Bitboard) -> Bitboard {
        match p {
            Pezzo::Cavallo => knight_attacks(sq), 
            Pezzo::Alfiere => bishop_attacks(sq, occ),
            Pezzo::Torre => rook_attacks(sq, occ), 
            Pezzo::Regina => queen_attacks(sq, occ),
            Pezzo::Re => king_attacks(sq), 
            _ => 0
        }
    }

    fn aggiorna_diritti(&mut self, from: usize, to: usize) {
        if from == 4 || to == 4 { self.diritti_arrocco &= !3; } // Bianco Re
        if from == 60 || to == 60 { self.diritti_arrocco &= !12; } // Nero Re
        
        if from == 0 || to == 0 { self.diritti_arrocco &= !2; } // Torre a1
        if from == 7 || to == 7 { self.diritti_arrocco &= !1; } // Torre h1
        if from == 56 || to == 56 { self.diritti_arrocco &= !8; } // Torre a8
        if from == 63 || to == 63 { self.diritti_arrocco &= !4; } // Torre h8
    }

    // --- AGGIORNAMENTO NNUE PROTETTO (Gestisce Option) ---
    pub fn aggiorna_nnue_aggiungi(&mut self, p: Pezzo, c: Colore, sq: usize, net: Option<&crate::nnue::Network>) {
        if let Some(net) = net {
            let idx_w = crate::nnue::get_feature_index(p, c, sq);
            let idx_b = crate::nnue::get_feature_index(p, c, sq ^ 56);
            let w_w = &net.feature_weights[idx_w * 1536..(idx_w + 1) * 1536];
            let w_b = &net.feature_weights[idx_b * 1536..(idx_b + 1) * 1536];
            for i in 0..1536 { 
                self.current_acc[0].v[i] = self.current_acc[0].v[i].wrapping_add(w_w[i]); 
                self.current_acc[1].v[i] = self.current_acc[1].v[i].wrapping_add(w_b[i]); 
            }
        }
    }

    pub fn aggiorna_nnue_rimuovi(&mut self, p: Pezzo, c: Colore, sq: usize, net: Option<&crate::nnue::Network>) {
        if let Some(net) = net {
            let idx_w = crate::nnue::get_feature_index(p, c, sq);
            let idx_b = crate::nnue::get_feature_index(p, c, sq ^ 56);
            let w_w = &net.feature_weights[idx_w * 1536..(idx_w + 1) * 1536];
            let w_b = &net.feature_weights[idx_b * 1536..(idx_b + 1) * 1536];
            for i in 0..1536 { 
                self.current_acc[0].v[i] = self.current_acc[0].v[i].wrapping_sub(w_w[i]); 
                self.current_acc[1].v[i] = self.current_acc[1].v[i].wrapping_sub(w_b[i]); 
            }
        }
    }
}