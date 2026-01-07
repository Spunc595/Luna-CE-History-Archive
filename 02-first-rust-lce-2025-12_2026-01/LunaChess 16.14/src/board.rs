use std::fmt;

// Esportiamo Bitboard come alias di u64 per compatibilità con altri file (es. nnue.rs)
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
    // Aggiungo 'new' come alias di 'nuova' per compatibilità se serve, ma meglio usare nuova
    pub fn nuova(da: usize, a: usize, flag: u16, prom: u16) -> Self {
        let d = (da as u16) | ((a as u16) << 6) | (flag << 12) | (prom << 14);
        Self { data: d }
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
            // Usiamo i8 per poter gestire i numeri negativi (fuori scacchiera)
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
    pub castling: u8, 
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
        // Se il re non c'è (es. posizione illegale caricata), ritorna false per evitare crash
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
            if sq >= 9 && (pawns & (1u64 << (sq - 9))) != 0 && (sq % 8 != 0) { return true; } 
            if sq >= 7 && (pawns & (1u64 << (sq - 7))) != 0 && (sq % 8 != 7) { return true; } 
        } else {
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
                        // Arrocco semplificato o omesso per sicurezza in questo stadio
                    }
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
            None => return false, // Sicurezza
        };
        
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
    
    // Funzione vuota per compatibilità con il codice che la chiama, 
    // visto che nel search usiamo clone() l'annulla non serve più realmente,
    // ma serve che il metodo esista per evitare errori di compilazione se non si aggiorna tutto.
    pub fn annulla_mossa(&mut self, _m: &Mossa, _captured: Option<Pezzo>, _ep: Option<usize>) {
    }

    pub fn make_null_move(&mut self) {
        self.turno = self.turno.opposto();
        self.en_passant = None;
        // In un motore completo qui bisognerebbe gestire la regola delle 50 mosse e l'arrocco,
        // ma per la stabilità corrente e dato che cloniamo, questo è sufficiente.
    }
}