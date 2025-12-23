use crate::board::{Scacchiera, Pezzo, Colore, Casella, Tables};
use std::fmt;

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Mossa {
    pub da: Casella,
    pub a: Casella,
    pub promozione: Option<Pezzo>,
}

impl Mossa {
    pub fn nuova(da: Casella, a: Casella) -> Self {
        Mossa {
            da,
            a,
            promozione: None,
        }
    }
    
    pub fn nuova_con_promozione(da: Casella, a: Casella, pezzo: Pezzo) -> Self {
        Mossa {
            da,
            a,
            promozione: Some(pezzo),
        }
    }
    
    pub fn to_string(&self) -> String {
        let mut result = format!("{}{}", self.da.to_string(), self.a.to_string());
        if let Some(promo) = self.promozione {
            let c = match promo {
                Pezzo::Regina => 'q',
                Pezzo::Torre => 'r',
                Pezzo::Alfiere => 'b',
                Pezzo::Cavallo => 'n',
                _ => ' ',
            };
            if c != ' ' {
                result.push(c);
            }
        }
        result
    }
    
    pub fn to_uci_string(&self) -> String {
        self.to_string()
    }
}

impl fmt::Display for Mossa {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

pub fn genera_e_filtra_mosse(scacchiera: &Scacchiera) -> Vec<Mossa> {
    let mut mosse = Vec::new();
    let colore = scacchiera.colore_attivo();
    let avversario = colore.opposto();
    
    // Le variabili vengono usate nelle funzioni chiamate
    let _occupazione_totale = scacchiera.occupazione_totale();
    let _occupazione_colore = scacchiera.bitboard_colore(colore);
    let _occupazione_avversario = scacchiera.bitboard_colore(avversario);
    
    // Genera mosse per ogni tipo di pezzo
    genera_mosse_pedone(scacchiera, colore, &mut mosse);
    genera_mosse_cavallo(scacchiera, colore, &mut mosse);
    genera_mosse_alfiere(scacchiera, colore, &mut mosse);
    genera_mosse_torre(scacchiera, colore, &mut mosse);
    genera_mosse_regina(scacchiera, colore, &mut mosse);
    genera_mosse_re(scacchiera, colore, &mut mosse);
    
    // Filtra mosse che lasciano il re sotto scacco
    mosse.into_iter()
        .filter(|m| {
            let mut scacchiera_test = scacchiera.clone();
            esegui_mossa_completa(&mut scacchiera_test, m)
        })
        .collect()
}

fn genera_mosse_pedone(scacchiera: &Scacchiera, colore: Colore, mosse: &mut Vec<Mossa>) {
    let pedoni = scacchiera.bitboard_pezzo(Pezzo::Pedone, colore);
    if pedoni == 0 {
        return;
    }
    
    let direzione = colore.direzione_pedone();
    let occupazione_totale = scacchiera.occupazione_totale();
    let occupazione_avversario = scacchiera.bitboard_colore(colore.opposto());
    
    let mut pedoni_bb = pedoni;
    while pedoni_bb != 0 {
        let square = pedoni_bb.trailing_zeros() as usize;
        pedoni_bb &= pedoni_bb - 1; // Rimuovi il bit
        
        let casella_da = Casella::da_indice(square).unwrap();
        
        // Mossa in avanti di una casella
        let target_square = (square as i32 + direzione * 8) as usize;
        if target_square < 64 {
            let target_casella = Casella::da_indice(target_square).unwrap();
            let target_bit = 1u64 << target_square;
            
            if (occupazione_totale & target_bit) == 0 {
                // Promozione
                if (colore == Colore::Bianco && target_square >= 56) || 
                   (colore == Colore::Nero && target_square < 8) {
                    for pezzo in &[Pezzo::Regina, Pezzo::Torre, Pezzo::Alfiere, Pezzo::Cavallo] {
                        mosse.push(Mossa::nuova_con_promozione(casella_da, target_casella, *pezzo));
                    }
                } else {
                    mosse.push(Mossa::nuova(casella_da, target_casella));
                    
                    // Doppio passo iniziale
                    let start_rank = if colore == Colore::Bianco { 1 } else { 6 };
                    if (square / 8) == start_rank {
                        let double_square = (square as i32 + direzione * 16) as usize;
                        let double_bit = 1u64 << double_square;
                        if (occupazione_totale & double_bit) == 0 {
                            let double_casella = Casella::da_indice(double_square).unwrap();
                            mosse.push(Mossa::nuova(casella_da, double_casella));
                        }
                    }
                }
            }
        }
        
        // Catture
        let attacchi_pedone = Tables::globale().attacchi_pedone[colore.indice()][square];
        let catture_possibili = attacchi_pedone & occupazione_avversario;
        
        let mut catture_bb = catture_possibili;
        while catture_bb != 0 {
            let capture_square = catture_bb.trailing_zeros() as usize;
            catture_bb &= catture_bb - 1;
            
            let target_casella = Casella::da_indice(capture_square).unwrap();
            
            // Promozione su cattura
            if (colore == Colore::Bianco && capture_square >= 56) || 
               (colore == Colore::Nero && capture_square < 8) {
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
            let en_passant_attacks = attacchi_pedone & en_passant_bitboard;
            
            if en_passant_attacks != 0 {
                mosse.push(Mossa::nuova(casella_da, en_passant_sq));
            }
        }
    }
}

fn genera_mosse_cavallo(scacchiera: &Scacchiera, colore: Colore, mosse: &mut Vec<Mossa>) {
    let cavalli = scacchiera.bitboard_pezzo(Pezzo::Cavallo, colore);
    if cavalli == 0 {
        return;
    }
    
    let occupazione_colore = scacchiera.bitboard_colore(colore);
    
    let mut cavalli_bb = cavalli;
    while cavalli_bb != 0 {
        let square = cavalli_bb.trailing_zeros() as usize;
        cavalli_bb &= cavalli_bb - 1;
        
        let casella_da = Casella::da_indice(square).unwrap();
        let attacchi = Tables::globale().attacchi_cavallo[square];
        let mosse_legali = attacchi & !occupazione_colore;
        
        let mut target_bb = mosse_legali;
        while target_bb != 0 {
            let target_square = target_bb.trailing_zeros() as usize;
            target_bb &= target_bb - 1;
            
            let target_casella = Casella::da_indice(target_square).unwrap();
            mosse.push(Mossa::nuova(casella_da, target_casella));
        }
    }
}

fn genera_mosse_alfiere(scacchiera: &Scacchiera, colore: Colore, mosse: &mut Vec<Mossa>) {
    let alfieri = scacchiera.bitboard_pezzo(Pezzo::Alfiere, colore);
    if alfieri == 0 {
        return;
    }
    
    let occupazione_totale = scacchiera.occupazione_totale();
    let occupazione_colore = scacchiera.bitboard_colore(colore);
    
    let mut alfieri_bb = alfieri;
    while alfieri_bb != 0 {
        let square = alfieri_bb.trailing_zeros() as usize;
        alfieri_bb &= alfieri_bb - 1;
        
        let casella_da = Casella::da_indice(square).unwrap();
        let attacchi = Tables::globale().attacchi_alfiere(square, occupazione_totale);
        let mosse_legali = attacchi & !occupazione_colore;
        
        let mut target_bb = mosse_legali;
        while target_bb != 0 {
            let target_square = target_bb.trailing_zeros() as usize;
            target_bb &= target_bb - 1;
            
            let target_casella = Casella::da_indice(target_square).unwrap();
            mosse.push(Mossa::nuova(casella_da, target_casella));
        }
    }
}

fn genera_mosse_torre(scacchiera: &Scacchiera, colore: Colore, mosse: &mut Vec<Mossa>) {
    let torri = scacchiera.bitboard_pezzo(Pezzo::Torre, colore);
    if torri == 0 {
        return;
    }
    
    let occupazione_totale = scacchiera.occupazione_totale();
    let occupazione_colore = scacchiera.bitboard_colore(colore);
    
    let mut torri_bb = torri;
    while torri_bb != 0 {
        let square = torri_bb.trailing_zeros() as usize;
        torri_bb &= torri_bb - 1;
        
        let casella_da = Casella::da_indice(square).unwrap();
        let attacchi = Tables::globale().attacchi_torre(square, occupazione_totale);
        let mosse_legali = attacchi & !occupazione_colore;
        
        let mut target_bb = mosse_legali;
        while target_bb != 0 {
            let target_square = target_bb.trailing_zeros() as usize;
            target_bb &= target_bb - 1;
            
            let target_casella = Casella::da_indice(target_square).unwrap();
            mosse.push(Mossa::nuova(casella_da, target_casella));
        }
    }
}

fn genera_mosse_regina(scacchiera: &Scacchiera, colore: Colore, mosse: &mut Vec<Mossa>) {
    let regine = scacchiera.bitboard_pezzo(Pezzo::Regina, colore);
    if regine == 0 {
        return;
    }
    
    let occupazione_totale = scacchiera.occupazione_totale();
    let occupazione_colore = scacchiera.bitboard_colore(colore);
    
    let mut regine_bb = regine;
    while regine_bb != 0 {
        let square = regine_bb.trailing_zeros() as usize;
        regine_bb &= regine_bb - 1;
        
        let casella_da = Casella::da_indice(square).unwrap();
        
        // La regina si muove come alfiere + torre
        let attacchi_alfiere = Tables::globale().attacchi_alfiere(square, occupazione_totale);
        let attacchi_torre = Tables::globale().attacchi_torre(square, occupazione_totale);
        let attacchi = attacchi_alfiere | attacchi_torre;
        
        let mosse_legali = attacchi & !occupazione_colore;
        
        let mut target_bb = mosse_legali;
        while target_bb != 0 {
            let target_square = target_bb.trailing_zeros() as usize;
            target_bb &= target_bb - 1;
            
            let target_casella = Casella::da_indice(target_square).unwrap();
            mosse.push(Mossa::nuova(casella_da, target_casella));
        }
    }
}

fn genera_mosse_re(scacchiera: &Scacchiera, colore: Colore, mosse: &mut Vec<Mossa>) {
    let re = scacchiera.bitboard_pezzo(Pezzo::Re, colore);
    if re == 0 {
        return;
    }
    
    let square = re.trailing_zeros() as usize;
    let casella_da = Casella::da_indice(square).unwrap();
    let occupazione_colore = scacchiera.bitboard_colore(colore);
    
    // Mosse normali del re
    let attacchi = Tables::globale().attacchi_re[square];
    let mosse_legali = attacchi & !occupazione_colore;
    
    let mut target_bb = mosse_legali;
    while target_bb != 0 {
        let target_square = target_bb.trailing_zeros() as usize;
        target_bb &= target_bb - 1;
        
        let target_casella = Casella::da_indice(target_square).unwrap();
        mosse.push(Mossa::nuova(casella_da, target_casella));
    }
    
    // Arrocco
    if !scacchiera.re_in_scacco(colore) {
        let diritti = scacchiera.diritti_arrocco();
        let occupazione_totale = scacchiera.occupazione_totale();
        
        if colore == Colore::Bianco {
            // Arrocco corto
            if diritti.bianco_lato_re &&
               (occupazione_totale & (1u64 << 5)) == 0 &&
               (occupazione_totale & (1u64 << 6)) == 0 &&
               !scacchiera.casella_attaccata(Casella::da_indice(5).unwrap(), Colore::Nero) &&
               !scacchiera.casella_attaccata(Casella::da_indice(6).unwrap(), Colore::Nero) {
                if let Some(target) = Casella::da_indice(6) {
                    mosse.push(Mossa::nuova(casella_da, target));
                }
            }
            
            // Arrocco lungo
            if diritti.bianco_lato_regina &&
               (occupazione_totale & (1u64 << 1)) == 0 &&
               (occupazione_totale & (1u64 << 2)) == 0 &&
               (occupazione_totale & (1u64 << 3)) == 0 &&
               !scacchiera.casella_attaccata(Casella::da_indice(2).unwrap(), Colore::Nero) &&
               !scacchiera.casella_attaccata(Casella::da_indice(3).unwrap(), Colore::Nero) {
                if let Some(target) = Casella::da_indice(2) {
                    mosse.push(Mossa::nuova(casella_da, target));
                }
            }
        } else {
            // Arrocco nero corto (caselle 61, 62)
            if diritti.nero_lato_re &&
               (occupazione_totale & (1u64 << 61)) == 0 &&
               (occupazione_totale & (1u64 << 62)) == 0 &&
               !scacchiera.casella_attaccata(Casella::da_indice(61).unwrap(), Colore::Bianco) &&
               !scacchiera.casella_attaccata(Casella::da_indice(62).unwrap(), Colore::Bianco) {
                if let Some(target) = Casella::da_indice(62) {
                    mosse.push(Mossa::nuova(casella_da, target));
                }
            }
            
            // Arrocco nero lungo (caselle 57, 58, 59)
            if diritti.nero_lato_regina &&
               (occupazione_totale & (1u64 << 57)) == 0 &&
               (occupazione_totale & (1u64 << 58)) == 0 &&
               (occupazione_totale & (1u64 << 59)) == 0 &&
               !scacchiera.casella_attaccata(Casella::da_indice(58).unwrap(), Colore::Bianco) &&
               !scacchiera.casella_attaccata(Casella::da_indice(59).unwrap(), Colore::Bianco) {
                if let Some(target) = Casella::da_indice(58) {
                    mosse.push(Mossa::nuova(casella_da, target));
                }
            }
        }
    }
}

pub fn esegui_mossa_completa(scacchiera: &mut Scacchiera, mossa: &Mossa) -> bool {
    // Salva lo stato per il rollback se necessario
    let stato_precedente = scacchiera.clone();
    
    // Ottieni informazioni sulla mossa
    let colore = scacchiera.colore_attivo();
    let avversario = colore.opposto();
    
    // Controlla se c'è un pezzo nella casella di partenza
    let (pezzo, colore_pezzo) = match scacchiera.ottieni_pezzo(mossa.da) {
        Some(info) => info,
        None => return false,
    };
    
    if colore_pezzo != colore {
        return false;
    }
    
    // Salva vecchi valori
    let contatore_semimosse_vecchio = scacchiera.contatore_semimosse();
    let diritti_vecchi = scacchiera.diritti_arrocco();
    let en_passant_vecchio = scacchiera.en_passant();
    
    // Gestisci cattura
    let pezzo_catturato = scacchiera.ottieni_pezzo(mossa.a);
    let e_cattura = pezzo_catturato.is_some();
    
    // Aggiorna contatore semimosse
    if pezzo == Pezzo::Pedone || e_cattura {
        scacchiera.imposta_contatore_semimosse(0);
    } else {
        scacchiera.imposta_contatore_semimosse(contatore_semimosse_vecchio + 1);
    }
    
    // Rimuovi pezzo catturato
    if let Some((_pezzo_catturato, _colore_catturato)) = pezzo_catturato {
        scacchiera.rimuovi_pezzo(mossa.a);
    }
    
    // Muovi il pezzo
    scacchiera.muovi_pezzo(mossa.da, mossa.a);
    
    // Gestisci promozione
    if let Some(pezzo_promosso) = mossa.promozione {
        scacchiera.imposta_pezzo(mossa.a, Some((pezzo_promosso, colore)));
    }
    
    // Inizializza nuovo en passant
    let mut nuovo_en_passant: Option<Casella> = None;
    
    // Gestisci en passant
    if pezzo == Pezzo::Pedone {
        // Controlla se è una cattura en passant
        if let Some(en_passant_sq) = en_passant_vecchio {
            if mossa.a == en_passant_sq && mossa.da.file != mossa.a.file {
                // Cattura en passant
                let rank_cattura = if colore == Colore::Bianco { 
                    mossa.a.rank - 1 
                } else { 
                    mossa.a.rank + 1 
                };
                if let Some(casella_cattura) = Casella::nuova(mossa.a.file, rank_cattura) {
                    scacchiera.rimuovi_pezzo(casella_cattura);
                }
            }
        }
        
        // Imposta nuovo en passant per doppio passo
        let distanza = (mossa.da.rank as i32 - mossa.a.rank as i32).abs();
        if distanza == 2 {
            let rank_en_passant = (mossa.da.rank as i32 + mossa.a.rank as i32) / 2;
            nuovo_en_passant = Casella::nuova(mossa.da.file, rank_en_passant as u8);
        }
    }
    
    scacchiera.imposta_en_passant(nuovo_en_passant);
    
    // Gestisci arrocco
    if pezzo == Pezzo::Re {
        let distanza_file = (mossa.da.file as i32 - mossa.a.file as i32).abs();
        if distanza_file == 2 {
            // Arrocco: muovi anche la torre
            if mossa.a.file == 6 { // Arrocco corto
                if let Some(torre_da) = Casella::nuova(7, mossa.da.rank) {
                    if let Some(torre_a) = Casella::nuova(5, mossa.da.rank) {
                        scacchiera.muovi_pezzo(torre_da, torre_a);
                    }
                }
            } else if mossa.a.file == 2 { // Arrocco lungo
                if let Some(torre_da) = Casella::nuova(0, mossa.da.rank) {
                    if let Some(torre_a) = Casella::nuova(3, mossa.da.rank) {
                        scacchiera.muovi_pezzo(torre_da, torre_a);
                    }
                }
            }
        }
    }
    
    // Aggiorna diritti di arrocco
    let mut nuovi_diritti = diritti_vecchi;
    
    // Se il re si muove, perde tutti i diritti
    if pezzo == Pezzo::Re {
        if colore == Colore::Bianco {
            nuovi_diritti.bianco_lato_re = false;
            nuovi_diritti.bianco_lato_regina = false;
        } else {
            nuovi_diritti.nero_lato_re = false;
            nuovi_diritti.nero_lato_regina = false;
        }
    }
    
    // Se una torre si muove dalla sua casella iniziale, perde il diritto corrispondente
    if pezzo == Pezzo::Torre {
        if mossa.da.file == 0 { // Torre di regina
            if colore == Colore::Bianco && mossa.da.rank == 0 {
                nuovi_diritti.bianco_lato_regina = false;
            } else if colore == Colore::Nero && mossa.da.rank == 7 {
                nuovi_diritti.nero_lato_regina = false;
            }
        } else if mossa.da.file == 7 { // Torre di re
            if colore == Colore::Bianco && mossa.da.rank == 0 {
                nuovi_diritti.bianco_lato_re = false;
            } else if colore == Colore::Nero && mossa.da.rank == 7 {
                nuovi_diritti.nero_lato_re = false;
            }
        }
    }
    
    // Se una torre viene catturata, rimuovi i diritti corrispondenti
    if let Some((Pezzo::Torre, colore_catturato)) = pezzo_catturato {
        if mossa.a.file == 0 { // Torre di regina catturata
            if colore_catturato == Colore::Bianco && mossa.a.rank == 0 {
                nuovi_diritti.bianco_lato_regina = false;
            } else if colore_catturato == Colore::Nero && mossa.a.rank == 7 {
                nuovi_diritti.nero_lato_regina = false;
            }
        } else if mossa.a.file == 7 { // Torre di re catturata
            if colore_catturato == Colore::Bianco && mossa.a.rank == 0 {
                nuovi_diritti.bianco_lato_re = false;
            } else if colore_catturato == Colore::Nero && mossa.a.rank == 7 {
                nuovi_diritti.nero_lato_re = false;
            }
        }
    }
    
    scacchiera.imposta_diritti_arrocco(nuovi_diritti);
    
    // Aggiorna numero mossa se il nero ha mosso
    if colore == Colore::Nero {
        scacchiera.imposta_numero_mossa(scacchiera.numero_mossa() + 1);
    }
    
    // Cambia colore attivo
    scacchiera.imposta_colore_attivo(avversario);
    
    // Incrementa ply count (per supporto endgame)
    scacchiera.incrementa_ply();
    
    // Controlla se la mossa lascia il re sotto scacco
    if scacchiera.re_in_scacco(colore) {
        // Ripristina lo stato
        *scacchiera = stato_precedente;
        return false;
    }
    
    true
}

pub fn esegui_mossa(scacchiera: &mut Scacchiera, mossa: &Mossa) -> bool {
    esegui_mossa_completa(scacchiera, mossa)
}

pub fn parse_mossa_da_stringa(_scacchiera: &Scacchiera, mossa_str: &str) -> Option<Mossa> {
    if mossa_str.len() < 4 {
        return None;
    }
    
    let chars: Vec<char> = mossa_str.chars().collect();
    
    let from_str = format!("{}{}", chars[0], chars[1]);
    let to_str = format!("{}{}", chars[2], chars[3]);
    
    let from = Casella::da_string(&from_str)?;
    let to = Casella::da_string(&to_str)?;
    
    if mossa_str.len() > 4 {
        let pezzo_promosso = match chars[4] {
            'q' => Pezzo::Regina,
            'r' => Pezzo::Torre,
            'b' => Pezzo::Alfiere,
            'n' => Pezzo::Cavallo,
            _ => return None,
        };
        Some(Mossa::nuova_con_promozione(from, to, pezzo_promosso))
    } else {
        Some(Mossa::nuova(from, to))
    }
}