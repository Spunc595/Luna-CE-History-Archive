use crate::board::{Bitboard, Scacchiera, Pezzo, Colore, Casella};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mossa {
    pub da: Casella,
    pub a: Casella,
    pub promozione: Option<Pezzo>,
}

impl Mossa {
    pub fn nuova(da: Casella, a: Casella) -> Self {
        Self {
            da,
            a,
            promozione: None,
        }
    }
    
    pub fn nuova_con_promozione(da: Casella, a: Casella, promozione: Pezzo) -> Self {
        Self {
            da,
            a,
            promozione: Some(promozione),
        }
    }
}

impl fmt::Display for Mossa {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}", self.da, self.a)?;
        if let Some(pezzo) = self.promozione {
            let promozione_char = match pezzo {
                Pezzo::Regina => 'q',
                Pezzo::Torre => 'r',
                Pezzo::Alfiere => 'b',
                Pezzo::Cavallo => 'n',
                _ => 'q',
            };
            write!(f, "{}", promozione_char)?;
        }
        Ok(())
    }
}

// ==================== TABELLE PRECALCOLATE PER BITBOARD ====================

pub struct BitboardTables {
    mosse_cavallo: [Bitboard; 64],
    mosse_re: [Bitboard; 64],
    attacchi_pedone: [[Bitboard; 64]; 2],
    mask_torre: [Bitboard; 64],
    mask_alfiere: [Bitboard; 64],
}

impl BitboardTables {
    pub fn global() -> &'static BitboardTables {
        static INSTANCE: std::sync::OnceLock<BitboardTables> = std::sync::OnceLock::new();
        INSTANCE.get_or_init(|| BitboardTables::nuova())
    }
    
    pub fn nuova() -> Self {
        let mut tables = BitboardTables {
            mosse_cavallo: [Bitboard::EMPTY; 64],
            mosse_re: [Bitboard::EMPTY; 64],
            attacchi_pedone: [[Bitboard::EMPTY; 64]; 2],
            mask_torre: [Bitboard::EMPTY; 64],
            mask_alfiere: [Bitboard::EMPTY; 64],
        };
        
        tables.precalcola_tutto();
        tables
    }
    
    fn precalcola_tutto(&mut self) {
        for square in 0..64 {
            let square_u8 = square as u8;
            self.mosse_cavallo[square] = self.calcola_mosse_cavallo(square_u8);
            self.mosse_re[square] = self.calcola_mosse_re(square_u8);
            self.attacchi_pedone[Colore::Bianco.indice()][square] = self.calcola_attacchi_pedone(square_u8, Colore::Bianco);
            self.attacchi_pedone[Colore::Nero.indice()][square] = self.calcola_attacchi_pedone(square_u8, Colore::Nero);
            self.mask_torre[square] = self.calcola_mask_torre(square_u8);
            self.mask_alfiere[square] = self.calcola_mask_alfiere(square_u8);
        }
    }
    
    fn calcola_mosse_cavallo(&self, square: u8) -> Bitboard {
        let mut bb = Bitboard::nuova();
        let file = (square % 8) as i8;
        let rank = (square / 8) as i8;
        
        let mosse = [
            (2, 1), (2, -1), (-2, 1), (-2, -1),
            (1, 2), (1, -2), (-1, 2), (-1, -2),
        ];
        
        for (dx, dy) in mosse {
            let new_file = file + dx;
            let new_rank = rank + dy;
            
            if new_file >= 0 && new_file < 8 && new_rank >= 0 && new_rank < 8 {
                let new_square = (new_rank * 8 + new_file) as u8;
                bb.imposta_casella(new_square);
            }
        }
        bb
    }
    
    fn calcola_mosse_re(&self, square: u8) -> Bitboard {
        let mut bb = Bitboard::nuova();
        let file = (square % 8) as i8;
        let rank = (square / 8) as i8;
        
        for dx in -1..=1 {
            for dy in -1..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                
                let new_file = file + dx;
                let new_rank = rank + dy;
                
                if new_file >= 0 && new_file < 8 && new_rank >= 0 && new_rank < 8 {
                    let new_square = (new_rank * 8 + new_file) as u8;
                    bb.imposta_casella(new_square);
                }
            }
        }
        bb
    }
    
    fn calcola_attacchi_pedone(&self, square: u8, colore: Colore) -> Bitboard {
        let mut bb = Bitboard::nuova();
        let file = (square % 8) as i8;
        let rank = (square / 8) as i8;
        
        let direzione = match colore {
            Colore::Bianco => 1,
            Colore::Nero => -1,
        };
        
        for dx in [-1, 1] {
            let new_file = file + dx;
            let new_rank = rank + direzione;
            
            if new_file >= 0 && new_file < 8 && new_rank >= 0 && new_rank < 8 {
                let new_square = (new_rank * 8 + new_file) as u8;
                bb.imposta_casella(new_square);
            }
        }
        bb
    }
    
    fn calcola_mask_torre(&self, square: u8) -> Bitboard {
        let mut bb = Bitboard::nuova();
        let file = square % 8;
        let rank = square / 8;
        
        for r in 1..7 {
            if r != rank {
                bb.imposta_casella(r * 8 + file);
            }
        }
        
        for f in 1..7 {
            if f != file {
                bb.imposta_casella(rank * 8 + f);
            }
        }
        
        bb
    }
    
    fn calcola_mask_alfiere(&self, square: u8) -> Bitboard {
        let mut bb = Bitboard::nuova();
        let file = square % 8;
        let rank = square / 8;
        
        let mut f = file as i8 + 1;
        let mut r = rank as i8 + 1;
        while f < 7 && r < 7 {
            bb.imposta_casella((r * 8 + f) as u8);
            f += 1;
            r += 1;
        }
        
        let mut f = file as i8 - 1;
        let mut r = rank as i8 + 1;
        while f > 0 && r < 7 {
            bb.imposta_casella((r * 8 + f) as u8);
            f -= 1;
            r += 1;
        }
        
        let mut f = file as i8 - 1;
        let mut r = rank as i8 - 1;
        while f > 0 && r > 0 {
            bb.imposta_casella((r * 8 + f) as u8);
            f -= 1;
            r -= 1;
        }
        
        let mut f = file as i8 + 1;
        let mut r = rank as i8 - 1;
        while f < 7 && r > 0 {
            bb.imposta_casella((r * 8 + f) as u8);
            f += 1;
            r -= 1;
        }
        
        bb
    }
    
    pub fn mosse_cavallo(square: u8) -> Bitboard {
        Self::global().mosse_cavallo[square as usize]
    }
    
    pub fn mosse_re(square: u8) -> Bitboard {
        Self::global().mosse_re[square as usize]
    }
    
    pub fn attacchi_pedone(square: u8, colore: Colore) -> Bitboard {
        Self::global().attacchi_pedone[colore.indice()][square as usize]
    }
    
    pub fn attacchi_torre(square: u8, occupazione: Bitboard) -> Bitboard {
        Self::attacchi_scorrevoli(square, occupazione, true)
    }
    
    pub fn attacchi_alfiere(square: u8, occupazione: Bitboard) -> Bitboard {
        Self::attacchi_scorrevoli(square, occupazione, false)
    }
    
    pub fn attacchi_regina(square: u8, occupazione: Bitboard) -> Bitboard {
        Self::attacchi_torre(square, occupazione) | Self::attacchi_alfiere(square, occupazione)
    }
    
    fn attacchi_scorrevoli(square: u8, occupazione: Bitboard, is_torre: bool) -> Bitboard {
        let mut attacks = Bitboard::EMPTY;
        let directions = if is_torre {
            [(1, 0), (-1, 0), (0, 1), (0, -1)]
        } else {
            [(1, 1), (1, -1), (-1, 1), (-1, -1)]
        };
        
        for (dx, dy) in directions {
            let mut current = square;
            loop {
                let file = (current % 8) as i8 + dx;
                let rank = (current / 8) as i8 + dy;
                
                if file < 0 || file >= 8 || rank < 0 || rank >= 8 {
                    break;
                }
                
                let next_square = (rank * 8 + file) as u8;
                attacks.imposta_casella(next_square);
                
                if occupazione.contiene_casella(next_square) {
                    break;
                }
                
                current = next_square;
            }
        }
        
        attacks
    }
}

// ==================== GENERAZIONE MOSSE CON BITBOARD ====================

pub fn genera_mosse(scacchiera: &Scacchiera) -> Vec<Mossa> {
    let mut mosse = Vec::new();
    let colore = scacchiera.colore_attivo();
    let avversario = colore.opposto();
    
    let occupazione_totale = scacchiera.occupazione_totale();
    let occupazione_colore = scacchiera.bitboard_colore(colore);
    let occupazione_avversario = scacchiera.bitboard_colore(avversario);
    let caselle_vuote = !occupazione_totale;
    
    genera_mosse_pedone_bitboard(scacchiera, colore, caselle_vuote, occupazione_avversario, &mut mosse);
    genera_mosse_cavallo_bitboard(scacchiera, colore, occupazione_colore, &mut mosse);
    genera_mosse_alfiere_bitboard(scacchiera, colore, occupazione_totale, occupazione_colore, &mut mosse);
    genera_mosse_torre_bitboard(scacchiera, colore, occupazione_totale, occupazione_colore, &mut mosse);
    genera_mosse_regina_bitboard(scacchiera, colore, occupazione_totale, occupazione_colore, &mut mosse);
    genera_mosse_re_bitboard(scacchiera, colore, occupazione_colore, &mut mosse);
    
    mosse
}

fn genera_mosse_pedone_bitboard(scacchiera: &Scacchiera, colore: Colore, caselle_vuote: Bitboard, 
                                occupazione_avversario: Bitboard, mosse: &mut Vec<Mossa>) {
    let pedoni = scacchiera.bitboard_pezzo(Pezzo::Pedone, colore);
    if pedoni.is_vuota() {
        return;
    }
    
    let direzione = colore.direzione_pedone();
    let rank_iniziale = if colore == Colore::Bianco { 1 } else { 6 };
    let rank_promozione = if colore == Colore::Bianco { 6 } else { 1 }; // Nota: questo è il rank PRIMA della promozione
    
    let mut pedoni_temp = pedoni;
    while let Some(square) = pedoni_temp.pop_lsb() {
        let casella_da = Casella::da_indice(square).unwrap();
        let _file = square % 8;
        let rank = square / 8;
        
        // Mossa avanti di una casella
        let target_square = (square as i16 + (direzione * 8) as i16) as u8;
        if target_square < 64 {
            let target_casella = Casella::da_indice(target_square).unwrap();
            let target_rank = target_square / 8;
            
            if caselle_vuote.contiene_casella(target_square) {
                // Controlla se è una promozione
                if (colore == Colore::Bianco && target_rank == 7) || (colore == Colore::Nero && target_rank == 0) {
                    // Promozione
                    for pezzo in &[Pezzo::Regina, Pezzo::Torre, Pezzo::Alfiere, Pezzo::Cavallo] {
                        mosse.push(Mossa::nuova_con_promozione(casella_da, target_casella, *pezzo));
                    }
                } else {
                    mosse.push(Mossa::nuova(casella_da, target_casella));
                    
                    // Mossa avanti di due caselle dalla posizione iniziale
                    if rank == rank_iniziale {
                        let double_square = (square as i16 + (direzione * 16) as i16) as u8;
                        if double_square < 64 {
                            let double_casella = Casella::da_indice(double_square).unwrap();
                            
                            // Controlla che entrambe le caselle siano vuote
                            if caselle_vuote.contiene_casella(target_square) && 
                               caselle_vuote.contiene_casella(double_square) {
                                mosse.push(Mossa::nuova(casella_da, double_casella));
                            }
                        }
                    }
                }
            }
        }
        
        // Catture
        let attacchi = BitboardTables::attacchi_pedone(square, colore);
        let catture = attacchi & occupazione_avversario;
        
        let mut catture_temp = catture;
        while let Some(capture_square) = catture_temp.pop_lsb() {
            let target_casella = Casella::da_indice(capture_square).unwrap();
            let target_rank = capture_square / 8;
            
            if (colore == Colore::Bianco && target_rank == 7) || (colore == Colore::Nero && target_rank == 0) {
                // Promozione con cattura
                for pezzo in &[Pezzo::Regina, Pezzo::Torre, Pezzo::Alfiere, Pezzo::Cavallo] {
                    mosse.push(Mossa::nuova_con_promozione(casella_da, target_casella, *pezzo));
                }
            } else {
                mosse.push(Mossa::nuova(casella_da, target_casella));
            }
        }
        
        // En passant
        if let Some(en_passant_sq) = scacchiera.en_passant() {
            let en_passant_bitboard = en_passant_sq.to_bitboard();
            let en_passant_attacks = attacchi & en_passant_bitboard;
            
            if !en_passant_attacks.is_vuota() {
                // En passant non può essere promozione
                mosse.push(Mossa::nuova(casella_da, en_passant_sq));
            }
        }
    }
}

fn genera_mosse_cavallo_bitboard(scacchiera: &Scacchiera, colore: Colore, occupazione_colore: Bitboard, 
                                 mosse: &mut Vec<Mossa>) {
    let cavalli = scacchiera.bitboard_pezzo(Pezzo::Cavallo, colore);
    if cavalli.is_vuota() {
        return;
    }
    
    let mut cavalli_temp = cavalli;
    while let Some(square) = cavalli_temp.pop_lsb() {
        let casella_da = Casella::da_indice(square).unwrap();
        let mosse_possibili = BitboardTables::mosse_cavallo(square);
        let mosse_valide = mosse_possibili & !occupazione_colore;
        
        let mut mosse_temp = mosse_valide;
        while let Some(target_square) = mosse_temp.pop_lsb() {
            let target_casella = Casella::da_indice(target_square).unwrap();
            mosse.push(Mossa::nuova(casella_da, target_casella));
        }
    }
}

fn genera_mosse_alfiere_bitboard(scacchiera: &Scacchiera, colore: Colore, occupazione_totale: Bitboard, 
                                 occupazione_colore: Bitboard, mosse: &mut Vec<Mossa>) {
    let alfieri = scacchiera.bitboard_pezzo(Pezzo::Alfiere, colore);
    
    if alfieri.is_vuota() {
        return;
    }
    
    let mut alfieri_temp = alfieri;
    while let Some(square) = alfieri_temp.pop_lsb() {
        let casella_da = Casella::da_indice(square).unwrap();
        let attacchi = BitboardTables::attacchi_alfiere(square, occupazione_totale);
        let mosse_valide = attacchi & !occupazione_colore;
        
        let mut mosse_temp = mosse_valide;
        while let Some(target_square) = mosse_temp.pop_lsb() {
            let target_casella = Casella::da_indice(target_square).unwrap();
            mosse.push(Mossa::nuova(casella_da, target_casella));
        }
    }
}

fn genera_mosse_torre_bitboard(scacchiera: &Scacchiera, colore: Colore, occupazione_totale: Bitboard, 
                               occupazione_colore: Bitboard, mosse: &mut Vec<Mossa>) {
    let torri = scacchiera.bitboard_pezzo(Pezzo::Torre, colore);
    
    if torri.is_vuota() {
        return;
    }
    
    let mut torri_temp = torri;
    while let Some(square) = torri_temp.pop_lsb() {
        let casella_da = Casella::da_indice(square).unwrap();
        let attacchi = BitboardTables::attacchi_torre(square, occupazione_totale);
        let mosse_valide = attacchi & !occupazione_colore;
        
        let mut mosse_temp = mosse_valide;
        while let Some(target_square) = mosse_temp.pop_lsb() {
            let target_casella = Casella::da_indice(target_square).unwrap();
            mosse.push(Mossa::nuova(casella_da, target_casella));
        }
    }
}

fn genera_mosse_regina_bitboard(scacchiera: &Scacchiera, colore: Colore, occupazione_totale: Bitboard, 
                                occupazione_colore: Bitboard, mosse: &mut Vec<Mossa>) {
    let regine = scacchiera.bitboard_pezzo(Pezzo::Regina, colore);
    
    if regine.is_vuota() {
        return;
    }
    
    let mut regine_temp = regine;
    while let Some(square) = regine_temp.pop_lsb() {
        let casella_da = Casella::da_indice(square).unwrap();
        let attacchi = BitboardTables::attacchi_regina(square, occupazione_totale);
        let mosse_valide = attacchi & !occupazione_colore;
        
        let mut mosse_temp = mosse_valide;
        while let Some(target_square) = mosse_temp.pop_lsb() {
            let target_casella = Casella::da_indice(target_square).unwrap();
            mosse.push(Mossa::nuova(casella_da, target_casella));
        }
    }
}

fn genera_mosse_re_bitboard(scacchiera: &Scacchiera, colore: Colore, occupazione_colore: Bitboard, 
                            mosse: &mut Vec<Mossa>) {
    let re = scacchiera.bitboard_pezzo(Pezzo::Re, colore);
    if re.is_vuota() {
        return;
    }
    
    let square = re.lsb().unwrap();
    let casella_da = Casella::da_indice(square).unwrap();
    let mosse_possibili = BitboardTables::mosse_re(square);
    let mosse_valide = mosse_possibili & !occupazione_colore;
    
    let mut mosse_temp = mosse_valide;
    while let Some(target_square) = mosse_temp.pop_lsb() {
        let target_casella = Casella::da_indice(target_square).unwrap();
        mosse.push(Mossa::nuova(casella_da, target_casella));
    }
    
    genera_arrocco_bitboard(scacchiera, colore, square, mosse);
}

fn genera_arrocco_bitboard(scacchiera: &Scacchiera, colore: Colore, re_square: u8, mosse: &mut Vec<Mossa>) {
    if scacchiera.re_in_scacco(colore) {
        return;
    }
    
    let diritti = scacchiera.diritti_arrocco();
    let rank = re_square / 8;
    let occupazione_totale = scacchiera.occupazione_totale();
    let casella_re = Casella::da_indice(re_square).unwrap();
    
    match colore {
        Colore::Bianco if rank == 0 => {
            if diritti.bianco_lato_re {
                let caselle_libere = Bitboard::da_casella(5) | Bitboard::da_casella(6);
                
                if (occupazione_totale & caselle_libere).is_vuota() &&
                   !scacchiera.casella_attaccata(Casella::da_indice(5).unwrap(), Colore::Nero) &&
                   !scacchiera.casella_attaccata(Casella::da_indice(6).unwrap(), Colore::Nero) {
                    if let Some(target) = Casella::da_indice(6) {
                        mosse.push(Mossa::nuova(casella_re, target));
                    }
                }
            }
            
            if diritti.bianco_lato_regina {
                let caselle_libere = Bitboard::da_casella(1) | Bitboard::da_casella(2) | Bitboard::da_casella(3);
                
                if (occupazione_totale & caselle_libere).is_vuota() &&
                   !scacchiera.casella_attaccata(Casella::da_indice(2).unwrap(), Colore::Nero) &&
                   !scacchiera.casella_attaccata(Casella::da_indice(3).unwrap(), Colore::Nero) {
                    if let Some(target) = Casella::da_indice(2) {
                        mosse.push(Mossa::nuova(casella_re, target));
                    }
                }
            }
        }
        Colore::Nero if rank == 7 => {
            if diritti.nero_lato_re {
                let caselle_libere = Bitboard::da_casella(61) | Bitboard::da_casella(62);
                
                if (occupazione_totale & caselle_libere).is_vuota() &&
                   !scacchiera.casella_attaccata(Casella::da_indice(61).unwrap(), Colore::Bianco) &&
                   !scacchiera.casella_attaccata(Casella::da_indice(62).unwrap(), Colore::Bianco) {
                    if let Some(target) = Casella::da_indice(62) {
                        mosse.push(Mossa::nuova(casella_re, target));
                    }
                }
            }
            
            if diritti.nero_lato_regina {
                let caselle_libere = Bitboard::da_casella(57) | Bitboard::da_casella(58) | Bitboard::da_casella(59);
                
                if (occupazione_totale & caselle_libere).is_vuota() &&
                   !scacchiera.casella_attaccata(Casella::da_indice(58).unwrap(), Colore::Bianco) &&
                   !scacchiera.casella_attaccata(Casella::da_indice(59).unwrap(), Colore::Bianco) {
                    if let Some(target) = Casella::da_indice(58) {
                        mosse.push(Mossa::nuova(casella_re, target));
                    }
                }
            }
        }
        _ => {}
    }
}

// ==================== ESECUZIONE MOSSE CON BITBOARD ====================

pub fn esegui_mossa_completa(scacchiera: &mut Scacchiera, mossa: &Mossa) -> bool {
    let pezzo_partenza = scacchiera.ottieni_pezzo(mossa.da);
    if pezzo_partenza.is_none() {
        return false;
    }
    
    let (pezzo, colore) = pezzo_partenza.unwrap();
    let avversario = colore.opposto();
    
    let diritti_vecchi = scacchiera.diritti_arrocco();
    let en_passant_vecchio = scacchiera.en_passant();
    let contatore_semimosse_vecchio = scacchiera.contatore_semimosse();
    
    let mut nuovi_diritti = diritti_vecchi;
    
    // Aggiorna diritti di arrocco se il re o una torre si muovono
    if pezzo == Pezzo::Re {
        match colore {
            Colore::Bianco => {
                nuovi_diritti.bianco_lato_re = false;
                nuovi_diritti.bianco_lato_regina = false;
            }
            Colore::Nero => {
                nuovi_diritti.nero_lato_re = false;
                nuovi_diritti.nero_lato_regina = false;
            }
        }
    }
    
    if pezzo == Pezzo::Torre {
        match (colore, mossa.da.file(), mossa.da.rank()) {
            (Colore::Bianco, 0, 0) => nuovi_diritti.bianco_lato_regina = false,
            (Colore::Bianco, 7, 0) => nuovi_diritti.bianco_lato_re = false,
            (Colore::Nero, 0, 7) => nuovi_diritti.nero_lato_regina = false,
            (Colore::Nero, 7, 7) => nuovi_diritti.nero_lato_re = false,
            _ => {}
        }
    }
    
    // Gestisci cattura di una torre (che rimuove diritti di arrocco)
    if let Some((pezzo_catturato, colore_catturato)) = scacchiera.ottieni_pezzo(mossa.a) {
        if pezzo_catturato == Pezzo::Torre {
            match (colore_catturato, mossa.a.file(), mossa.a.rank()) {
                (Colore::Bianco, 0, 0) => nuovi_diritti.bianco_lato_regina = false,
                (Colore::Bianco, 7, 0) => nuovi_diritti.bianco_lato_re = false,
                (Colore::Nero, 0, 7) => nuovi_diritti.nero_lato_regina = false,
                (Colore::Nero, 7, 7) => nuovi_diritti.nero_lato_re = false,
                _ => {}
            }
        }
    }
    
    scacchiera.imposta_diritti_arrocco(nuovi_diritti);
    
    // Gestisci en passant
    let mut nuovo_en_passant = None;
    if pezzo == Pezzo::Pedone {
        let distanza = (mossa.da.rank() as i8 - mossa.a.rank() as i8).abs();
        if distanza == 2 {
            let rank_en_passant = (mossa.da.rank() as i8 + mossa.a.rank() as i8) / 2;
            nuovo_en_passant = Casella::nuova(mossa.da.file(), rank_en_passant as u8);
        }
        
        // Cattura en passant
        if let Some(en_passant_sq) = en_passant_vecchio {
            if mossa.a == en_passant_sq && mossa.da.file() != mossa.a.file() {
                let rank_cattura = mossa.da.rank(); // Il pedone catturato è sullo stesso rank di partenza
                if let Some(casella_cattura) = Casella::nuova(mossa.a.file(), rank_cattura) {
                    scacchiera.imposta_pezzo(casella_cattura, None);
                }
            }
        }
    }
    
    scacchiera.imposta_en_passant(nuovo_en_passant);
    
    // Gestisci arrocco
    if pezzo == Pezzo::Re {
        let distanza_file = (mossa.da.file() as i8 - mossa.a.file() as i8).abs();
        if distanza_file == 2 {
            if mossa.a.file() == 6 {
                // Arrocco corto
                let torre_da = Casella::nuova(7, mossa.da.rank()).unwrap();
                let torre_a = Casella::nuova(5, mossa.da.rank()).unwrap();
                if let Some((_, colore_torre)) = scacchiera.ottieni_pezzo(torre_da) {
                    scacchiera.imposta_pezzo(torre_a, Some((Pezzo::Torre, colore_torre)));
                    scacchiera.imposta_pezzo(torre_da, None);
                }
            } else if mossa.a.file() == 2 {
                // Arrocco lungo
                let torre_da = Casella::nuova(0, mossa.da.rank()).unwrap();
                let torre_a = Casella::nuova(3, mossa.da.rank()).unwrap();
                if let Some((_, colore_torre)) = scacchiera.ottieni_pezzo(torre_da) {
                    scacchiera.imposta_pezzo(torre_a, Some((Pezzo::Torre, colore_torre)));
                    scacchiera.imposta_pezzo(torre_da, None);
                }
            }
        }
    }
    
    // Promozione
    if let Some(pezzo_promosso) = mossa.promozione {
        scacchiera.imposta_pezzo(mossa.a, Some((pezzo_promosso, colore)));
    } else {
        scacchiera.imposta_pezzo(mossa.a, Some((pezzo, colore)));
    }
    scacchiera.imposta_pezzo(mossa.da, None);
    
    // Aggiorna contatore semi-mosse per la regola delle 50 mosse
    if pezzo == Pezzo::Pedone || scacchiera.ottieni_pezzo(mossa.a).is_some() {
        scacchiera.imposta_contatore_semimosse(0);
    } else {
        scacchiera.imposta_contatore_semimosse(contatore_semimosse_vecchio + 1);
    }
    
    // Aggiorna numero mossa solo dopo che il nero ha mosso
    if colore == Colore::Nero {
        scacchiera.imposta_numero_mossa(scacchiera.numero_mossa() + 1);
    }
    
    // Cambia il colore attivo
    scacchiera.imposta_colore_attivo(avversario);
    
    // Controlla se il re del giocatore che HA MOSSO è sotto scacco
    // Questo è il controllo CORRETTO: la mossa è illegale se lascia il proprio re sotto scacco
    let re_sotto_scacco = scacchiera.re_in_scacco(colore);
    
    // Restituisci true se la mossa NON lascia il proprio re sotto scacco
    !re_sotto_scacco
}

// ==================== UTILITY FUNCTIONS ====================

pub fn filtra_mosse_legali(scacchiera: &Scacchiera, mosse: &[Mossa]) -> Vec<Mossa> {
    let mut mosse_legali = Vec::new();
    
    for mossa in mosse {
        let mut scacchiera_temp = scacchiera.clone();
        if esegui_mossa_completa(&mut scacchiera_temp, mossa) {
            // Dopo aver eseguito la mossa, dobbiamo controllare se il re del giocatore
            // che HA MOSSO è sotto scacco (questo viene già fatto in esegui_mossa_completa)
            // Quindi se esegui_mossa_completa ha restituito true, la mossa è legale
            mosse_legali.push(*mossa);
        }
    }
    
    mosse_legali
}

pub fn genera_e_filtra_mosse(scacchiera: &Scacchiera) -> Vec<Mossa> {
    let tutte_mosse = genera_mosse(scacchiera);
    filtra_mosse_legali(scacchiera, &tutte_mosse)
}

pub fn debug_generazione_mosse(scacchiera: &Scacchiera) {
    println!("=== DEBUG GENERAZIONE MOSSE ===");
    println!("Colore attivo: {:?}", scacchiera.colore_attivo());
    
    let mosse = genera_mosse(scacchiera);
    println!("Mosse totali generate: {}", mosse.len());
    
    for (i, mossa) in mosse.iter().enumerate() {
        println!("{}. {} da {:?}", i + 1, mossa, mossa.da);
    }
    
    let mosse_legali = filtra_mosse_legali(scacchiera, &mosse);
    println!("Mosse legali: {}", mosse_legali.len());
    for (i, mossa) in mosse_legali.iter().enumerate() {
        println!("{}. {}", i + 1, mossa);
    }
}

// ==================== NUOVE FUNZIONI PER TEST ====================

pub fn test_mossa_specifica(scacchiera: &Scacchiera, mossa_str: &str) -> bool {
    // Funzione per testare una mossa specifica
    println!("=== TEST MOSSA: {} ===", mossa_str);
    
    // Prima generiamo tutte le mosse legali
    let mosse_legali = genera_e_filtra_mosse(scacchiera);
    
    // Converti la stringa in Mossa
    if let Some(mossa) = parse_mossa_da_stringa(scacchiera, mossa_str) {
        println!("Mossa parsata: {}", mossa);
        
        // Controlla se la mossa è nelle mosse legali
        let is_legale = mosse_legali.contains(&mossa);
        println!("La mossa è legale? {}", is_legale);
        
        // Esegui la mossa per vedere cosa succede
        let mut scacchiera_test = scacchiera.clone();
        let esito = esegui_mossa_completa(&mut scacchiera_test, &mossa);
        println!("Esegui_mossa_completa ha restituito: {}", esito);
        
        if esito {
            println!("Dopo la mossa:");
            println!("Re bianco in scacco? {}", scacchiera_test.re_in_scacco(Colore::Bianco));
            println!("Re nero in scacco? {}", scacchiera_test.re_in_scacco(Colore::Nero));
        }
        
        return is_legale;
    } else {
        println!("Impossibile parsare la mossa: {}", mossa_str);
        return false;
    }
}

pub fn parse_mossa_da_stringa(scacchiera: &Scacchiera, mossa_str: &str) -> Option<Mossa> {
    // Semplice parser per mosse in notazione algebrica
    if mossa_str.len() < 4 {
        return None;
    }
    
    let chars: Vec<char> = mossa_str.chars().collect();
    
    // Estrai partenza e arrivo
    let file_da = chars[0] as u8 - b'a';
    let rank_da = chars[1] as u8 - b'1';
    let file_a = chars[2] as u8 - b'a';
    let rank_a = chars[3] as u8 - b'1';
    
    if file_da > 7 || rank_da > 7 || file_a > 7 || rank_a > 7 {
        return None;
    }
    
    let da = Casella::nuova(file_da, rank_da)?;
    let a = Casella::nuova(file_a, rank_a)?;
    
    // Controlla se c'è promozione
    if mossa_str.len() > 4 {
        let promozione_char = chars[4];
        let pezzo_promosso = match promozione_char.to_lowercase().next()? {
            'q' => Pezzo::Regina,
            'r' => Pezzo::Torre,
            'b' => Pezzo::Alfiere,
            'n' => Pezzo::Cavallo,
            _ => return None,
        };
        
        Some(Mossa::nuova_con_promozione(da, a, pezzo_promosso))
    } else {
        Some(Mossa::nuova(da, a))
    }
}

// ==================== FUNZIONI DI UTILITY PER DEBUG ====================

pub fn stampa_scacchiera_debug(scacchiera: &Scacchiera) {
    println!("=== DEBUG SCACCHIERA ===");
    println!("Colore attivo: {:?}", scacchiera.colore_attivo());
    println!("Diritti arrocco: {:?}", scacchiera.diritti_arrocco());
    println!("En passant: {:?}", scacchiera.en_passant());
    println!("Contatore semi-mosse: {}", scacchiera.contatore_semimosse());
    println!("Numero mossa: {}", scacchiera.numero_mossa());
}

pub fn verifica_posizione_fen(fen: &str, mossa_test: &str) {
    match Scacchiera::da_fen(fen) {
        Ok(scacchiera) => {
            println!("=== VERIFICA POSIZIONE FEN ===");
            println!("FEN: {}", fen);
            println!("Scacchiera caricata correttamente");
            
            // Testa la mossa
            test_mossa_specifica(&scacchiera, mossa_test);
            
            // Mostra tutte le mosse legali
            let mosse = genera_e_filtra_mosse(&scacchiera);
            println!("\nTutte le mosse legali ({}):", mosse.len());
            for mossa in mosse {
                println!("  {}", mossa);
            }
        }
        Err(e) => {
            println!("Errore nel caricamento FEN: {}", e);
        }
    }
}