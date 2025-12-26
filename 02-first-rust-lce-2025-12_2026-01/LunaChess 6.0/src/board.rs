use std::fmt;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use lazy_static::lazy_static;
use rand::Rng;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Colore {
    Bianco,
    Nero,
}

impl Colore {
    pub fn opposto(&self) -> Colore {
        match self {
            Colore::Bianco => Colore::Nero,
            Colore::Nero => Colore::Bianco,
        }
    }
    
    pub fn indice(&self) -> usize {
        match self {
            Colore::Bianco => 0,
            Colore::Nero => 1,
        }
    }
    
    pub fn direzione_pedone(&self) -> i32 {
        match self {
            Colore::Bianco => 1,
            Colore::Nero => -1,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Pezzo {
    Pedone,
    Cavallo,
    Alfiere,
    Torre,
    Regina,
    Re,
}

impl Pezzo {
    pub fn valore(&self) -> i32 {
        match self {
            Pezzo::Pedone => 100,
            Pezzo::Cavallo => 320,
            Pezzo::Alfiere => 330,
            Pezzo::Torre => 500,
            Pezzo::Regina => 900,
            Pezzo::Re => 20000,
        }
    }
    
    pub fn simbolo(&self, colore: Colore) -> char {
        let c = match self {
            Pezzo::Pedone => 'p',
            Pezzo::Cavallo => 'n',
            Pezzo::Alfiere => 'b',
            Pezzo::Torre => 'r',
            Pezzo::Regina => 'q',
            Pezzo::Re => 'k',
        };
        
        if colore == Colore::Bianco {
            c.to_ascii_uppercase()
        } else {
            c
        }
    }
    
    pub fn indice(&self) -> usize {
        match self {
            Pezzo::Pedone => 0,
            Pezzo::Cavallo => 1,
            Pezzo::Alfiere => 2,
            Pezzo::Torre => 3,
            Pezzo::Regina => 4,
            Pezzo::Re => 5,
        }
    }
    
    pub fn from_char(c: char) -> Option<(Pezzo, Colore)> {
        match c {
            'P' => Some((Pezzo::Pedone, Colore::Bianco)),
            'N' => Some((Pezzo::Cavallo, Colore::Bianco)),
            'B' => Some((Pezzo::Alfiere, Colore::Bianco)),
            'R' => Some((Pezzo::Torre, Colore::Bianco)),
            'Q' => Some((Pezzo::Regina, Colore::Bianco)),
            'K' => Some((Pezzo::Re, Colore::Bianco)),
            'p' => Some((Pezzo::Pedone, Colore::Nero)),
            'n' => Some((Pezzo::Cavallo, Colore::Nero)),
            'b' => Some((Pezzo::Alfiere, Colore::Nero)),
            'r' => Some((Pezzo::Torre, Colore::Nero)),
            'q' => Some((Pezzo::Regina, Colore::Nero)),
            'k' => Some((Pezzo::Re, Colore::Nero)),
            _ => None,
        }
    }
}

pub type Bitboard = u64;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct Casella {
    pub file: u8,
    pub rank: u8,
}

impl Casella {
    pub fn nuova(file: u8, rank: u8) -> Option<Self> {
        if file < 8 && rank < 8 {
            Some(Casella { file, rank })
        } else {
            None
        }
    }
    
    pub fn da_string(s: &str) -> Option<Self> {
        if s.len() != 2 {
            return None;
        }
        
        let chars: Vec<char> = s.chars().collect();
        let file_char = chars[0];
        let rank_char = chars[1];
        
        if file_char < 'a' || file_char > 'h' || rank_char < '1' || rank_char > '8' {
            return None;
        }
        
        let file = file_char as u8 - b'a';
        let rank = rank_char as u8 - b'1';
        
        Some(Casella { file, rank })
    }
    
    pub fn to_string(&self) -> String {
        let file_char = (b'a' + self.file) as char;
        let rank_char = (b'1' + self.rank) as char;
        format!("{}{}", file_char, rank_char)
    }
    
    pub fn indice(&self) -> usize {
        (self.rank * 8 + self.file) as usize
    }
    
    pub fn da_indice(indice: usize) -> Option<Self> {
        if indice < 64 {
            let rank = (indice / 8) as u8;
            let file = (indice % 8) as u8;
            Some(Casella { file, rank })
        } else {
            None
        }
    }
    
    pub fn to_bitboard(&self) -> Bitboard {
        1u64 << self.indice()
    }
    
    pub fn file(&self) -> u8 {
        self.file
    }
    
    pub fn rank(&self) -> u8 {
        self.rank
    }
    
    pub fn is_valid(&self) -> bool {
        self.file < 8 && self.rank < 8
    }
}

impl fmt::Display for Casella {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DirittiArrocco {
    pub bianco_lato_re: bool,
    pub bianco_lato_regina: bool,
    pub nero_lato_re: bool,
    pub nero_lato_regina: bool,
}

impl DirittiArrocco {
    pub fn tutti() -> Self {
        DirittiArrocco {
            bianco_lato_re: true,
            bianco_lato_regina: true,
            nero_lato_re: true,
            nero_lato_regina: true,
        }
    }
    
    pub fn nessuno() -> Self {
        DirittiArrocco {
            bianco_lato_re: false,
            bianco_lato_regina: false,
            nero_lato_re: false,
            nero_lato_regina: false,
        }
    }
    
    // Metodo per ottenere diritti come u8 (compatibilità con movegen)
    pub fn to_u8(&self) -> u8 {
        let mut value = 0;
        if self.bianco_lato_re { value |= 1; }
        if self.bianco_lato_regina { value |= 2; }
        if self.nero_lato_re { value |= 4; }
        if self.nero_lato_regina { value |= 8; }
        value
    }
}

impl Default for DirittiArrocco {
    fn default() -> Self {
        DirittiArrocco::nessuno()
    }
}

impl Hash for DirittiArrocco {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.to_u8().hash(state);
    }
}

// ===== MoveFlag ENUM =====

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveFlag {
    None = 0,
    Capture = 1,
    Promotion = 2,
    PromotionCapture = 3,
    EnPassant = 4,
    CastleKingSide = 5,
    CastleQueenSide = 6,
}

impl MoveFlag {
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => MoveFlag::None,
            1 => MoveFlag::Capture,
            2 => MoveFlag::Promotion,
            3 => MoveFlag::PromotionCapture,
            4 => MoveFlag::EnPassant,
            5 => MoveFlag::CastleKingSide,
            6 => MoveFlag::CastleQueenSide,
            _ => MoveFlag::None,
        }
    }
    
    pub fn is_capture(&self) -> bool {
        matches!(self, MoveFlag::Capture | MoveFlag::PromotionCapture | MoveFlag::EnPassant)
    }
    
    pub fn is_promotion(&self) -> bool {
        matches!(self, MoveFlag::Promotion | MoveFlag::PromotionCapture)
    }
}

impl Hash for MoveFlag {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (*self as u8).hash(state);
    }
}

// ===== MOSSA STRUCT =====

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mossa {
    pub da: Casella,
    pub a: Casella,
    pub promozione: Option<Pezzo>,
    pub flag: MoveFlag,
}

impl Mossa {
    pub fn new(da: Casella, a: Casella, promozione: Option<Pezzo>) -> Self {
        let flag = if promozione.is_some() {
            MoveFlag::Promotion
        } else {
            MoveFlag::None
        };
        
        Mossa {
            da,
            a,
            promozione,
            flag,
        }
    }
    
    pub fn new_with_flag(da: Casella, a: Casella, promozione: Option<Pezzo>, flag: MoveFlag) -> Self {
        Mossa {
            da,
            a,
            promozione,
            flag,
        }
    }
    
    pub fn con_promozione(da: Casella, a: Casella, promozione: Pezzo) -> Self {
        Mossa {
            da,
            a,
            promozione: Some(promozione),
            flag: MoveFlag::Promotion,
        }
    }
    
    pub fn en_passant(da: Casella, a: Casella) -> Self {
        Mossa {
            da,
            a,
            promozione: None,
            flag: MoveFlag::EnPassant,
        }
    }
    
    pub fn arrocco(da: Casella, a: Casella) -> Self {
        let flag = if a.file == 6 || a.file == 62 % 8 {
            MoveFlag::CastleKingSide
        } else {
            MoveFlag::CastleQueenSide
        };
        
        Mossa {
            da,
            a,
            promozione: None,
            flag,
        }
    }
    
    // Metodi di accesso (richiesti da movegen)
    pub fn da(&self) -> Casella {
        self.da
    }
    
    pub fn a(&self) -> Casella {
        self.a
    }
    
    pub fn promozione(&self) -> Option<Pezzo> {
        self.promozione
    }
    
    pub fn flag(&self) -> MoveFlag {
        self.flag
    }
    
    // Metodo per ottenere indici (compatibilità)
    pub fn da_indice(&self) -> usize {
        self.da.indice()
    }
    
    pub fn a_indice(&self) -> usize {
        self.a.indice()
    }
    
    pub fn to_string(&self) -> String {
        let mut result = format!("{}{}", self.da, self.a);
        
        if let Some(promo) = self.promozione {
            result.push(match promo {
                Pezzo::Regina => 'q',
                Pezzo::Torre => 'r',
                Pezzo::Alfiere => 'b',
                Pezzo::Cavallo => 'n',
                _ => 'q',
            });
        }
        
        result
    }
    
    pub fn from_uci(uci: &str) -> Option<Self> {
        if uci.len() < 4 {
            return None;
        }
        
        let from_str = &uci[0..2];
        let to_str = &uci[2..4];
        
        let da = Casella::da_string(from_str)?;
        let a = Casella::da_string(to_str)?;
        
        let mut mossa = Mossa::new(da, a, None);
        
        if uci.len() == 5 {
            let promo_char = uci.chars().nth(4)?;
            let promo_pezzo = match promo_char {
                'q' | 'Q' => Pezzo::Regina,
                'r' | 'R' => Pezzo::Torre,
                'b' | 'B' => Pezzo::Alfiere,
                'n' | 'N' => Pezzo::Cavallo,
                _ => return None,
            };
            mossa.promozione = Some(promo_pezzo);
            mossa.flag = MoveFlag::Promotion;
        }
        
        Some(mossa)
    }
    
    // Metodi per encoding/decoding (richiesti da movegen)
    pub fn to_u32(&self) -> u32 {
        let from = self.da.indice() as u32;
        let to = self.a.indice() as u32;
        let promo = self.promozione.map_or(0, |p| p.indice() as u32);
        let flag = self.flag as u8 as u32;
        
        (from << 24) | (to << 18) | (promo << 14) | flag
    }
    
    pub fn from_u32(val: u32) -> Option<Self> {
        let from_idx = ((val >> 24) & 0x3F) as usize;
        let to_idx = ((val >> 18) & 0x3F) as usize;
        let promo_val = ((val >> 14) & 0x0F) as u8;
        let flag_val = (val & 0x0F) as u8;
        
        let from = Casella::da_indice(from_idx)?;
        let to = Casella::da_indice(to_idx)?;
        let promo = if promo_val > 0 {
            match promo_val {
                0 => Some(Pezzo::Regina),
                1 => Some(Pezzo::Torre),
                2 => Some(Pezzo::Alfiere),
                3 => Some(Pezzo::Cavallo),
                _ => None,
            }
        } else {
            None
        };
        
        let flag = MoveFlag::from_u8(flag_val);
        
        Some(Mossa::new_with_flag(from, to, promo, flag))
    }
    
    pub fn to_hash(&self) -> u64 {
        (self.da.indice() as u64) << 16 | (self.a.indice() as u64)
    }
}

impl Hash for Mossa {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.da.hash(state);
        self.a.hash(state);
        self.promozione.hash(state);
        self.flag.hash(state);
    }
}

impl fmt::Display for Mossa {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

// ===== ZOBRIST HASHING =====

// Struttura per hash Zobrist ottimizzato
pub struct ZobristTable {  // Aggiungi 'pub' qui
    pezzi: [[[u64; 64]; 6]; 2],  // [colore][pezzo][casella]
    arrocco: [u64; 4],            // 4 diritti di arrocco: 0=bianco_lato_re, 1=bianco_lato_regina, 2=nero_lato_re, 3=nero_lato_regina
    en_passant: [u64; 8],         // colonne en passant
    colore: u64,                  // XOR per cambio colore
}
impl ZobristTable {
    fn nuova() -> Self {
        let mut rng = rand::thread_rng();
        
        let mut pezzi = [[[0; 64]; 6]; 2];
        for colore in 0..2 {
            for pezzo in 0..6 {
                for casella in 0..64 {
                    pezzi[colore][pezzo][casella] = rng.gen();
                }
            }
        }
        
        let mut arrocco = [0; 4];
        for i in 0..4 {
            arrocco[i] = rng.gen();
        }
        
        let mut en_passant = [0; 8];
        for i in 0..8 {
            en_passant[i] = rng.gen();
        }
        
        ZobristTable {
            pezzi,
            colore: rng.gen(),
            arrocco,
            en_passant,
        }
    }
}

lazy_static! {
    pub static ref ZOBRIST: ZobristTable = ZobristTable::nuova();
}

// ===== ATTACCHI TABLES =====

// Tabelle di attacco precalcolate per cavalli e re
fn precalcola_attacchi_cavallo() -> [Bitboard; 64] {
    let mut attacchi = [0; 64];
    for casella in 0..64 {
        let (file, rank) = (casella % 8, casella / 8);
        let offsets = [(-2, -1), (-2, 1), (-1, -2), (-1, 2),
                      (1, -2), (1, 2), (2, -1), (2, 1)];
        
        for (df, dr) in offsets.iter() {
            let new_file = file as i32 + df;
            let new_rank = rank as i32 + dr;
            
            if new_file >= 0 && new_file < 8 && new_rank >= 0 && new_rank < 8 {
                attacchi[casella] |= 1u64 << (new_rank * 8 + new_file);
            }
        }
    }
    attacchi
}

fn precalcola_attacchi_re() -> [Bitboard; 64] {
    let mut attacchi = [0; 64];
    for casella in 0..64 {
        let (file, rank) = (casella % 8, casella / 8);
        
        for df in -1..=1 {
            for dr in -1..=1 {
                if df == 0 && dr == 0 {
                    continue;
                }
                
                let new_file = file as i32 + df;
                let new_rank = rank as i32 + dr;
                
                if new_file >= 0 && new_file < 8 && new_rank >= 0 && new_rank < 8 {
                    attacchi[casella] |= 1u64 << (new_rank * 8 + new_file);
                }
            }
        }
    }
    attacchi
}

lazy_static! {
    pub static ref ATTACCHI_CAVALLO: [Bitboard; 64] = precalcola_attacchi_cavallo();
    pub static ref ATTACCHI_RE: [Bitboard; 64] = precalcola_attacchi_re();
}

// ===== MASCHERE PER FILE E RANK =====

const FILE_A: Bitboard = 0x0101010101010101;
const FILE_H: Bitboard = 0x8080808080808080;
const RANK_1: Bitboard = 0x00000000000000FF;
const RANK_8: Bitboard = 0xFF00000000000000;

// ===== SCACCHIERA STRUCT =====

#[derive(Clone)]
pub struct Scacchiera {
    pezzi: [[Option<(Pezzo, Colore)>; 8]; 8],
    bitboards: [[Bitboard; 6]; 2],
    occupazione_totale: Bitboard,
    occupazione_colore: [Bitboard; 2],
    turno: Colore,
    diritti_arrocco: DirittiArrocco,
    en_passant: Option<Casella>,
    hash: u64,
    halfmove_clock: u32,
    fullmove_number: u32,
    storico_mosse: Vec<Mossa>,
    storico_hash: Vec<u64>,
}

impl Scacchiera {
    pub fn nuova() -> Self {
        let mut scacchiera = Scacchiera {
            pezzi: [[None; 8]; 8],
            bitboards: [[0; 6]; 2],
            occupazione_totale: 0,
            occupazione_colore: [0, 0],
            turno: Colore::Bianco,
            diritti_arrocco: DirittiArrocco::tutti(),
            en_passant: None,
            hash: 0,
            halfmove_clock: 0,
            fullmove_number: 1,
            storico_mosse: Vec::new(),
            storico_hash: Vec::new(),
        };
        
        // Posiziona i pezzi
        scacchiera.metti_pezzo(Casella::nuova(0, 0).unwrap(), Pezzo::Torre, Colore::Bianco);
        scacchiera.metti_pezzo(Casella::nuova(1, 0).unwrap(), Pezzo::Cavallo, Colore::Bianco);
        scacchiera.metti_pezzo(Casella::nuova(2, 0).unwrap(), Pezzo::Alfiere, Colore::Bianco);
        scacchiera.metti_pezzo(Casella::nuova(3, 0).unwrap(), Pezzo::Regina, Colore::Bianco);
        scacchiera.metti_pezzo(Casella::nuova(4, 0).unwrap(), Pezzo::Re, Colore::Bianco);
        scacchiera.metti_pezzo(Casella::nuova(5, 0).unwrap(), Pezzo::Alfiere, Colore::Bianco);
        scacchiera.metti_pezzo(Casella::nuova(6, 0).unwrap(), Pezzo::Cavallo, Colore::Bianco);
        scacchiera.metti_pezzo(Casella::nuova(7, 0).unwrap(), Pezzo::Torre, Colore::Bianco);
        
        for i in 0..8 {
            scacchiera.metti_pezzo(Casella::nuova(i, 1).unwrap(), Pezzo::Pedone, Colore::Bianco);
        }
        
        scacchiera.metti_pezzo(Casella::nuova(0, 7).unwrap(), Pezzo::Torre, Colore::Nero);
        scacchiera.metti_pezzo(Casella::nuova(1, 7).unwrap(), Pezzo::Cavallo, Colore::Nero);
        scacchiera.metti_pezzo(Casella::nuova(2, 7).unwrap(), Pezzo::Alfiere, Colore::Nero);
        scacchiera.metti_pezzo(Casella::nuova(3, 7).unwrap(), Pezzo::Regina, Colore::Nero);
        scacchiera.metti_pezzo(Casella::nuova(4, 7).unwrap(), Pezzo::Re, Colore::Nero);
        scacchiera.metti_pezzo(Casella::nuova(5, 7).unwrap(), Pezzo::Alfiere, Colore::Nero);
        scacchiera.metti_pezzo(Casella::nuova(6, 7).unwrap(), Pezzo::Cavallo, Colore::Nero);
        scacchiera.metti_pezzo(Casella::nuova(7, 7).unwrap(), Pezzo::Torre, Colore::Nero);
        
        for i in 0..8 {
            scacchiera.metti_pezzo(Casella::nuova(i, 6).unwrap(), Pezzo::Pedone, Colore::Nero);
        }
        
        scacchiera.calcola_occupazione();
        scacchiera.hash = scacchiera.calcola_hash();
        
        scacchiera
    }
    
    fn metti_pezzo(&mut self, casella: Casella, pezzo: Pezzo, colore: Colore) {
        if !casella.is_valid() {
            return;
        }
        
        let idx = casella.indice();
        self.pezzi[casella.rank as usize][casella.file as usize] = Some((pezzo, colore));
        let bit = 1u64 << idx;
        self.bitboards[colore.indice()][pezzo.indice()] |= bit;
    }
    
    fn calcola_occupazione(&mut self) {
        self.occupazione_colore[0] = 0;
        self.occupazione_colore[1] = 0;
        
        for colore in 0..2 {
            for pezzo in 0..6 {
                self.occupazione_colore[colore] |= self.bitboards[colore][pezzo];
            }
        }
        
        self.occupazione_totale = self.occupazione_colore[0] | self.occupazione_colore[1];
    }
    
    pub fn da_fen(fen: &str) -> Result<Self, String> {
        let parts: Vec<&str> = fen.split_whitespace().collect();
        
        if parts.len() < 4 {
            return Err("FEN troppo corto".to_string());
        }
        
        let posizione = parts[0];
        let colore = parts[1];
        let arrocco = parts[2];
        let en_passant = parts[3];
        
        let mut scacchiera = Scacchiera {
            pezzi: [[None; 8]; 8],
            bitboards: [[0; 6]; 2],
            occupazione_totale: 0,
            occupazione_colore: [0, 0],
            turno: Colore::Bianco,
            diritti_arrocco: DirittiArrocco::nessuno(),
            en_passant: None,
            hash: 0,
            halfmove_clock: 0,
            fullmove_number: 1,
            storico_mosse: Vec::new(),
            storico_hash: Vec::new(),
        };
        
        if parts.len() >= 5 {
            scacchiera.halfmove_clock = parts[4].parse().unwrap_or(0);
        }
        
        if parts.len() >= 6 {
            scacchiera.fullmove_number = parts[5].parse().unwrap_or(1);
        }
        
        let righe: Vec<&str> = posizione.split('/').collect();
        if righe.len() != 8 {
            return Err("Posizione FEN deve avere 8 righe".to_string());
        }
        
        for (rank_idx, riga) in righe.iter().enumerate() {
            let rank = (7 - rank_idx) as u8;
            let mut file = 0;
            
            for c in riga.chars() {
                if file >= 8 {
                    return Err("Troppi pezzi/campi in una riga".to_string());
                }
                
                if c.is_ascii_digit() {
                    file += c.to_digit(10).unwrap() as u8;
                } else if let Some((pezzo, colore_pezzo)) = Pezzo::from_char(c) {
                    scacchiera.metti_pezzo(Casella::nuova(file, rank).unwrap(), pezzo, colore_pezzo);
                    file += 1;
                } else {
                    return Err(format!("Carattere FEN non valido: {}", c));
                }
            }
        }
        
        scacchiera.turno = match colore {
            "w" => Colore::Bianco,
            "b" => Colore::Nero,
            _ => return Err("Colore attivo non valido".to_string()),
        };
        
        if arrocco != "-" {
            for c in arrocco.chars() {
                match c {
                    'K' => scacchiera.diritti_arrocco.bianco_lato_re = true,
                    'Q' => scacchiera.diritti_arrocco.bianco_lato_regina = true,
                    'k' => scacchiera.diritti_arrocco.nero_lato_re = true,
                    'q' => scacchiera.diritti_arrocco.nero_lato_regina = true,
                    _ => {}
                }
            }
        }
        
        scacchiera.en_passant = if en_passant == "-" {
            None
        } else {
            Casella::da_string(en_passant)
        };
        
        scacchiera.calcola_occupazione();
        scacchiera.hash = scacchiera.calcola_hash();
        
        Ok(scacchiera)
    }
    
    pub fn a_fen(&self) -> String {
        let mut fen = String::new();
        
        for rank in (0..8).rev() {
            let mut empty_count = 0;
            
            for file in 0..8 {
                if let Some((pezzo, colore)) = self.pezzi[rank][file] {
                    if empty_count > 0 {
                        fen.push_str(&empty_count.to_string());
                        empty_count = 0;
                    }
                    fen.push(pezzo.simbolo(colore));
                } else {
                    empty_count += 1;
                }
            }
            
            if empty_count > 0 {
                fen.push_str(&empty_count.to_string());
            }
            
            if rank > 0 {
                fen.push('/');
            }
        }
        
        fen.push(' ');
        fen.push(match self.turno {
            Colore::Bianco => 'w',
            Colore::Nero => 'b',
        });
        
        fen.push(' ');
        let mut diritti_str = String::new();
        
        if self.diritti_arrocco.bianco_lato_re { diritti_str.push('K'); }
        if self.diritti_arrocco.bianco_lato_regina { diritti_str.push('Q'); }
        if self.diritti_arrocco.nero_lato_re { diritti_str.push('k'); }
        if self.diritti_arrocco.nero_lato_regina { diritti_str.push('q'); }
        
        if diritti_str.is_empty() {
            fen.push('-');
        } else {
            fen.push_str(&diritti_str);
        }
        
        fen.push(' ');
        if let Some(casella) = self.en_passant {
            fen.push_str(&casella.to_string());
        } else {
            fen.push('-');
        }
        
        fen.push_str(&format!(" {} {}", self.halfmove_clock, self.fullmove_number));
        
        fen
    }
    
    // ===== HASH ZOBRIST =====
    
    fn aggiorna_hash_pezzo(&mut self, casella: Casella, pezzo: Pezzo, colore: Colore) {
        let idx = casella.indice();
        let colore_idx = colore.indice();
        let pezzo_idx = pezzo.indice();
        
        self.hash ^= ZOBRIST.pezzi[colore_idx][pezzo_idx][idx];
    }
    
    fn aggiorna_hash_colore(&mut self) {
        if self.turno == Colore::Nero {
            self.hash ^= ZOBRIST.colore;
        }
    }
    
    fn aggiorna_hash_diritti(&mut self) {
        if self.diritti_arrocco.bianco_lato_re {
            self.hash ^= ZOBRIST.arrocco[0];
        }
        if self.diritti_arrocco.bianco_lato_regina {
            self.hash ^= ZOBRIST.arrocco[1];
        }
        if self.diritti_arrocco.nero_lato_re {
            self.hash ^= ZOBRIST.arrocco[2];
        }
        if self.diritti_arrocco.nero_lato_regina {
            self.hash ^= ZOBRIST.arrocco[3];
        }
    }
    
    fn aggiorna_hash_en_passant(&mut self) {
        if let Some(casella) = self.en_passant {
            let file = casella.file() as usize;
            if file < 8 {
                self.hash ^= ZOBRIST.en_passant[file];
            }
        }
    }
    
    fn calcola_hash(&self) -> u64 {
        let mut hash = 0;
        
        // Pezzi
        for rank in 0..8 {
            for file in 0..8 {
                if let Some((pezzo, colore)) = self.pezzi[rank][file] {
                    let idx = rank * 8 + file;
                    hash ^= ZOBRIST.pezzi[colore.indice()][pezzo.indice()][idx];
                }
            }
        }
        
        // Colore
        if self.turno == Colore::Nero {
            hash ^= ZOBRIST.colore;
        }
        
        // Diritti arrocco
        if self.diritti_arrocco.bianco_lato_re {
            hash ^= ZOBRIST.arrocco[0];
        }
        if self.diritti_arrocco.bianco_lato_regina {
            hash ^= ZOBRIST.arrocco[1];
        }
        if self.diritti_arrocco.nero_lato_re {
            hash ^= ZOBRIST.arrocco[2];
        }
        if self.diritti_arrocco.nero_lato_regina {
            hash ^= ZOBRIST.arrocco[3];
        }
        
        // En passant
        if let Some(casella) = self.en_passant {
            let file = casella.file() as usize;
            if file < 8 {
                hash ^= ZOBRIST.en_passant[file];
            }
        }
        
        hash
    }
    
    // ===== ESECUZIONE MOSSE =====
    
    pub fn esegui_mossa(&mut self, mossa: &Mossa) -> bool {
        // Salva stato per annullamento
        self.storico_hash.push(self.hash);
        self.storico_mosse.push(*mossa);
        
        // Gestione en passant
        let old_en_passant = self.en_passant;
        self.en_passant = None;
        
        // Aggiorna hash per vecchio en passant
        if let Some(casella) = old_en_passant {
            let file = casella.file() as usize;
            if file < 8 {
                self.hash ^= ZOBRIST.en_passant[file];
            }
        }
        
        let da = mossa.da();
        let a = mossa.a();
        let flag = mossa.flag();
        
        // Ottieni il pezzo che si sta muovendo
        let (pezzo_mosso, colore_mosso) = match self.pezzo_su_casella(da) {
            Some(p) => p,
            None => return false,
        };
        
        // Gestione arrocco
        if flag == MoveFlag::CastleKingSide || flag == MoveFlag::CastleQueenSide {
            // Sposta il re
            self.imposta_pezzo(da, None);
            self.imposta_pezzo(a, Some((pezzo_mosso, colore_mosso)));
            
            // Sposta la torre
            if flag == MoveFlag::CastleKingSide { // Arrocco corto
                self.imposta_pezzo(Casella::nuova(7, da.rank()).unwrap(), None);
                self.imposta_pezzo(Casella::nuova(5, da.rank()).unwrap(), Some((Pezzo::Torre, colore_mosso)));
            } else { // Arrocco lungo
                self.imposta_pezzo(Casella::nuova(0, da.rank()).unwrap(), None);
                self.imposta_pezzo(Casella::nuova(3, da.rank()).unwrap(), Some((Pezzo::Torre, colore_mosso)));
            }
        } else {
            // Movimento normale
            let pezzo_catturato = if flag == MoveFlag::EnPassant {
                // Cattura en passant
                let pedone_catturato_rank = if colore_mosso == Colore::Bianco { a.rank() - 1 } else { a.rank() + 1 };
                let casella_cattura = Casella::nuova(a.file(), pedone_catturato_rank).unwrap();
                self.imposta_pezzo(casella_cattura, None)
            } else if flag.is_capture() {
                // Cattura normale
                self.imposta_pezzo(a, None)
            } else {
                None
            };
            
            // Muovi il pezzo
            self.imposta_pezzo(da, None);
            
            // Gestione promozione
            if let Some(promozione) = mossa.promozione() {
                self.imposta_pezzo(a, Some((promozione, colore_mosso)));
            } else {
                self.imposta_pezzo(a, Some((pezzo_mosso, colore_mosso)));
            }
            
            // Aggiorna contatori
            if pezzo_mosso == Pezzo::Pedone || pezzo_catturato.is_some() {
                self.halfmove_clock = 0;
            } else {
                self.halfmove_clock += 1;
            }
            
            // Imposta en passant per pedoni che avanzano di due case
            if pezzo_mosso == Pezzo::Pedone && (da.rank() as i32 - a.rank() as i32).abs() == 2 {
                let rank_en_passant = (da.rank() + a.rank()) / 2;
                self.en_passant = Casella::nuova(da.file(), rank_en_passant);
                
                // Aggiorna hash per nuovo en passant
                let file = da.file() as usize;
                if file < 8 {
                    self.hash ^= ZOBRIST.en_passant[file];
                }
            }
        }
        
        // Aggiorna diritti arrocco
        let old_diritti = self.diritti_arrocco;
        
        // Se si muove il re, perde tutti i diritti di arrocco
        if pezzo_mosso == Pezzo::Re {
            match colore_mosso {
                Colore::Bianco => {
                    self.diritti_arrocco.bianco_lato_re = false;
                    self.diritti_arrocco.bianco_lato_regina = false;
                }
                Colore::Nero => {
                    self.diritti_arrocco.nero_lato_re = false;
                    self.diritti_arrocco.nero_lato_regina = false;
                }
            }
        }
        
        // Se si muove una torre dalla sua posizione iniziale, perde il diritto di arrocco da quel lato
        if pezzo_mosso == Pezzo::Torre {
            match (colore_mosso, da.file(), da.rank()) {
                (Colore::Bianco, 0, 0) => self.diritti_arrocco.bianco_lato_regina = false,
                (Colore::Bianco, 7, 0) => self.diritti_arrocco.bianco_lato_re = false,
                (Colore::Nero, 0, 7) => self.diritti_arrocco.nero_lato_regina = false,
                (Colore::Nero, 7, 7) => self.diritti_arrocco.nero_lato_re = false,
                _ => {}
            }
        }
        
        // Se viene catturata una torre, l'avversario perde i diritti di arrocco da quel lato
        if flag.is_capture() && flag != MoveFlag::EnPassant {
            if let Some((pezzo_catturato, colore_catturato)) = self.pezzo_su_casella(a) {
                if pezzo_catturato == Pezzo::Torre {
                    match (colore_catturato, a.file(), a.rank()) {
                        (Colore::Bianco, 0, 0) => self.diritti_arrocco.bianco_lato_regina = false,
                        (Colore::Bianco, 7, 0) => self.diritti_arrocco.bianco_lato_re = false,
                        (Colore::Nero, 0, 7) => self.diritti_arrocco.nero_lato_regina = false,
                        (Colore::Nero, 7, 7) => self.diritti_arrocco.nero_lato_re = false,
                        _ => {}
                    }
                }
            }
        }
        
        // Aggiorna hash per diritti arrocco cambiati
        if self.diritti_arrocco != old_diritti {
            self.aggiorna_hash_diritti();
        }
        
        // Cambia turno
        self.turno = self.turno.opposto();
        self.aggiorna_hash_colore();
        
        // Aggiorna fullmove number
        if self.turno == Colore::Bianco {
            self.fullmove_number += 1;
        }
        
        true
    }
    
    pub fn esegui_mossa_simulata(&mut self, mossa: Mossa) -> bool {
        // Versione che accetta Mossa invece di &Mossa per compatibilità
        self.esegui_mossa(&mossa)
    }
    
    pub fn annulla_mossa(&mut self) -> Option<Mossa> {
        if let Some(mossa) = self.storico_mosse.pop() {
            if let Some(old_hash) = self.storico_hash.pop() {
                self.hash = old_hash;
            }
            
            // Ripristina il turno
            self.turno = self.turno.opposto();
            
            // Aggiorna fullmove number
            if self.turno == Colore::Nero {
                self.fullmove_number = self.fullmove_number.saturating_sub(1);
            }
            
            Some(mossa)
        } else {
            None
        }
    }
    
    // ===== METODI DI ACCESSO =====
    
    pub fn turno(&self) -> Colore {
        self.turno
    }
    
    // Alias per compatibilità
    pub fn colore_attivo(&self) -> Colore {
        self.turno
    }
    
    pub fn diritti_arrocco(&self) -> u8 {
        self.diritti_arrocco.to_u8()
    }
    
    pub fn diritti_arrocco_struct(&self) -> DirittiArrocco {
        self.diritti_arrocco
    }
    
    pub fn en_passant(&self) -> Option<usize> {
        self.en_passant.map(|c| c.indice())
    }
    
    pub fn pezzo_su_casella(&self, casella: Casella) -> Option<(Pezzo, Colore)> {
        if casella.is_valid() {
            self.pezzi[casella.rank() as usize][casella.file() as usize]
        } else {
            None
        }
    }
    
    pub fn pezzo_su_casella_indice(&self, idx: usize) -> Option<(Pezzo, Colore)> {
        if let Some(casella) = Casella::da_indice(idx) {
            self.pezzo_su_casella(casella)
        } else {
            None
        }
    }
    
    // Alias per compatibilità
    pub fn ottieni_pezzo(&self, casella: Casella) -> Option<(Pezzo, Colore)> {
        self.pezzo_su_casella(casella)
    }
    
    pub fn pezzo_a(&self, casella: Casella) -> Option<(Pezzo, Colore)> {
        self.pezzo_su_casella(casella)
    }
    
    fn imposta_pezzo(&mut self, casella: Casella, pezzo: Option<(Pezzo, Colore)>) -> Option<(Pezzo, Colore)> {
        if !casella.is_valid() {
            return None;
        }
        
        let rank = casella.rank() as usize;
        let file = casella.file() as usize;
        let old = self.pezzi[rank][file];
        
        // Rimuovi vecchio pezzo
        if let Some((old_pezzo, old_colore)) = old {
            self.aggiorna_hash_pezzo(casella, old_pezzo, old_colore);
            
            let bit = casella.to_bitboard();
            self.bitboards[old_colore.indice()][old_pezzo.indice()] &= !bit;
        }
        
        // Imposta nuovo pezzo
        self.pezzi[rank][file] = pezzo;
        if let Some((pezzo, colore)) = pezzo {
            self.aggiorna_hash_pezzo(casella, pezzo, colore);
            
            let bit = casella.to_bitboard();
            self.bitboards[colore.indice()][pezzo.indice()] |= bit;
        }
        
        self.calcola_occupazione();
        old
    }
    
    // ===== METODI BITBOARD =====
    
    pub fn occupazione(&self) -> Bitboard {
        self.occupazione_totale
    }
    
    // Alias per compatibilità
    pub fn occupazione_totale(&self) -> Bitboard {
        self.occupazione_totale
    }
    
    pub fn bitboard_colore(&self, colore: Colore) -> Bitboard {
        self.occupazione_colore[colore.indice()]
    }
    
    pub fn bitboard_pezzo(&self, pezzo: Pezzo) -> Bitboard {
        let mut bb = 0;
        for colore in 0..2 {
            bb |= self.bitboards[colore][pezzo.indice()];
        }
        bb
    }
    
    pub fn bitboard_pezzo_colore(&self, pezzo: Pezzo, colore: Colore) -> Bitboard {
        self.bitboards[colore.indice()][pezzo.indice()]
    }
    
    // ===== CONTATORI =====
    
    pub fn halfmove_clock(&self) -> u32 {
        self.halfmove_clock
    }
    
    pub fn fullmove_number(&self) -> u32 {
        self.fullmove_number
    }
    
    // ===== METODI PER HASH =====
    
    pub fn hash(&self) -> u64 {
        self.hash
    }
    
    // ===== METODI DI UTILITÀ =====
    
    pub fn trova_re(&self, colore: Colore) -> Option<Casella> {
        let re_bb = self.bitboard_pezzo_colore(Pezzo::Re, colore);
        if re_bb == 0 {
            return None;
        }
        
        let indice = re_bb.trailing_zeros() as usize;
        Casella::da_indice(indice)
    }
    
    pub fn sotto_scacco(&self, colore: Colore) -> bool {
        if let Some(pos_re) = self.trova_re(colore) {
            self.casella_attaccata(pos_re, colore.opposto())
        } else {
            false
        }
    }
    
    // Alias per compatibilità
    pub fn re_in_scacco(&self, colore: Colore) -> bool {
        self.sotto_scacco(colore)
    }
    
    // ===== FUNZIONI DI ATTACCO OTTIMIZZATE =====
    
    pub fn attacchi_pedone(&self, casella: Casella, colore: Colore) -> Bitboard {
        let idx = casella.indice();
        let pedoni = self.bitboard_pezzo_colore(Pezzo::Pedone, colore);
        
        match colore {
            Colore::Bianco => {
                let catture_est = (pedoni << 7) & !FILE_H;
                let catture_ovest = (pedoni << 9) & !FILE_A;
                
                if (catture_est & (1u64 << idx)) != 0 || (catture_ovest & (1u64 << idx)) != 0 {
                    return 1u64 << idx;
                }
            }
            Colore::Nero => {
                let catture_est = (pedoni >> 9) & !FILE_H;
                let catture_ovest = (pedoni >> 7) & !FILE_A;
                
                if (catture_est & (1u64 << idx)) != 0 || (catture_ovest & (1u64 << idx)) != 0 {
                    return 1u64 << idx;
                }
        }
    }
    
        0
    }
    
    pub fn casella_attaccata(&self, casella: Casella, da_colore: Colore) -> bool {
        let idx = casella.indice();
        
        // Controlla pedoni
        if self.attacchi_pedone(casella, da_colore) != 0 {
            return true;
        }
        
        // Controlla cavalli (precalcolato)
        let cavalli = self.bitboard_pezzo_colore(Pezzo::Cavallo, da_colore);
        if (cavalli & ATTACCHI_CAVALLO[idx]) != 0 {
            return true;
        }
        
        // Controlla re (precalcolato)
        let re = self.bitboard_pezzo_colore(Pezzo::Re, da_colore);
        if (re & ATTACCHI_RE[idx]) != 0 {
            return true;
        }
        
        // Controlla attacchi di alfiere/regina (diagonali)
        let alfieri = self.bitboard_pezzo_colore(Pezzo::Alfiere, da_colore);
        let regine = self.bitboard_pezzo_colore(Pezzo::Regina, da_colore);
        let pezzi_diagonali = alfieri | regine;
        
        if pezzi_diagonali != 0 {
            let attacchi_diagonali = self.attacchi_alfiere(casella, self.occupazione_totale);
            if (pezzi_diagonali & attacchi_diagonali) != 0 {
                return true;
            }
        }
        
        // Controlla attacchi di torre/regina (lineari)
        let torri = self.bitboard_pezzo_colore(Pezzo::Torre, da_colore);
        let pezzi_lineari = torri | regine;
        
        if pezzi_lineari != 0 {
            let attacchi_lineari = self.attacchi_torre(casella, self.occupazione_totale);
            if (pezzi_lineari & attacchi_lineari) != 0 {
                return true;
            }
        }
        
        false
    }
    
    // Funzioni di attacco con lookup tables per pezzi a slittamento
    pub fn attacchi_alfiere(&self, casella: Casella, occupazione: Bitboard) -> Bitboard {
        let idx = casella.indice();
        
        // Implementazione con bitboard magics (semplificata per ora)
        let mut attacks = 0u64;
        
        // Nord-est
        let mut target = 1u64 << idx;
        for _ in 0..7 {
            target = (target << 9) & !FILE_A;
            if target == 0 { break; }
            attacks |= target;
            if (occupazione & target) != 0 { break; }
        }
        
        // Nord-ovest
        let mut target = 1u64 << idx;
        for _ in 0..7 {
            target = (target << 7) & !FILE_H;
            if target == 0 { break; }
            attacks |= target;
            if (occupazione & target) != 0 { break; }
        }
        
        // Sud-est
        let mut target = 1u64 << idx;
        for _ in 0..7 {
            target = (target >> 7) & !FILE_A;
            if target == 0 { break; }
            attacks |= target;
            if (occupazione & target) != 0 { break; }
        }
        
        // Sud-ovest
        let mut target = 1u64 << idx;
        for _ in 0..7 {
            target = (target >> 9) & !FILE_H;
            if target == 0 { break; }
            attacks |= target;
            if (occupazione & target) != 0 { break; }
        }
        
        attacks
    }
    
    pub fn attacchi_torre(&self, casella: Casella, occupazione: Bitboard) -> Bitboard {
        let idx = casella.indice();
        let mut attacks = 0u64;
        
        // Nord
        let mut target = 1u64 << idx;
        for _ in 0..7 {
            target <<= 8;
            if target == 0 { break; }
            attacks |= target;
            if (occupazione & target) != 0 { break; }
        }
        
        // Sud
        let mut target = 1u64 << idx;
        for _ in 0..7 {
            target >>= 8;
            if target == 0 { break; }
            attacks |= target;
            if (occupazione & target) != 0 { break; }
        }
        
        // Est
        let mut target = 1u64 << idx;
        for _ in 0..7 {
            target = (target << 1) & !FILE_A;
            if target == 0 { break; }
            attacks |= target;
            if (occupazione & target) != 0 { break; }
        }
        
        // Ovest
        let mut target = 1u64 << idx;
        for _ in 0..7 {
            target = (target >> 1) & !FILE_H;
            if target == 0 { break; }
            attacks |= target;
            if (occupazione & target) != 0 { break; }
        }
        
        attacks
    }
    
    pub fn e_scacco_matto(&self, colore: Colore) -> bool {
        // Un re in scacco senza mosse legali è scacco matto
        if !self.sotto_scacco(colore) {
            return false;
        }
        
        // Genera tutte le mosse possibili e controlla se almeno una è legale
        // Implementazione semplificata
        // In una implementazione completa, usare movegen::genera_mosse_legali()
        false
    }
    
    pub fn e_stallo(&self, colore: Colore) -> bool {
        // Un re non in scacco senza mosse legali è stallo
        if self.sotto_scacco(colore) {
            return false;
        }
        
        // Controlla se ci sono mosse legali
        // In una implementazione completa, usare movegen::genera_mosse_legali()
        false
    }
    
    pub fn stampa(&self) {
        println!("  a b c d e f g h");
        println!("  ----------------");
        
        for rank in (0..8).rev() {
            print!("{}|", rank + 1);
            for file in 0..8 {
                let simbolo = match self.pezzi[rank][file] {
                    Some((pezzo, colore)) => pezzo.simbolo(colore),
                    None => '.',
                };
                print!("{} ", simbolo);
            }
            println!("|{}", rank + 1);
        }
        
        println!("  ----------------");
        println!("  a b c d e f g h");
        
        println!("Colore attivo: {:?}", self.turno);
        println!("En passant: {:?}", self.en_passant);
        println!("Hash: 0x{:016x}", self.hash);
        println!("Halfmove: {}, Fullmove: {}", self.halfmove_clock, self.fullmove_number);
    }
    
    pub fn clone(&self) -> Self {
        Scacchiera {
            pezzi: self.pezzi,
            bitboards: self.bitboards,
            occupazione_totale: self.occupazione_totale,
            occupazione_colore: self.occupazione_colore,
            turno: self.turno,
            diritti_arrocco: self.diritti_arrocco,
            en_passant: self.en_passant,
            hash: self.hash,
            halfmove_clock: self.halfmove_clock,
            fullmove_number: self.fullmove_number,
            storico_mosse: self.storico_mosse.clone(),
            storico_hash: self.storico_hash.clone(),
        }
    }
    
    pub fn valore_materiale(&self, colore: Colore) -> i32 {
        let mut valore = 0;
        let colore_idx = colore.indice();
        
        for pezzo_idx in 0..6 {
            let pezzo = match pezzo_idx {
                0 => Pezzo::Pedone,
                1 => Pezzo::Cavallo,
                2 => Pezzo::Alfiere,
                3 => Pezzo::Torre,
                4 => Pezzo::Regina,
                5 => Pezzo::Re,
                _ => continue,
            };
            
            let bitboard = self.bitboards[colore_idx][pezzo_idx];
            let count = bitboard.count_ones() as i32;
            valore += count * pezzo.valore();
        }
        
        valore
    }
    
    pub fn valore_materiale_totale(&self) -> i32 {
        self.valore_materiale(Colore::Bianco) - self.valore_materiale(Colore::Nero)
    }
}

// ===== FUNZIONI DI UTILITÀ =====

// Funzione di utilità per iterare su bitboard
fn bitboard_to_indices(bb: Bitboard) -> Vec<usize> {
    let mut indices = Vec::new();
    let mut bitboard = bb;
    
    while bitboard != 0 {
        let idx = bitboard.trailing_zeros() as usize;
        indices.push(idx);
        bitboard &= bitboard - 1; // Rimuovi il bit meno significativo
    }
    
    indices
}

impl fmt::Display for Scacchiera {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "  a b c d e f g h")?;
        writeln!(f, "  ----------------")?;
        
        for rank in (0..8).rev() {
            write!(f, "{}|", rank + 1)?;
            for file in 0..8 {
                let simbolo = match self.pezzi[rank][file] {
                    Some((pezzo, colore)) => pezzo.simbolo(colore),
                    None => '.',
                };
                write!(f, "{} ", simbolo)?;
            }
            writeln!(f, "|{}", rank + 1)?;
        }
        
        writeln!(f, "  ----------------")?;
        writeln!(f, "  a b c d e f g h")?;
        
        writeln!(f, "Turno: {:?}", self.turno)?;
        writeln!(f, "Valore materiale: {}", self.valore_materiale_totale())?;
        
        Ok(())
    }
}

impl Hash for Scacchiera {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hash.hash(state);
    }
}

impl Default for Scacchiera {
    fn default() -> Self {
        Self::nuova()
    }
}

// ===== TEST =====

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_scacchiera_iniziale() {
        let scacchiera = Scacchiera::nuova();
        
        // Controlla che ci siano 32 pezzi
        let mut count = 0;
        for rank in 0..8 {
            for file in 0..8 {
                if scacchiera.pezzi[rank][file].is_some() {
                    count += 1;
                }
            }
        }
        assert_eq!(count, 32);
        
        // Controlla posizione re bianco
        assert_eq!(scacchiera.pezzo_a(Casella::nuova(4, 0).unwrap()), 
                   Some((Pezzo::Re, Colore::Bianco)));
        
        // Controlla posizione re nero
        assert_eq!(scacchiera.pezzo_a(Casella::nuova(4, 7).unwrap()), 
                   Some((Pezzo::Re, Colore::Nero)));
    }
    
    #[test]
    fn test_fen_parsing() {
        let fen_iniziale = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
        let scacchiera = Scacchiera::da_fen(fen_iniziale).unwrap();
        
        assert_eq!(scacchiera.turno(), Colore::Bianco);
        assert_eq!(scacchiera.diritti_arrocco_struct().bianco_lato_re, true);
        assert_eq!(scacchiera.en_passant(), None);
    }
    
    #[test]
    fn test_mossa_semplice() {
        let mut scacchiera = Scacchiera::nuova();
        
        // Prova mossa pedone bianco
        let mossa = Mossa::new(
            Casella::nuova(4, 1).unwrap(),
            Casella::nuova(4, 3).unwrap(),
            None
        );
        
        let result = scacchiera.esegui_mossa(&mossa);
        assert_eq!(result, true);
        assert_eq!(scacchiera.turno(), Colore::Nero);
        assert_eq!(scacchiera.pezzo_a(Casella::nuova(4, 3).unwrap()),
                   Some((Pezzo::Pedone, Colore::Bianco)));
    }
    
    #[test]
    fn test_hash_consistente() {
        let scacchiera1 = Scacchiera::nuova();
        let scacchiera2 = Scacchiera::nuova();
        
        // Due scacchiere identiche dovrebbero avere lo stesso hash
        assert_eq!(scacchiera1.hash(), scacchiera2.hash());
        
        let mut scacchiera3 = scacchiera1.clone();
        let mossa = Mossa::new(
            Casella::nuova(4, 1).unwrap(),
            Casella::nuova(4, 3).unwrap(),
            None
        );
        scacchiera3.esegui_mossa(&mossa);
        
        // Dopo una mossa, l'hash dovrebbe essere diverso
        assert_ne!(scacchiera1.hash(), scacchiera3.hash());
    }
    
    #[test]
    fn test_mossa_encoding() {
        let mossa = Mossa::new(
            Casella::nuova(4, 1).unwrap(),
            Casella::nuova(4, 3).unwrap(),
            None
        );
        
        let encoded = mossa.to_u32();
        let decoded = Mossa::from_u32(encoded).unwrap();
        
        assert_eq!(mossa.da(), decoded.da());
        assert_eq!(mossa.a(), decoded.a());
    }
    
    #[test]
    fn test_imposta_pezzo() {
        let mut scacchiera = Scacchiera::nuova();
        let casella = Casella::nuova(3, 3).unwrap();
        
        // Rimuovi pezzo esistente
        let old = scacchiera.imposta_pezzo(casella, None);
        assert_eq!(scacchiera.pezzo_a(casella), None);
        
        // Metti nuovo pezzo
        scacchiera.imposta_pezzo(casella, Some((Pezzo::Regina, Colore::Bianco)));
        assert_eq!(scacchiera.pezzo_a(casella), Some((Pezzo::Regina, Colore::Bianco)));
    }
}