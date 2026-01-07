use std::fmt;

// Esportiamo Bitboard come alias di u64
pub type Bitboard = u64;

// --- DEFINIZIONI BITBOARD E UTILS ---

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
pub enum MoveFlag { Quiet, Capture, Castling, EnPassant, Promotion }
impl MoveFlag {
    pub fn is_capture(&self) -> bool {
        matches!(self, MoveFlag::Capture | MoveFlag::EnPassant)
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct Mossa { pub data: u16 }

impl Mossa {
    pub fn nuova(da: usize, a: usize, flag: u16, prom: u16) -> Self {
        let d = (da as u16) | ((a as u16) << 6) | (flag << 12) | (prom << 14);
        Self { data: d }
    }
    // Alias per compatibilità con vecchio codice se necessario
    pub fn new(da: usize, a: usize, flag: MoveFlag) -> Self {
        let f = match flag {
            MoveFlag::Quiet => 0, MoveFlag::Capture => 1, MoveFlag::Castling => 2,
            MoveFlag::EnPassant => 3, MoveFlag::Promotion => 0, // Promo gestita a parte solitamente
        };
        Self::nuova(da, a, f, 0)
    }

    pub fn da(&self) -> usize { (self.data & 0x3F) as usize }
    pub fn a(&self) -> usize { ((self.data >> 6) & 0x3F) as usize }
    pub fn move_flag(&self) -> MoveFlag {
        match (self.data >> 12) & 0x3 {
            1 => MoveFlag::Capture,
            2 => MoveFlag::Castling,
            3 => MoveFlag::EnPassant,
            _ => MoveFlag::Quiet,
        }
    }
    pub fn is_promotion(&self) -> bool { ((self.data >> 14) & 0x3) != 0 }
    pub fn promotion_piece(&self) -> Option<Pezzo> {
        match (self.data >> 14) & 0x3 {
            1 => Some(Pezzo::Cavallo),
            2 => Some(Pezzo::Alfiere),
            3 => Some(Pezzo::Torre), 
            _ => Some(Pezzo::Regina) 
        }
    }
}

impl fmt::Display for Mossa {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let cols = ['a','b','c','d','e','f','g','h'];
        let da = self.da();
        let a = self.a();
        let mut s = format!("{}{}{}{}", cols[da%8], da/8+1, cols[a%8], a/8+1);
        
        if self.is_promotion() {
             let p_char = match (self.data >> 14) & 0x3 {
                 1 => 'n', 2 => 'b', 3 => 'r', _ => 'q'
             };
             s.push(p_char);
        }
        write!(f, "{}", s)
    }
}

// --- TABELLE PRECALCOLATE ---
static mut KNIGHT_MOVES: [u64; 64] = [0; 64];
static mut KING_MOVES: [u64; 64] = [0; 64];

fn init_tables() {
    unsafe {
        if KNIGHT_MOVES[0] != 0 { return; } 
        
        for sq in 0..64 {
            let mut b: u64 = 0;
            let r = (sq / 8) as i8;
            let c = (sq % 8) as i8;
            
            // Cavallo
            let km = [(r+2, c+1), (r+2, c-1), (r-2, c+1), (r-2, c-1),
                      (r+1, c+2), (r+1, c-2), (r-1, c+2), (r-1, c-2)];
            for (nr, nc) in km {
                if nr >= 0 && nr < 8 && nc >= 0 && nc < 8 { 
                    b |= 1u64 << (nr*8 + nc); 
                }
            }
            KNIGHT_MOVES[sq] = b;

            // Re
            let mut k = 0;
            let kmov = [(r+1,c), (r+1,c+1), (r+1,c-1), 
                        (r,c+1), (r,c-1),
                        (r-1,c), (r-1,c+1), (r-1,c-1)];
            for (nr, nc) in kmov {
                if nr >= 0 && nr < 8 && nc >= 0 && nc < 8 { 
                    k |= 1u64 << (nr*8 + nc); 
                }
            }
            KING_MOVES[sq] = k;
        }
    }
}

// --- STRUTTURA SCACCHIERA ---

#[derive(Clone)]
pub struct Scacchiera {
    pub pezzi: [u64; 6],     
    pub colori: [u64; 2],    
    pub turno: Colore,
    pub en_passant: Option<usize>,
    pub castling: u8, // Bitmask: 1=WK, 2=WQ, 4=BK, 8=BQ
}

impl Scacchiera {
    pub fn nuova() -> Self {
        init_tables(); 
        let mut s = Self {
            pezzi: [0; 6],
            colori: [0; 2],
            turno: Colore::Bianco,
            en_passant: None,
            castling: 15,
        };
        // Setup Fen Iniziale
        s.pezzi[Pezzo::Pedone.indice()] = 0x00FF00000000FF00;
        s.pezzi[Pezzo::Cavallo.indice()] = 0x4200000000000042;
        s.pezzi[Pezzo::Alfiere.indice()] = 0x2400000000000024;
        s.pezzi[Pezzo::Torre.indice()] = 0x8100000000000081;
        s.pezzi[Pezzo::Regina.indice()] = 0x0800000000000008;
        s.pezzi[Pezzo::Re.indice()] = 0x1000000000000010;

        s.colori[Colore::Bianco.indice()] = 0x000000000000FFFF;
        s.colori[Colore::Nero.indice()] = 0xFFFF000000000000;
        s
    }

    pub fn get_piece_at(&self, sq: usize) -> Option<Pezzo> {
        let mask = 1u64 << sq;
        if (self.colori[0] | self.colori[1]) & mask == 0 { return None; }
        for p in 0..6 {
            if (self.pezzi[p] & mask) != 0 {
                return match p {
                    0 => Some(Pezzo::Pedone), 1 => Some(Pezzo::Cavallo), 2 => Some(Pezzo::Alfiere),
                    3 => Some(Pezzo::Torre), 4 => Some(Pezzo::Regina), _ => Some(Pezzo::Re),
                };
            }
        }
        None
    }

    pub fn re_in_scacco(&self, c: Colore) -> bool {
        let king_bb = self.pezzi[Pezzo::Re.indice()] & self.colori[c.indice()];
        if king_bb == 0 { return false; }
        let king_sq = king_bb.trailing_zeros() as usize;
        self.casella_attaccata(king_sq, c.opposto())
    }

    pub fn casella_attaccata(&self, sq: usize, by_color: Colore) -> bool {
        let enemy_pieces = self.colori[by_color.indice()];
        let occ = self.colori[0] | self.colori[1];

        // 1. Attacchi Pedoni
        let pawns = self.pezzi[Pezzo::Pedone.indice()] & enemy_pieces;
        if by_color == Colore::Bianco {
            // Pedoni bianchi attaccano verso l'alto (rank + 1)
            // Se sono bianco, attacco sq-7 e sq-9
            // Se sq è la casella bersaglio, il pedone attaccante è in sq-7 o sq-9
            // Aspetta: se "by_color" è Bianco, i pedoni SONO bianchi.
            // Un pedone bianco in x attacca x+7 e x+9.
            // Quindi se voglio sapere se 'sq' è attaccata da un bianco:
            if sq >= 9 && (pawns & (1u64 << (sq - 9))) != 0 && (sq % 8 != 0) { return true; } 
            if sq >= 7 && (pawns & (1u64 << (sq - 7))) != 0 && (sq % 8 != 7) { return true; } 
        } else {
            // Pedoni neri attaccano verso il basso
            if sq <= 54 && (pawns & (1u64 << (sq + 9))) != 0 && (sq % 8 != 7) { return true; }
            if sq <= 56 && (pawns & (1u64 << (sq + 7))) != 0 && (sq % 8 != 0) { return true; }
        }

        // 2. Attacchi Cavallo
        let knights = self.pezzi[Pezzo::Cavallo.indice()] & enemy_pieces;
        unsafe {
            if (KNIGHT_MOVES[sq] & knights) != 0 { return true; }
        }

        // 3. Attacchi Re
        let king = self.pezzi[Pezzo::Re.indice()] & enemy_pieces;
        unsafe {
            if (KING_MOVES[sq] & king) != 0 { return true; }
        }

        // 4. Sliding
        let rooks_queens = (self.pezzi[Pezzo::Torre.indice()] | self.pezzi[Pezzo::Regina.indice()]) & enemy_pieces;
        let bishops_queens = (self.pezzi[Pezzo::Alfiere.indice()] | self.pezzi[Pezzo::Regina.indice()]) & enemy_pieces;

        if rooks_queens != 0 {
            if (self.get_rook_attacks(sq, occ) & rooks_queens) != 0 { return true; }
        }
        if bishops_queens != 0 {
            if (self.get_bishop_attacks(sq, occ) & bishops_queens) != 0 { return true; }
        }

        false
    }

    fn get_rook_attacks(&self, sq: usize, occ: u64) -> u64 {
        let mut attacks = 0;
        let r = sq / 8; let c = sq % 8;
        for i in (c+1)..8 { let b = 1u64 << (r*8+i); attacks |= b; if (occ & b) != 0 { break; } }
        for i in (0..c).rev() { let b = 1u64 << (r*8+i); attacks |= b; if (occ & b) != 0 { break; } }
        for i in (r+1)..8 { let b = 1u64 << (i*8+c); attacks |= b; if (occ & b) != 0 { break; } }
        for i in (0..r).rev() { let b = 1u64 << (i*8+c); attacks |= b; if (occ & b) != 0 { break; } }
        attacks
    }
    
    fn get_bishop_attacks(&self, sq: usize, occ: u64) -> u64 {
        let mut attacks = 0;
        let r = sq / 8; let c = sq % 8;
        let mut i = 1; while r+i < 8 && c+i < 8 { let b = 1u64 << ((r+i)*8 + c+i); attacks |= b; if (occ & b) != 0 { break; } i+=1; }
        let mut i = 1; while r+i < 8 && c >= i { let b = 1u64 << ((r+i)*8 + c-i); attacks |= b; if (occ & b) != 0 { break; } i+=1; }
        let mut i = 1; while r >= i && c+i < 8 { let b = 1u64 << ((r-i)*8 + c+i); attacks |= b; if (occ & b) != 0 { break; } i+=1; }
        let mut i = 1; while r >= i && c >= i { let b = 1u64 << ((r-i)*8 + c-i); attacks |= b; if (occ & b) != 0 { break; } i+=1; }
        attacks
    }

    pub fn genera_mosse(&self) -> Vec<Mossa> {
        let mut mosse = Vec::with_capacity(64);
        let us = self.turno;
        let us_idx = us.indice();
        let my_pieces = self.colori[us_idx];
        let occ = self.colori[0] | self.colori[1];

        for sq in 0..64 {
            if (my_pieces & (1u64 << sq)) == 0 { continue; }
            let p = self.get_piece_at(sq).unwrap();

            match p {
                Pezzo::Pedone => self.gen_pawn_moves(sq, us, occ, &mut mosse),
                Pezzo::Cavallo => {
                    unsafe {
                        let moves = KNIGHT_MOVES[sq] & !my_pieces;
                        self.serialize_moves(sq, moves, &mut mosse, occ);
                    }
                },
                Pezzo::Re => {
                    unsafe {
                        let moves = KING_MOVES[sq] & !my_pieces;
                        self.serialize_moves(sq, moves, &mut mosse, occ);
                    }
                    // Arrocco
                    self.gen_castling(us, occ, &mut mosse);
                },
                Pezzo::Alfiere => {
                    let moves = self.get_bishop_attacks(sq, occ) & !my_pieces;
                    self.serialize_moves(sq, moves, &mut mosse, occ);
                },
                Pezzo::Torre => {
                    let moves = self.get_rook_attacks(sq, occ) & !my_pieces;
                    self.serialize_moves(sq, moves, &mut mosse, occ);
                },
                Pezzo::Regina => {
                    let moves = (self.get_bishop_attacks(sq, occ) | self.get_rook_attacks(sq, occ)) & !my_pieces;
                    self.serialize_moves(sq, moves, &mut mosse, occ);
                },
            }
        }
        mosse
    }

    fn gen_castling(&self, c: Colore, occ: u64, list: &mut Vec<Mossa>) {
        // Verifica preliminare: Re non deve essere sotto scacco
        if self.re_in_scacco(c) { return; }

        let them = c.opposto();

        if c == Colore::Bianco {
            // Kingside (e1 -> g1)
            if (self.castling & 1) != 0 {
                // Check empty f1, g1
                if (occ & 0x60) == 0 {
                    // Check squares not attacked (f1, g1)
                    if !self.casella_attaccata(5, them) && !self.casella_attaccata(6, them) {
                        list.push(Mossa::nuova(4, 6, 2, 0)); // 2 = Castling Flag
                    }
                }
            }
            // Queenside (e1 -> c1)
            if (self.castling & 2) != 0 {
                // Check empty b1, c1, d1
                if (occ & 0x0E) == 0 {
                    // Check squares not attacked (c1, d1) - b1 non importa per regole FIDE ma deve essere vuota
                    if !self.casella_attaccata(3, them) && !self.casella_attaccata(2, them) {
                        list.push(Mossa::nuova(4, 2, 2, 0));
                    }
                }
            }
        } else {
            // Kingside (e8 -> g8)
            if (self.castling & 4) != 0 {
                if (occ & 0x6000000000000000) == 0 {
                    if !self.casella_attaccata(61, them) && !self.casella_attaccata(62, them) {
                        list.push(Mossa::nuova(60, 62, 2, 0));
                    }
                }
            }
            // Queenside (e8 -> c8)
            if (self.castling & 8) != 0 {
                if (occ & 0x0E00000000000000) == 0 {
                    if !self.casella_attaccata(59, them) && !self.casella_attaccata(58, them) {
                        list.push(Mossa::nuova(60, 58, 2, 0));
                    }
                }
            }
        }
    }

    fn gen_pawn_moves(&self, sq: usize, c: Colore, occ: u64, list: &mut Vec<Mossa>) {
        let r = sq / 8;
        let c_idx = sq % 8;
        let forward = if c == Colore::Bianco { 8 } else { -8_i32 } as isize;
        let start_rank = if c == Colore::Bianco { 1 } else { 6 };
        let prom_rank = if c == Colore::Bianco { 7 } else { 0 };

        // 1 passo
        let to = (sq as isize + forward) as usize;
        if to < 64 && (occ & (1u64 << to)) == 0 {
            if r == prom_rank {
                for prom in [4, 3, 2, 1] { 
                    list.push(Mossa::nuova(sq, to, 0, prom as u16));
                }
            } else {
                list.push(Mossa::nuova(sq, to, 0, 0));
                // 2 passi
                if r == start_rank {
                    let to2 = (sq as isize + forward * 2) as usize;
                    if (occ & (1u64 << to2)) == 0 {
                        list.push(Mossa::nuova(sq, to2, 0, 0)); 
                    }
                }
            }
        }

        // Catture
        let attacks = [c_idx as isize - 1, c_idx as isize + 1];
        for dc in attacks {
            if dc >= 0 && dc < 8 {
                let to_cap = ((r as isize + if c==Colore::Bianco {1} else {-1}) * 8 + dc) as usize;
                let enemy_color = self.colori[c.opposto().indice()];
                if (enemy_color & (1u64 << to_cap)) != 0 {
                    if r == prom_rank {
                        for prom in [4, 3, 2, 1] { 
                           list.push(Mossa::nuova(sq, to_cap, 1, prom as u16)); 
                        }
                    } else {
                        list.push(Mossa::nuova(sq, to_cap, 1, 0));
                    }
                }
                // En Passant
                if let Some(ep_sq) = self.en_passant {
                    if to_cap == ep_sq {
                         list.push(Mossa::nuova(sq, to_cap, 3, 0)); 
                    }
                }
            }
        }
    }

    fn serialize_moves(&self, from: usize, mut moves_bb: u64, list: &mut Vec<Mossa>, occ: u64) {
        while moves_bb != 0 {
            let to = moves_bb.trailing_zeros() as usize;
            let is_capture = (occ & (1u64 << to)) != 0;
            list.push(Mossa::nuova(from, to, if is_capture { 1 } else { 0 }, 0));
            moves_bb &= moves_bb - 1;
        }
    }

    pub fn esegui_mossa(&mut self, m: &Mossa, _prev_state: Option<()>) -> bool {
        let us = self.turno;
        let them = us.opposto();
        let from = m.da();
        let to = m.a();
        let flag = m.move_flag();
        
        let mut piece_type = match self.get_piece_at(from) {
            Some(p) => p,
            None => return false,
        };
        
        // --- GESTIONE ARROCCO (Diritti) ---
        // Se muove il Re, perde diritti
        if piece_type == Pezzo::Re {
            if us == Colore::Bianco { self.castling &= !3; } else { self.castling &= !12; }
        }
        // Se muove la Torre, perde diritti su quel lato
        if piece_type == Pezzo::Torre {
            if us == Colore::Bianco {
                if from == 0 { self.castling &= !2; } // a1
                if from == 7 { self.castling &= !1; } // h1
            } else {
                if from == 56 { self.castling &= !8; } // a8
                if from == 63 { self.castling &= !4; } // h8
            }
        }
        // Se viene catturata una torre
        if flag == MoveFlag::Capture {
            if to == 0 { self.castling &= !2; } // Captured a1
            if to == 7 { self.castling &= !1; } // Captured h1
            if to == 56 { self.castling &= !8; } // Captured a8
            if to == 63 { self.castling &= !4; } // Captured h8
        }

        // --- MOVIMENTO BASE ---
        self.pezzi[piece_type.indice()] &= !(1u64 << from);
        self.colori[us.indice()] &= !(1u64 << from);

        if flag == MoveFlag::Capture {
            let mut captured_type = None;
            for p in 0..6 {
                if (self.pezzi[p] & (1u64 << to)) != 0 {
                    self.pezzi[p] &= !(1u64 << to);
                    captured_type = Some(p);
                    break;
                }
            }
            if captured_type.is_some() {
                self.colori[them.indice()] &= !(1u64 << to);
            }
        } else if flag == MoveFlag::EnPassant {
            let cap_sq = if us == Colore::Bianco { to - 8 } else { to + 8 };
            self.pezzi[Pezzo::Pedone.indice()] &= !(1u64 << cap_sq);
            self.colori[them.indice()] &= !(1u64 << cap_sq);
        } else if flag == MoveFlag::Castling {
            // Muovi la torre
            let (r_from, r_to) = match to {
                6 => (7, 5),   // White KS (h1->f1)
                2 => (0, 3),   // White QS (a1->d1)
                62 => (63, 61),// Black KS (h8->f8)
                58 => (56, 59),// Black QS (a8->d8)
                _ => (0, 0) // Should not happen
            };
            self.pezzi[Pezzo::Torre.indice()] &= !(1u64 << r_from);
            self.colori[us.indice()] &= !(1u64 << r_from);
            self.pezzi[Pezzo::Torre.indice()] |= 1u64 << r_to;
            self.colori[us.indice()] |= 1u64 << r_to;
        }

        if m.is_promotion() {
            piece_type = m.promotion_piece().unwrap_or(Pezzo::Regina);
        }

        self.pezzi[piece_type.indice()] |= 1u64 << to;
        self.colori[us.indice()] |= 1u64 << to;

        if self.re_in_scacco(us) {
            return false;
        }

        self.turno = them;
        self.en_passant = None; 
        
        if piece_type == Pezzo::Pedone && (from as isize - to as isize).abs() == 16 {
            self.en_passant = Some((from + to) / 2);
        }

        true
    }
    
    pub fn annulla_mossa(&mut self, _m: &Mossa, _captured: Option<Pezzo>, _ep: Option<usize>) {
    }

    pub fn make_null_move(&mut self) {
        self.turno = self.turno.opposto();
        self.en_passant = None;
    }
}