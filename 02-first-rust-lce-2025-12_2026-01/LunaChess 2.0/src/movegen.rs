use crate::board::{Scacchiera, Pezzo, Colore, Casella};
use std::fmt;

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Mossa {
    pub da: Casella,
    pub a: Casella,
    pub promozione: Option<Pezzo>,
}

pub fn genera_mosse(scacchiera: &Scacchiera) -> Vec<Mossa> {
    let mut mosse = Vec::new();
    let colore = scacchiera.colore_attivo();

    for indice in 0..64 {
        if let Some(casella) = Casella::da_indice(indice) {
            if let Some((pezzo, colore_pezzo)) = scacchiera.ottieni_pezzo(casella) {
                if colore_pezzo == colore {
                    match pezzo {
                        Pezzo::Pedone => genera_mosse_pedone(&mut mosse, scacchiera, casella),
                        Pezzo::Cavallo => genera_mosse_cavallo(&mut mosse, scacchiera, casella),
                        Pezzo::Alfiere => genera_mosse_alfiere(&mut mosse, scacchiera, casella),
                        Pezzo::Torre => genera_mosse_torre(&mut mosse, scacchiera, casella),
                        Pezzo::Regina => {
                            genera_mosse_alfiere(&mut mosse, scacchiera, casella);
                            genera_mosse_torre(&mut mosse, scacchiera, casella);
                        }
                        Pezzo::Re => genera_mosse_re(&mut mosse, scacchiera, casella),
                    }
                }
            }
        }
    }

    // Filtra mosse illegali (che lasciano il re in scacco)
    mosse.retain(|m| {
        let mut nuova_scacchiera = scacchiera.clone();
        if nuova_scacchiera.esegui_mossa(m) {
            !nuova_scacchiera.in_scacco(colore)
        } else {
            false
        }
    });

    mosse
}

fn genera_mosse_pedone(mosse: &mut Vec<Mossa>, scacchiera: &Scacchiera, da: Casella) {
    let colore = scacchiera.colore_attivo();
    let direzione = match colore {
        Colore::Bianco => 1,
        Colore::Nero => -1,
    };
    let numero_partenza = match colore {
        Colore::Bianco => 1,
        Colore::Nero => 6,
    };

    // Una casella in avanti
    if let Some(casella_avanti) = Casella::nuova(da.lettera(), (da.numero() as i8 + direzione) as u8) {
        if scacchiera.ottieni_pezzo(casella_avanti).is_none() {
            // Promozione
            if (colore == Colore::Bianco && da.numero() == 6) || (colore == Colore::Nero && da.numero() == 1) {
                for &pezzo_promosso in &[Pezzo::Cavallo, Pezzo::Alfiere, Pezzo::Torre, Pezzo::Regina] {
                    mosse.push(Mossa { da, a: casella_avanti, promozione: Some(pezzo_promosso) });
                }
            } else {
                mosse.push(Mossa { da, a: casella_avanti, promozione: None });
            }

            // Due caselle in avanti (solo dalla posizione iniziale)
            if da.numero() == numero_partenza {
                if let Some(casella_due_avanti) = Casella::nuova(da.lettera(), (da.numero() as i8 + 2 * direzione) as u8) {
                    if scacchiera.ottieni_pezzo(casella_due_avanti).is_none() {
                        mosse.push(Mossa { da, a: casella_due_avanti, promozione: None });
                    }
                }
            }
        }
    }

    // Catture
    for dx in [-1, 1].iter() {
        let lettera = da.lettera() as i8 + dx;
        let numero = da.numero() as i8 + direzione;
        
        if lettera >= 0 && lettera < 8 && numero >= 0 && numero < 8 {
            if let Some(casella_cattura) = Casella::nuova(lettera as u8, numero as u8) {
                // Cattura normale
                if let Some((_, colore_pezzo)) = scacchiera.ottieni_pezzo(casella_cattura) {
                    if colore_pezzo != colore {
                        // Promozione con cattura
                        if (colore == Colore::Bianco && da.numero() == 6) || (colore == Colore::Nero && da.numero() == 1) {
                            for &pezzo_promosso in &[Pezzo::Cavallo, Pezzo::Alfiere, Pezzo::Torre, Pezzo::Regina] {
                                mosse.push(Mossa { da, a: casella_cattura, promozione: Some(pezzo_promosso) });
                            }
                        } else {
                            mosse.push(Mossa { da, a: casella_cattura, promozione: None });
                        }
                    }
                }
                // En passant
                else if let Some(casella_en_passant) = scacchiera.en_passant() {
                    if casella_cattura == casella_en_passant {
                        mosse.push(Mossa { da, a: casella_cattura, promozione: None });
                    }
                }
            }
        }
    }
}

fn genera_mosse_cavallo(mosse: &mut Vec<Mossa>, scacchiera: &Scacchiera, da: Casella) {
    let mosse_cavallo = [
        (-2, -1), (-2, 1), (-1, -2), (-1, 2),
        (1, -2), (1, 2), (2, -1), (2, 1),
    ];
    
    for &(dx, dy) in &mosse_cavallo {
        let lettera = da.lettera() as i8 + dx;
        let numero = da.numero() as i8 + dy;
        
        if lettera >= 0 && lettera < 8 && numero >= 0 && numero < 8 {
            if let Some(casella_destinazione) = Casella::nuova(lettera as u8, numero as u8) {
                if puoi_muovere_a(scacchiera, da, casella_destinazione) {
                    mosse.push(Mossa { da, a: casella_destinazione, promozione: None });
                }
            }
        }
    }
}

fn genera_mosse_alfiere(mosse: &mut Vec<Mossa>, scacchiera: &Scacchiera, da: Casella) {
    let direzioni = [(1, 1), (1, -1), (-1, 1), (-1, -1)];
    
    for &(dx, dy) in &direzioni {
        let mut lettera = da.lettera() as i8 + dx;
        let mut numero = da.numero() as i8 + dy;
        
        while lettera >= 0 && lettera < 8 && numero >= 0 && numero < 8 {
            if let Some(casella_destinazione) = Casella::nuova(lettera as u8, numero as u8) {
                if !puoi_muovere_a(scacchiera, da, casella_destinazione) {
                    break;
                }
                
                mosse.push(Mossa { da, a: casella_destinazione, promozione: None });
                
                // Se abbiamo catturato un pezzo, non possiamo andare oltre
                if scacchiera.ottieni_pezzo(casella_destinazione).is_some() {
                    break;
                }
                
                lettera += dx;
                numero += dy;
            } else {
                break;
            }
        }
    }
}

fn genera_mosse_torre(mosse: &mut Vec<Mossa>, scacchiera: &Scacchiera, da: Casella) {
    let direzioni = [(1, 0), (-1, 0), (0, 1), (0, -1)];
    
    for &(dx, dy) in &direzioni {
        let mut lettera = da.lettera() as i8 + dx;
        let mut numero = da.numero() as i8 + dy;
        
        while lettera >= 0 && lettera < 8 && numero >= 0 && numero < 8 {
            if let Some(casella_destinazione) = Casella::nuova(lettera as u8, numero as u8) {
                if !puoi_muovere_a(scacchiera, da, casella_destinazione) {
                    break;
                }
                
                mosse.push(Mossa { da, a: casella_destinazione, promozione: None });
                
                // Se abbiamo catturato un pezzo, non possiamo andare oltre
                if scacchiera.ottieni_pezzo(casella_destinazione).is_some() {
                    break;
                }
                
                lettera += dx;
                numero += dy;
            } else {
                break;
            }
        }
    }
}

fn genera_mosse_re(mosse: &mut Vec<Mossa>, scacchiera: &Scacchiera, da: Casella) {
    let mosse_re = [
        (-1, -1), (-1, 0), (-1, 1),
        (0, -1),           (0, 1),
        (1, -1),  (1, 0), (1, 1),
    ];
    
    for &(dx, dy) in &mosse_re {
        let lettera = da.lettera() as i8 + dx;
        let numero = da.numero() as i8 + dy;
        
        if lettera >= 0 && lettera < 8 && numero >= 0 && numero < 8 {
            if let Some(casella_destinazione) = Casella::nuova(lettera as u8, numero as u8) {
                if puoi_muovere_a(scacchiera, da, casella_destinazione) {
                    mosse.push(Mossa { da, a: casella_destinazione, promozione: None });
                }
            }
        }
    }
    
    // Arrocco
    genera_arrocco(mosse, scacchiera, da);
}

fn genera_arrocco(mosse: &mut Vec<Mossa>, scacchiera: &Scacchiera, da: Casella) {
    let colore = scacchiera.colore_attivo();
    let diritti = scacchiera.diritti_arrocco();
    
    // Il re non deve essere in scacco
    if scacchiera.in_scacco(colore) {
        return;
    }
    
    // Arrocco corto (lato re)
    if (colore == Colore::Bianco && diritti.bianco_lato_re) || 
       (colore == Colore::Nero && diritti.nero_lato_re) {
        
        let lettera_re = match colore {
            Colore::Bianco => 4,
            Colore::Nero => 4,
        };
        
        // Controlla che le caselle tra re e torre siano vuote e non sotto attacco
        let caselle_intermedie = match colore {
            Colore::Bianco => [5, 6],  // f1, g1
            Colore::Nero => [5, 6],    // f8, g8
        };
        
        let mut arrocco_valido = true;
        for &indice in &caselle_intermedie {
            if let Some(casella) = Casella::nuova(indice, da.numero()) {
                if scacchiera.ottieni_pezzo(casella).is_some() {
                    arrocco_valido = false;
                    break;
                }
                // Controlla che le caselle che il re attraversa non siano sotto attacco
                if indice == 5 || indice == 6 {
                    if scacchiera.casella_attaccata(casella, colore.opposto()) {
                        arrocco_valido = false;
                        break;
                    }
                }
            }
        }
        
        if arrocco_valido {
            let lettera_destinazione = match colore {
                Colore::Bianco => 6, // g1
                Colore::Nero => 6,   // g8
            };
            if let Some(casella_destinazione) = Casella::nuova(lettera_destinazione, da.numero()) {
                mosse.push(Mossa { da, a: casella_destinazione, promozione: None });
            }
        }
    }
    
    // Arrocco lungo (lato regina)
    if (colore == Colore::Bianco && diritti.bianco_lato_regina) || 
       (colore == Colore::Nero && diritti.nero_lato_regina) {
        
        let caselle_intermedie = match colore {
            Colore::Bianco => [1, 2, 3],  // b1, c1, d1
            Colore::Nero => [1, 2, 3],    // b8, c8, d8
        };
        
        let mut arrocco_valido = true;
        for &indice in &caselle_intermedie {
            if let Some(casella) = Casella::nuova(indice, da.numero()) {
                if scacchiera.ottieni_pezzo(casella).is_some() {
                    arrocco_valido = false;
                    break;
                }
                // Controlla che le caselle che il re attraversa non siano sotto attacco
                if indice == 2 || indice == 3 {
                    if scacchiera.casella_attaccata(casella, colore.opposto()) {
                        arrocco_valido = false;
                        break;
                    }
                }
            }
        }
        
        if arrocco_valido {
            let lettera_destinazione = match colore {
                Colore::Bianco => 2, // c1
                Colore::Nero => 2,   // c8
            };
            if let Some(casella_destinazione) = Casella::nuova(lettera_destinazione, da.numero()) {
                mosse.push(Mossa { da, a: casella_destinazione, promozione: None });
            }
        }
    }
}

fn puoi_muovere_a(scacchiera: &Scacchiera, da: Casella, a: Casella) -> bool {
    if let Some((_, colore_partenza)) = scacchiera.ottieni_pezzo(da) {
        if let Some((_, colore_destinazione)) = scacchiera.ottieni_pezzo(a) {
            // Non puoi catturare un pezzo dello stesso colore
            colore_partenza != colore_destinazione
        } else {
            // Casella di destinazione vuota
            true
        }
    } else {
        false
    }
}

impl fmt::Display for Mossa {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}", self.da, self.a)?;
        if let Some(pezzo) = self.promozione {
            let simbolo = match pezzo {
                Pezzo::Cavallo => 'n',
                Pezzo::Alfiere => 'b',
                Pezzo::Torre => 'r',
                Pezzo::Regina => 'q',
                _ => ' ',
            };
            if simbolo != ' ' {
                write!(f, "{}", simbolo)?;
            }
        }
        Ok(())
    }
}