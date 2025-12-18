use std::fmt;

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Pezzo {
    Pedone,
    Cavallo,
    Alfiere,
    Torre,
    Regina,
    Re,
}

#[derive(Copy, Clone, PartialEq, Debug)]
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
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Casella(u8);

impl Casella {
    pub fn nuova(lettera: u8, numero: u8) -> Option<Self> {
        if lettera < 8 && numero < 8 {
            Some(Casella(numero * 8 + lettera))
        } else {
            None
        }
    }

    pub fn da_indice(indice: u8) -> Option<Self> {
        if indice < 64 {
            Some(Casella(indice))
        } else {
            None
        }
    }

    pub fn indice(&self) -> u8 {
        self.0
    }

    pub fn lettera(&self) -> u8 {
        self.0 % 8
    }

    pub fn numero(&self) -> u8 {
        self.0 / 8
    }
}

impl fmt::Display for Casella {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let lettera = (b'a' + self.lettera()) as char;
        let numero = self.numero() + 1;
        write!(f, "{}{}", lettera, numero)
    }
}

#[derive(Clone)]
pub struct Scacchiera {
    pezzi: [[Option<(Pezzo, Colore)>; 8]; 8],
    colore_attivo: Colore,
    diritti_arrocco: DirittiArrocco,
    en_passant: Option<Casella>,
    contatore_semimosse: u32,
    numero_mossa: u32,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct DirittiArrocco {
    pub bianco_lato_re: bool,
    pub bianco_lato_regina: bool,
    pub nero_lato_re: bool,
    pub nero_lato_regina: bool,
}

impl Scacchiera {
    pub fn nuova() -> Self {
        Scacchiera::da_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap()
    }

    pub fn da_fen(fen: &str) -> Result<Self, &'static str> {
        let mut scacchiera = Scacchiera {
            pezzi: [[None; 8]; 8],
            colore_attivo: Colore::Bianco,
            diritti_arrocco: DirittiArrocco {
                bianco_lato_re: false,
                bianco_lato_regina: false,
                nero_lato_re: false,
                nero_lato_regina: false,
            },
            en_passant: None,
            contatore_semimosse: 0,
            numero_mossa: 1,
        };

        let parti: Vec<&str> = fen.split_whitespace().collect();
        if parti.len() < 4 {
            return Err("Stringa FEN troppo corta");
        }

        // Analizza posizione pezzi
        let traverse: Vec<&str> = parti[0].split('/').collect();
        if traverse.len() != 8 {
            return Err("Numero di traverse non valido in FEN");
        }

        for (indice_traversa, str_traversa) in traverse.iter().enumerate() {
            let mut indice_lettera = 0;
            for c in str_traversa.chars() {
                if indice_lettera >= 8 {
                    return Err("Troppe lettere nella traversa");
                }

                if let Some(cifra) = c.to_digit(10) {
                    indice_lettera += cifra as usize;
                } else {
                    let pezzo = match c.to_ascii_lowercase() {
                        'p' => Pezzo::Pedone,
                        'n' => Pezzo::Cavallo,
                        'b' => Pezzo::Alfiere,
                        'r' => Pezzo::Torre,
                        'q' => Pezzo::Regina,
                        'k' => Pezzo::Re,
                        _ => return Err("Carattere pezzo non valido"),
                    };
                    let colore = if c.is_uppercase() { Colore::Bianco } else { Colore::Nero };
                    scacchiera.pezzi[7 - indice_traversa][indice_lettera] = Some((pezzo, colore));
                    indice_lettera += 1;
                }
            }
        }

        // Analizza colore attivo
        scacchiera.colore_attivo = match parti[1] {
            "w" => Colore::Bianco,
            "b" => Colore::Nero,
            _ => return Err("Colore attivo non valido"),
        };

        // Analizza diritti di arrocco
        if parti[2] != "-" {
            for c in parti[2].chars() {
                match c {
                    'K' => scacchiera.diritti_arrocco.bianco_lato_re = true,
                    'Q' => scacchiera.diritti_arrocco.bianco_lato_regina = true,
                    'k' => scacchiera.diritti_arrocco.nero_lato_re = true,
                    'q' => scacchiera.diritti_arrocco.nero_lato_regina = true,
                    _ => return Err("Diritto di arrocco non valido"),
                }
            }
        }

        // Analizza en passant
        if parti[3] != "-" {
            let caratteri: Vec<char> = parti[3].chars().collect();
            if caratteri.len() == 2 {
                let lettera = (caratteri[0] as u8) - b'a';
                let numero = (caratteri[1] as u8) - b'1';
                if lettera < 8 && numero < 8 {
                    scacchiera.en_passant = Casella::nuova(lettera, numero);
                }
            }
        }

        // Analizza contatore semimosse e numero mossa
        if parti.len() >= 5 {
            scacchiera.contatore_semimosse = parti[4].parse().unwrap_or(0);
        }
        if parti.len() >= 6 {
            scacchiera.numero_mossa = parti[5].parse().unwrap_or(1);
        }

        Ok(scacchiera)
    }

    pub fn ottieni_pezzo(&self, casella: Casella) -> Option<(Pezzo, Colore)> {
        let numero = (casella.indice() / 8) as usize;
        let lettera = (casella.indice() % 8) as usize;
        self.pezzi[numero][lettera]
    }

    pub fn imposta_pezzo(&mut self, casella: Casella, pezzo: Option<(Pezzo, Colore)>) {
        let numero = (casella.indice() / 8) as usize;
        let lettera = (casella.indice() % 8) as usize;
        self.pezzi[numero][lettera] = pezzo;
    }

    pub fn colore_attivo(&self) -> Colore {
        self.colore_attivo
    }

    pub fn diritti_arrocco(&self) -> DirittiArrocco {
        self.diritti_arrocco
    }

    pub fn en_passant(&self) -> Option<Casella> {
        self.en_passant
    }

    pub fn contatore_semimosse(&self) -> u32 {
        self.contatore_semimosse
    }

    pub fn numero_mossa(&self) -> u32 {
        self.numero_mossa
    }

    pub fn imposta_colore_attivo(&mut self, colore: Colore) {
        self.colore_attivo = colore;
    }

    pub fn imposta_diritti_arrocco(&mut self, diritti: DirittiArrocco) {
        self.diritti_arrocco = diritti;
    }

    pub fn imposta_en_passant(&mut self, casella: Option<Casella>) {
        self.en_passant = casella;
    }

    pub fn imposta_contatore_semimosse(&mut self, contatore: u32) {
        self.contatore_semimosse = contatore;
    }

    pub fn imposta_numero_mossa(&mut self, numero: u32) {
        self.numero_mossa = numero;
    }

    pub fn esegui_mossa(&mut self, mossa: &crate::movegen::Mossa) -> bool {
        let pezzo_partenza = self.ottieni_pezzo(mossa.da);
        if pezzo_partenza.is_none() {
            return false;
        }

        let (pezzo, colore) = pezzo_partenza.unwrap();

        // Basic move
        self.imposta_pezzo(mossa.a, Some((pezzo, colore)));
        self.imposta_pezzo(mossa.da, None);

        // Handle pawn promotion
        if let Some(promotion_piece) = mossa.promozione {
            self.imposta_pezzo(mossa.a, Some((promotion_piece, colore)));
        }

        // Update active color
        self.colore_attivo = self.colore_attivo.opposto();

        true
    }

    pub fn casella_attaccata(&self, casella: Casella, da_colore: Colore) -> bool {
        use Pezzo::*;

        // Check pawn attacks
        let pawn_dir = match da_colore {
            Colore::Bianco => 1,
            Colore::Nero => -1,
        };

        let pawn_attacks = [
            (casella.lettera() as i8 - 1, casella.numero() as i8 + pawn_dir),
            (casella.lettera() as i8 + 1, casella.numero() as i8 + pawn_dir),
        ];

        for &(file, rank) in &pawn_attacks {
            if file >= 0 && file < 8 && rank >= 0 && rank < 8 {
                if let Some(sq) = Casella::nuova(file as u8, rank as u8) {
                    if let Some((piece, color)) = self.ottieni_pezzo(sq) {
                        if piece == Pedone && color == da_colore {
                            return true;
                        }
                    }
                }
            }
        }

        // Check knight attacks
        let knight_moves = [
            (-2, -1), (-2, 1), (-1, -2), (-1, 2),
            (1, -2), (1, 2), (2, -1), (2, 1),
        ];

        for &(dx, dy) in &knight_moves {
            let file = casella.lettera() as i8 + dx;
            let rank = casella.numero() as i8 + dy;

            if file >= 0 && file < 8 && rank >= 0 && rank < 8 {
                if let Some(sq) = Casella::nuova(file as u8, rank as u8) {
                    if let Some((piece, color)) = self.ottieni_pezzo(sq) {
                        if piece == Cavallo && color == da_colore {
                            return true;
                        }
                    }
                }
            }
        }

        // Check sliding pieces (queen, rook, bishop)
        let directions = [
            (1, 0, vec![Torre, Regina]),   // right
            (-1, 0, vec![Torre, Regina]),  // left
            (0, 1, vec![Torre, Regina]),   // up
            (0, -1, vec![Torre, Regina]),  // down
            (1, 1, vec![Alfiere, Regina]), // up-right
            (1, -1, vec![Alfiere, Regina]),// down-right
            (-1, 1, vec![Alfiere, Regina]),// up-left
            (-1, -1, vec![Alfiere, Regina]),// down-left
        ];

        for &(dx, dy, ref pieces) in &directions {
            let mut file = casella.lettera() as i8 + dx;
            let mut rank = casella.numero() as i8 + dy;

            while file >= 0 && file < 8 && rank >= 0 && rank < 8 {
                if let Some(sq) = Casella::nuova(file as u8, rank as u8) {
                    if let Some((piece, color)) = self.ottieni_pezzo(sq) {
                        if color == da_colore && pieces.contains(&piece) {
                            return true;
                        }
                        // Stop if we hit any piece
                        break;
                    }
                }
                file += dx;
                rank += dy;
            }
        }

        // Check king attacks (adjacent squares)
        let king_moves = [
            (-1, -1), (-1, 0), (-1, 1),
            (0, -1),          (0, 1),
            (1, -1),  (1, 0), (1, 1),
        ];

        for &(dx, dy) in &king_moves {
            let file = casella.lettera() as i8 + dx;
            let rank = casella.numero() as i8 + dy;

            if file >= 0 && file < 8 && rank >= 0 && rank < 8 {
                if let Some(sq) = Casella::nuova(file as u8, rank as u8) {
                    if let Some((piece, color)) = self.ottieni_pezzo(sq) {
                        if piece == Re && color == da_colore {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }

    pub fn in_scacco(&self, colore: Colore) -> bool {
        // Trova posizione del re
        let mut casella_re = None;
        for numero in 0..8 {
            for lettera in 0..8 {
                if let Some(cas) = Casella::nuova(lettera as u8, numero as u8) {
                    if let Some((pezzo, colore_pezzo)) = self.ottieni_pezzo(cas) {
                        if pezzo == Pezzo::Re && colore_pezzo == colore {
                            casella_re = Some(cas);
                            break;
                        }
                    }
                }
            }
            if casella_re.is_some() {
                break;
            }
        }

        if let Some(re) = casella_re {
            self.casella_attaccata(re, colore.opposto())
        } else {
            false
        }
    }
}

impl fmt::Display for Scacchiera {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for numero in (0..8).rev() {
            write!(f, "{} ", numero + 1)?;
            for lettera in 0..8 {
                let casella = Casella::nuova(lettera, numero).unwrap();
                match self.ottieni_pezzo(casella) {
                    Some((pezzo, colore)) => {
                        let simbolo = match (pezzo, colore) {
                            (Pezzo::Pedone, Colore::Bianco) => '♙',
                            (Pezzo::Cavallo, Colore::Bianco) => '♘',
                            (Pezzo::Alfiere, Colore::Bianco) => '♗',
                            (Pezzo::Torre, Colore::Bianco) => '♖',
                            (Pezzo::Regina, Colore::Bianco) => '♕',
                            (Pezzo::Re, Colore::Bianco) => '♔',
                            (Pezzo::Pedone, Colore::Nero) => '♟',
                            (Pezzo::Cavallo, Colore::Nero) => '♞',
                            (Pezzo::Alfiere, Colore::Nero) => '♝',
                            (Pezzo::Torre, Colore::Nero) => '♜',
                            (Pezzo::Regina, Colore::Nero) => '♛',
                            (Pezzo::Re, Colore::Nero) => '♚',
                        };
                        write!(f, "{} ", simbolo)?;
                    }
                    None => write!(f, ". ")?,
                }
            }
            writeln!(f)?;
        }
        writeln!(f, "  a b c d e f g h")?;
        write!(f, "Colore attivo: {:?}", self.colore_attivo)
    }
}