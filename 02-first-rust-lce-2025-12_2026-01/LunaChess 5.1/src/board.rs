use std::fmt;

#[derive(Clone, Copy, PartialEq, Debug)]
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

// Trait per iterare su bitboard
pub trait BitboardIter {
    fn iter_caselle(&self) -> BitboardIterator;
}

impl BitboardIter for u64 {
    fn iter_caselle(&self) -> BitboardIterator {
        BitboardIterator { bitboard: *self }
    }
}

pub struct BitboardIterator {
    bitboard: u64,
}

impl Iterator for BitboardIterator {
    type Item = Casella;
    
    fn next(&mut self) -> Option<Self::Item> {
        if self.bitboard == 0 {
            None
        } else {
            let lsb = self.bitboard.trailing_zeros() as u8;
            self.bitboard &= self.bitboard - 1; // Rimuovi il bit più basso
            Casella::da_indice(lsb as usize)
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
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
    
    pub fn all() -> [Pezzo; 6] {
        [Pezzo::Pedone, Pezzo::Cavallo, Pezzo::Alfiere, 
         Pezzo::Torre, Pezzo::Regina, Pezzo::Re]
    }
}

pub type Bitboard = u64;

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Casella {
    pub file: u8, // 0-7 (a-h)
    pub rank: u8, // 0-7 (1-8)
}

impl Casella {
    pub fn nuova(file: u8, rank: u8) -> Option<Self> {
        if file < 8 && rank < 8 {
            Some(Casella { file, rank })
        } else {
            None
        }
    }
    
    pub fn da_coordinate(file: u8, rank: u8) -> Option<Self> {
        Self::nuova(file, rank)
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
    
    pub fn dalla_stringa(s: &str) -> Option<Self> {
        Self::da_string(s)
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
    
    pub fn flip_vertical(&self) -> Self {
        Casella {
            file: self.file,
            rank: 7 - self.rank,
        }
    }
    
    pub fn e_angolo(&self) -> bool {
        (self.file == 0 || self.file == 7) && (self.rank == 0 || self.rank == 7)
    }
    
    pub fn color(&self) -> u8 {
        (self.file + self.rank) % 2
    }
    
    pub fn distance(&self, other: Casella) -> u8 {
        let file_diff = (self.file as i8 - other.file as i8).abs() as u8;
        let rank_diff = (self.rank as i8 - other.rank as i8).abs() as u8;
        file_diff.max(rank_diff)
    }
    
    pub fn distance_to_center(&self) -> u8 {
        let center_file = 3.5;
        let center_rank = 3.5;
        let file_dist = (self.file as f32 - center_file).abs();
        let rank_dist = (self.rank as f32 - center_rank).abs();
        (file_dist + rank_dist) as u8
    }
}

impl fmt::Display for Casella {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

#[derive(Clone, Copy, Debug)]
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
}

#[derive(Clone)]
pub struct Scacchiera {
    pub pezzi: [[Option<(Pezzo, Colore)>; 8]; 8],
    bitboards: [[Bitboard; 6]; 2], // [colore][pezzo]
    occupazione_totale: Bitboard,
    colore_attivo: Colore,
    diritti_arrocco: DirittiArrocco,
    en_passant: Option<Casella>,
    contatore_semimosse: u32,
    numero_mossa: u32,
    ply_count: u32, // Aggiunto per compatibilità con endgame
}

impl Scacchiera {
    pub fn nuova() -> Self {
        let mut scacchiera = Scacchiera {
            pezzi: [[None; 8]; 8],
            bitboards: [[0; 6]; 2],
            occupazione_totale: 0,
            colore_attivo: Colore::Bianco,
            diritti_arrocco: DirittiArrocco::tutti(),
            en_passant: None,
            contatore_semimosse: 0,
            numero_mossa: 1,
            ply_count: 0,
        };
        
        // Posiziona i pezzi bianchi
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
        
        // Posiziona i pezzi neri
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
        
        scacchiera
    }
    
    fn metti_pezzo(&mut self, casella: Casella, pezzo: Pezzo, colore: Colore) {
        self.pezzi[casella.rank as usize][casella.file as usize] = Some((pezzo, colore));
        let bit = casella.to_bitboard();
        self.bitboards[colore.indice()][pezzo.indice()] |= bit;
        self.occupazione_totale |= bit;
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
            colore_attivo: Colore::Bianco,
            diritti_arrocco: DirittiArrocco::nessuno(),
            en_passant: None,
            contatore_semimosse: 0,
            numero_mossa: 1,
            ply_count: 0,
        };
        
        let righe: Vec<&str> = posizione.split('/').collect();
        if righe.len() != 8 {
            return Err("Posizione FEN deve avere 8 righe".to_string());
        }
        
        for (rank_idx, riga) in righe.iter().enumerate() {
            let rank = (7 - rank_idx) as u8; // FEN parte dall'ottava riga, convertiamo a u8
            let mut file = 0;
            
            for c in riga.chars() {
                if file >= 8 {
                    return Err("Troppi pezzi/campi in una riga".to_string());
                }
                
                if c.is_digit(10) {
                    let vuoti = c.to_digit(10).unwrap() as u8;
                    file += vuoti;
                } else {
                    let (pezzo, colore_pezzo) = match c {
                        'P' => (Pezzo::Pedone, Colore::Bianco),
                        'N' => (Pezzo::Cavallo, Colore::Bianco),
                        'B' => (Pezzo::Alfiere, Colore::Bianco),
                        'R' => (Pezzo::Torre, Colore::Bianco),
                        'Q' => (Pezzo::Regina, Colore::Bianco),
                        'K' => (Pezzo::Re, Colore::Bianco),
                        'p' => (Pezzo::Pedone, Colore::Nero),
                        'n' => (Pezzo::Cavallo, Colore::Nero),
                        'b' => (Pezzo::Alfiere, Colore::Nero),
                        'r' => (Pezzo::Torre, Colore::Nero),
                        'q' => (Pezzo::Regina, Colore::Nero),
                        'k' => (Pezzo::Re, Colore::Nero),
                        _ => return Err(format!("Carattere FEN non valido: {}", c)),
                    };
                    
                    let casella = Casella::nuova(file, rank).unwrap();
                    scacchiera.metti_pezzo(casella, pezzo, colore_pezzo);
                    file += 1;
                }
            }
        }
        
        scacchiera.colore_attivo = match colore {
            "w" => Colore::Bianco,
            "b" => Colore::Nero,
            _ => return Err("Colore attivo non valido".to_string()),
        };
        
        for c in arrocco.chars() {
            match c {
                'K' => scacchiera.diritti_arrocco.bianco_lato_re = true,
                'Q' => scacchiera.diritti_arrocco.bianco_lato_regina = true,
                'k' => scacchiera.diritti_arrocco.nero_lato_re = true,
                'q' => scacchiera.diritti_arrocco.nero_lato_regina = true,
                '-' => break,
                _ => {}
            }
        }
        
        scacchiera.en_passant = if en_passant == "-" {
            None
        } else {
            Casella::da_string(en_passant)
        };
        
        Ok(scacchiera)
    }
    
    pub fn a_fen(&self) -> String {
        let mut fen = String::new();
        
        // Posizione pezzi
        for rank in (0..8).rev() {
            let mut empty_count = 0;
            
            for file in 0..8 {
                if let Some((pezzo, colore)) = self.pezzi[rank][file] {
                    if empty_count > 0 {
                        fen.push_str(&empty_count.to_string());
                        empty_count = 0;
                    }
                    
                    let mut c = match pezzo {
                        Pezzo::Pedone => 'p',
                        Pezzo::Cavallo => 'n',
                        Pezzo::Alfiere => 'b',
                        Pezzo::Torre => 'r',
                        Pezzo::Regina => 'q',
                        Pezzo::Re => 'k',
                    };
                    
                    if colore == Colore::Bianco {
                        c = c.to_ascii_uppercase();
                    }
                    
                    fen.push(c);
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
        
        // Colore attivo
        fen.push(' ');
        fen.push(match self.colore_attivo {
            Colore::Bianco => 'w',
            Colore::Nero => 'b',
        });
        
        // Diritti di arrocco
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
        
        // En passant
        fen.push(' ');
        if let Some(casella) = self.en_passant {
            fen.push_str(&casella.to_string());
        } else {
            fen.push('-');
        }
        
        // Contatori
        fen.push_str(&format!(" {} {}", self.contatore_semimosse, self.numero_mossa));
        
        fen
    }
    
    // Metodi di accesso
    pub fn colore_attivo(&self) -> Colore {
        self.colore_attivo
    }
    
    pub fn set_colore_attivo(&mut self, colore: Colore) {
        self.colore_attivo = colore;
    }
    
    pub fn imposta_colore_attivo(&mut self, colore: Colore) {
        self.colore_attivo = colore;
    }
    
    pub fn diritti_arrocco(&self) -> DirittiArrocco {
        self.diritti_arrocco
    }
    
    pub fn set_diritti_arrocco(&mut self, diritti: DirittiArrocco) {
        self.diritti_arrocco = diritti;
    }
    
    pub fn imposta_diritti_arrocco(&mut self, diritti: DirittiArrocco) {
        self.diritti_arrocco = diritti;
    }
    
    pub fn en_passant(&self) -> Option<Casella> {
        self.en_passant
    }
    
    pub fn set_en_passant(&mut self, casella: Option<Casella>) {
        self.en_passant = casella;
    }
    
    pub fn imposta_en_passant(&mut self, casella: Option<Casella>) {
        self.en_passant = casella;
    }
    
    pub fn ottieni_pezzo(&self, casella: Casella) -> Option<(Pezzo, Colore)> {
        if casella.file < 8 && casella.rank < 8 {
            self.pezzi[casella.rank as usize][casella.file as usize]
        } else {
            None
        }
    }
    
    pub fn imposta_pezzo(&mut self, casella: Casella, pezzo: Option<(Pezzo, Colore)>) {
        if casella.file < 8 && casella.rank < 8 {
            // Rimuovi vecchio pezzo se presente
            if let Some((old_pezzo, old_colore)) = self.pezzi[casella.rank as usize][casella.file as usize] {
                let bit = casella.to_bitboard();
                self.bitboards[old_colore.indice()][old_pezzo.indice()] &= !bit;
                self.occupazione_totale &= !bit;
            }
            
            // Aggiungi nuovo pezzo
            self.pezzi[casella.rank as usize][casella.file as usize] = pezzo;
            if let Some((pezzo, colore)) = pezzo {
                let bit = casella.to_bitboard();
                self.bitboards[colore.indice()][pezzo.indice()] |= bit;
                self.occupazione_totale |= bit;
            }
        }
    }
    
    pub fn rimuovi_pezzo(&mut self, casella: Casella) -> Option<(Pezzo, Colore)> {
        if let Some(pezzo_info) = self.ottieni_pezzo(casella) {
            self.imposta_pezzo(casella, None);
            Some(pezzo_info)
        } else {
            None
        }
    }
    
    pub fn muovi_pezzo(&mut self, da: Casella, a: Casella) -> Option<(Pezzo, Colore)> {
        let pezzo = self.pezzi[da.rank as usize][da.file as usize]?;
        self.pezzi[da.rank as usize][da.file as usize] = None;
        self.pezzi[a.rank as usize][a.file as usize] = Some(pezzo);
        
        // Aggiorna bitboards
        let bit_da = da.to_bitboard();
        let bit_a = a.to_bitboard();
        let (pezzo_tipo, colore) = pezzo;
        
        self.bitboards[colore.indice()][pezzo_tipo.indice()] &= !bit_da;
        self.bitboards[colore.indice()][pezzo_tipo.indice()] |= bit_a;
        
        self.occupazione_totale &= !bit_da;
        self.occupazione_totale |= bit_a;
        
        Some(pezzo)
    }
    
    // Metodi Bitboard
    pub fn occupazione_totale(&self) -> Bitboard {
        self.occupazione_totale
    }
    
    pub fn bitboard_colore(&self, colore: Colore) -> Bitboard {
        let mut bb = 0;
        for pezzo_idx in 0..6 {
            bb |= self.bitboards[colore.indice()][pezzo_idx];
        }
        bb
    }
    
    pub fn bitboard_pezzo(&self, pezzo: Pezzo, colore: Colore) -> Bitboard {
        self.bitboards[colore.indice()][pezzo.indice()]
    }
    
    pub fn occupazione_totale_b(&self) -> Bitboard {
        self.occupazione_totale
    }
    
    // Contatori
    pub fn contatore_semimosse(&self) -> u32 {
        self.contatore_semimosse
    }
    
    pub fn imposta_contatore_semimosse(&mut self, valore: u32) {
        self.contatore_semimosse = valore;
    }
    
    pub fn numero_mossa(&self) -> u32 {
        self.numero_mossa
    }
    
    pub fn imposta_numero_mossa(&mut self, valore: u32) {
        self.numero_mossa = valore;
    }
    
    // METODI AGGIUNTI PER IL SUPPORTO FINALI
    
    /// Restituisce il numero totale di pezzi sulla scacchiera
    pub fn pezzi_totali(&self) -> usize {
        self.occupazione_totale.count_ones() as usize
    }
    
    /// Restituisce il ply (numero di semimosse dall'inizio)
    pub fn ply(&self) -> u32 {
        self.ply_count
    }
    
    /// Imposta il ply count
    pub fn imposta_ply(&mut self, ply: u32) {
        self.ply_count = ply;
    }
    
    /// Incrementa il ply count
    pub fn incrementa_ply(&mut self) {
        self.ply_count += 1;
    }
    
    /// Restituisce il bitboard di tutti i pedoni
    pub fn pedoni(&self) -> Bitboard {
        self.bitboard_pezzo(Pezzo::Pedone, Colore::Bianco) | 
        self.bitboard_pezzo(Pezzo::Pedone, Colore::Nero)
    }
    
    /// Restituisce il bitboard di tutti i cavalli
    pub fn cavalli(&self) -> Bitboard {
        self.bitboard_pezzo(Pezzo::Cavallo, Colore::Bianco) | 
        self.bitboard_pezzo(Pezzo::Cavallo, Colore::Nero)
    }
    
    /// Restituisce il bitboard di tutti gli alfieri
    pub fn alfieri(&self) -> Bitboard {
        self.bitboard_pezzo(Pezzo::Alfiere, Colore::Bianco) | 
        self.bitboard_pezzo(Pezzo::Alfiere, Colore::Nero)
    }
    
    /// Restituisce il bitboard di tutte le torri
    pub fn torri(&self) -> Bitboard {
        self.bitboard_pezzo(Pezzo::Torre, Colore::Bianco) | 
        self.bitboard_pezzo(Pezzo::Torre, Colore::Nero)
    }
    
    /// Restituisce il bitboard di tutte le regine
    pub fn regine(&self) -> Bitboard {
        self.bitboard_pezzo(Pezzo::Regina, Colore::Bianco) | 
        self.bitboard_pezzo(Pezzo::Regina, Colore::Nero)
    }
    
    /// Conta i pedoni di un colore specifico
    pub fn pedoni_colore(&self, colore: Colore) -> Bitboard {
        self.bitboard_pezzo(Pezzo::Pedone, colore)
    }
    
    /// Conta i cavalli di un colore specifico
    pub fn cavalli_colore(&self, colore: Colore) -> Bitboard {
        self.bitboard_pezzo(Pezzo::Cavallo, colore)
    }
    
    /// Conta gli alfieri di un colore specifico
    pub fn alfieri_colore(&self, colore: Colore) -> Bitboard {
        self.bitboard_pezzo(Pezzo::Alfiere, colore)
    }
    
    /// Conta le torri di un colore specifico
    pub fn torri_colore(&self, colore: Colore) -> Bitboard {
        self.bitboard_pezzo(Pezzo::Torre, colore)
    }
    
    /// Conta le regine di un colore specifico
    pub fn regine_colore(&self, colore: Colore) -> Bitboard {
        self.bitboard_pezzo(Pezzo::Regina, colore)
    }
    
    /// Restituisce la casella del re di un colore
    pub fn casella_re(&self, colore: Colore) -> Casella {
        self.trova_re(colore).unwrap_or_else(|| {
            // Fallback: restituisce una casella valida
            if colore == Colore::Bianco {
                Casella::nuova(4, 0).unwrap()
            } else {
                Casella::nuova(4, 7).unwrap()
            }
        })
    }
    
    /// Controlla se la scacchiera ha una regina del colore specificato
    pub fn ha_regina(&self, colore: Colore) -> bool {
        self.bitboard_pezzo(Pezzo::Regina, colore) != 0
    }
    
    /// Conta il numero totale di regine
    pub fn regine_totali(&self) -> u8 {
        (self.bitboard_pezzo(Pezzo::Regina, Colore::Bianco) | 
         self.bitboard_pezzo(Pezzo::Regina, Colore::Nero)).count_ones() as u8
    }
    
    /// Conta il numero totale di torri
    pub fn torri_totali(&self) -> u8 {
        (self.bitboard_pezzo(Pezzo::Torre, Colore::Bianco) | 
         self.bitboard_pezzo(Pezzo::Torre, Colore::Nero)).count_ones() as u8
    }
    
    /// Conta il numero totale di alfieri e cavalli
    pub fn alfieri_cavalli_totali(&self) -> u8 {
        ((self.bitboard_pezzo(Pezzo::Alfiere, Colore::Bianco) | 
          self.bitboard_pezzo(Pezzo::Alfiere, Colore::Nero)).count_ones() +
         (self.bitboard_pezzo(Pezzo::Cavallo, Colore::Bianco) | 
          self.bitboard_pezzo(Pezzo::Cavallo, Colore::Nero)).count_ones()) as u8
    }
    
    /// Conta il numero totale di pedoni
    pub fn pedoni_totali(&self) -> u8 {
        (self.bitboard_pezzo(Pezzo::Pedone, Colore::Bianco) | 
         self.bitboard_pezzo(Pezzo::Pedone, Colore::Nero)).count_ones() as u8
    }
    
    /// Controlla se è una patta per ripetizione o 50 mosse
    pub fn e_patta(&self) -> bool {
        // Implementazione semplificata: controlla solo il contatore delle 50 mosse
        self.contatore_semimosse >= 100 // 100 semimosse = 50 mosse
    }
    
    /// Controlla se è una posizione di scacco matto
    pub fn e_scacco_matto(&self) -> bool {
        self.re_in_scacco(self.colore_attivo)
    }
    
    /// Controlla se è stallo
    pub fn e_stallo(&self) -> bool {
        !self.re_in_scacco(self.colore_attivo) 
        // Nota: Per essere completo, dovremmo controllare se ci sono mosse legali
    }
    
    // Metodi di utilità
    pub fn trova_re(&self, colore: Colore) -> Option<Casella> {
        let re_bb = self.bitboard_pezzo(Pezzo::Re, colore);
        if re_bb == 0 {
            return None;
        }
        
        // Trova il bit set (dovrebbe essercene solo uno)
        let indice = re_bb.trailing_zeros() as usize;
        Casella::da_indice(indice)
    }
    
    pub fn re_in_scacco(&self, colore: Colore) -> bool {
        if let Some(pos_re) = self.trova_re(colore) {
            self.casella_attaccata(pos_re, colore.opposto())
        } else {
            false
        }
    }
    
    pub fn casella_attaccata(&self, casella: Casella, da_colore: Colore) -> bool {
        let indice = casella.indice();
        let bitboard_casella = 1u64 << indice;
        
        // Controlla pedoni
        let direzione = da_colore.direzione_pedone();
        let pedoni = self.bitboard_pezzo(Pezzo::Pedone, da_colore);
        
        // Catture di pedone
        let catture_pedone = if direzione > 0 {
            // Bianchi (muovono verso rank più alti)
            ((pedoni << 7) & !FILE_H) | ((pedoni << 9) & !FILE_A)
        } else {
            // Neri (muovono verso rank più bassi)
            ((pedoni >> 7) & !FILE_A) | ((pedoni >> 9) & !FILE_H)
        };
        
        if (catture_pedone & bitboard_casella) != 0 {
            return true;
        }
        
        // Controlla cavalli
        let cavalli = self.bitboard_pezzo(Pezzo::Cavallo, da_colore);
        let attacchi_cavallo = Tables::globale().attacchi_cavallo[indice];
        if (cavalli & attacchi_cavallo) != 0 {
            return true;
        }
        
        // Controlla re
        let re = self.bitboard_pezzo(Pezzo::Re, da_colore);
        let attacchi_re = Tables::globale().attacchi_re[indice];
        if (re & attacchi_re) != 0 {
            return true;
        }
        
        // Controlla pezzi a slittamento (alfieri, torri, regine)
        let occupazione = self.occupazione_totale();
        
        // Alfieri e regine (diagonali)
        let alfieri = self.bitboard_pezzo(Pezzo::Alfiere, da_colore);
        let regine = self.bitboard_pezzo(Pezzo::Regina, da_colore);
        let pezzi_diagonali = alfieri | regine;
        
        if pezzi_diagonali != 0 {
            let attacchi_alfiere = Tables::globale().attacchi_alfiere(indice, occupazione);
            if (pezzi_diagonali & attacchi_alfiere) != 0 {
                return true;
            }
        }
        
        // Torri e regine (orizzontali/verticali)
        let torri = self.bitboard_pezzo(Pezzo::Torre, da_colore);
        let pezzi_lineari = torri | regine;
        
        if pezzi_lineari != 0 {
            let attacchi_torre = Tables::globale().attacchi_torre(indice, occupazione);
            if (pezzi_lineari & attacchi_torre) != 0 {
                return true;
            }
        }
        
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
        
        println!("Colore attivo: {:?}", self.colore_attivo);
        println!("En passant: {:?}", self.en_passant);
        println!("Ply: {}", self.ply_count);
    }
    
    pub fn clone(&self) -> Self {
        Scacchiera {
            pezzi: self.pezzi,
            bitboards: self.bitboards,
            occupazione_totale: self.occupazione_totale,
            colore_attivo: self.colore_attivo,
            diritti_arrocco: self.diritti_arrocco,
            en_passant: self.en_passant,
            contatore_semimosse: self.contatore_semimosse,
            numero_mossa: self.numero_mossa,
            ply_count: self.ply_count,
        }
    }
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
        
        write!(f, "Colore attivo: {:?}, Ply: {}", self.colore_attivo, self.ply_count)
    }
}

// Costanti Bitboard
const FILE_A: Bitboard = 0x0101010101010101;
const FILE_H: Bitboard = 0x8080808080808080;
const RANK_1: Bitboard = 0x00000000000000FF;
const RANK_8: Bitboard = 0xFF00000000000000;

// Struttura per le tabelle precalcolate
pub struct Tables {
    pub attacchi_cavallo: [Bitboard; 64],
    pub attacchi_re: [Bitboard; 64],
    pub attacchi_pedone: [[Bitboard; 64]; 2],
}

impl Tables {
    pub fn globale() -> &'static Tables {
        static INSTANCE: std::sync::OnceLock<Tables> = std::sync::OnceLock::new();
        INSTANCE.get_or_init(|| Tables::nuova())
    }
    
    fn nuova() -> Tables {
        let mut tables = Tables {
            attacchi_cavallo: [0; 64],
            attacchi_re: [0; 64],
            attacchi_pedone: [[0; 64]; 2],
        };
        
        // Inizializza tutte le tabelle
        for square in 0..64 {
            tables.attacchi_cavallo[square] = tables.calcola_attacchi_cavallo(square);
            tables.attacchi_re[square] = tables.calcola_attacchi_re(square);
            tables.attacchi_pedone[Colore::Bianco.indice()][square] = tables.calcola_attacchi_pedone(square, Colore::Bianco);
            tables.attacchi_pedone[Colore::Nero.indice()][square] = tables.calcola_attacchi_pedone(square, Colore::Nero);
        }
        
        tables
    }
    
    fn calcola_attacchi_cavallo(&self, square: usize) -> Bitboard {
        let mut attacks = 0;
        let (file, rank) = (square % 8, square / 8);
        
        let offsets = [(-2, -1), (-2, 1), (-1, -2), (-1, 2),
                       (1, -2), (1, 2), (2, -1), (2, 1)];
        
        for (df, dr) in offsets {
            let new_file = file as i32 + df;
            let new_rank = rank as i32 + dr;
            
            if new_file >= 0 && new_file < 8 && new_rank >= 0 && new_rank < 8 {
                attacks |= 1u64 << ((new_rank * 8 + new_file) as u64);
            }
        }
        
        attacks
    }
    
    fn calcola_attacchi_re(&self, square: usize) -> Bitboard {
        let mut attacks = 0;
        let (file, rank) = (square % 8, square / 8);
        
        for df in -1..=1 {
            for dr in -1..=1 {
                if df == 0 && dr == 0 {
                    continue;
                }
                
                let new_file = file as i32 + df;
                let new_rank = rank as i32 + dr;
                
                if new_file >= 0 && new_file < 8 && new_rank >= 0 && new_rank < 8 {
                    attacks |= 1u64 << ((new_rank * 8 + new_file) as u64);
                }
            }
        }
        
        attacks
    }
    
    fn calcola_attacchi_pedone(&self, square: usize, colore: Colore) -> Bitboard {
        let mut attacks = 0;
        let file = square % 8;
        let rank = square / 8;
        
        let direzione = colore.direzione_pedone();
        let new_rank = rank as i32 + direzione;
        
        if new_rank >= 0 && new_rank < 8 {
            // Cattura a sinistra
            if file > 0 {
                attacks |= 1u64 << ((new_rank * 8 + file as i32 - 1) as u64);
            }
            // Cattura a destra
            if file < 7 {
                attacks |= 1u64 << ((new_rank * 8 + file as i32 + 1) as u64);
            }
        }
        
        attacks
    }
    
    pub fn attacchi_alfiere(&self, square: usize, occupazione: Bitboard) -> Bitboard {
        // Implementazione semplificata
        let mut attacks = 0;
        let (file, rank) = (square % 8, square / 8);
        
        // Diagonale nord-est
        let mut f = file as i32 + 1;
        let mut r = rank as i32 + 1;
        while f < 8 && r < 8 {
            let bit = 1u64 << ((r * 8 + f) as u64);
            attacks |= bit;
            if (occupazione & bit) != 0 {
                break;
            }
            f += 1;
            r += 1;
        }
        
        // Diagonale nord-ovest
        let mut f = file as i32 - 1;
        let mut r = rank as i32 + 1;
        while f >= 0 && r < 8 {
            let bit = 1u64 << ((r * 8 + f) as u64);
            attacks |= bit;
            if (occupazione & bit) != 0 {
                break;
            }
            f -= 1;
            r += 1;
        }
        
        // Diagonale sud-est
        let mut f = file as i32 + 1;
        let mut r = rank as i32 - 1;
        while f < 8 && r >= 0 {
            let bit = 1u64 << ((r * 8 + f) as u64);
            attacks |= bit;
            if (occupazione & bit) != 0 {
                break;
            }
            f += 1;
            r -= 1;
        }
        
        // Diagonale sud-ovest
        let mut f = file as i32 - 1;
        let mut r = rank as i32 - 1;
        while f >= 0 && r >= 0 {
            let bit = 1u64 << ((r * 8 + f) as u64);
            attacks |= bit;
            if (occupazione & bit) != 0 {
                break;
            }
            f -= 1;
            r -= 1;
        }
        
        attacks
    }
    
    pub fn attacchi_torre(&self, square: usize, occupazione: Bitboard) -> Bitboard {
        let mut attacks = 0;
        let (file, rank) = (square % 8, square / 8);
        
        // Nord
        for r in (rank + 1)..8 {
            let bit = 1u64 << ((r * 8 + file) as u64);
            attacks |= bit;
            if (occupazione & bit) != 0 {
                break;
            }
        }
        
        // Sud
        for r in (0..rank).rev() {
            let bit = 1u64 << ((r * 8 + file) as u64);
            attacks |= bit;
            if (occupazione & bit) != 0 {
                break;
            }
        }
        
        // Est
        for f in (file + 1)..8 {
            let bit = 1u64 << ((rank * 8 + f) as u64);
            attacks |= bit;
            if (occupazione & bit) != 0 {
                break;
            }
        }
        
        // Ovest
        for f in (0..file).rev() {
            let bit = 1u64 << ((rank * 8 + f) as u64);
            attacks |= bit;
            if (occupazione & bit) != 0 {
                break;
            }
        }
        
        attacks
    }
}