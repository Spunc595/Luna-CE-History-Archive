use crate::board::{Scacchiera, Pezzo, Colore, Casella, Mossa, MoveFlag};
use crate::attacks::{knight_attacks, king_attacks, bishop_attacks, rook_attacks, queen_attacks};

#[derive(Clone, Copy)]
pub struct MossaConInfo {
    pub mossa: Mossa,
    pub score: i32,
}

pub fn genera_tutte_mosse_pseudo_legali(s: &Scacchiera, mosse: &mut Vec<Mossa>) {
    let us = s.turno();
    let them = us.opposto();
    let occ = s.occupazione();
    let amici = s.bitboard_colore(us);
    let nemici = s.bitboard_colore(them);

    // Iteriamo su ogni tipo di pezzo
    for p_idx in 0..6 {
        let pezzo = match p_idx { 
            0=>Pezzo::Pedone, 1=>Pezzo::Cavallo, 2=>Pezzo::Alfiere, 
            3=>Pezzo::Torre, 4=>Pezzo::Regina, _=>Pezzo::Re 
        };
        
        let mut bb = s.bitboard_pezzo_colore(pezzo, us);

        while bb != 0 {
            let from = bb.trailing_zeros() as usize;
            bb &= bb - 1; // Rimuove il bit processato

            if pezzo == Pezzo::Pedone {
                genera_mosse_pedone(s, from, us, occ, nemici, mosse);
                continue;
            }

            // Calcolo attacchi base
            let attacchi = match pezzo {
                Pezzo::Cavallo => knight_attacks(from),
                Pezzo::Alfiere => bishop_attacks(from, occ),
                Pezzo::Torre => rook_attacks(from, occ),
                Pezzo::Regina => queen_attacks(from, occ),
                Pezzo::Re => king_attacks(from),
                _ => 0,
            };

            // FILTRO CRITICO: Rimuovi le case occupate da pezzi amici!
            // Questo previene l'errore "can't capture own piece"
            let mut mosse_legali_bb = attacchi & !amici;

            while mosse_legali_bb != 0 {
                let to = mosse_legali_bb.trailing_zeros() as usize;
                mosse_legali_bb &= mosse_legali_bb - 1;

                let is_capture = (nemici & (1u64 << to)) != 0;
                let flag = if is_capture { MoveFlag::Capture } else { MoveFlag::None };
                
                mosse.push(Mossa::new_with_flag(Casella(from), Casella(to), None, flag));
            }
        }
    }

    genera_arrocco(s, us, occ, mosse);
}

fn genera_mosse_pedone(s: &Scacchiera, from: usize, us: Colore, occ: u64, nemici: u64, mosse: &mut Vec<Mossa>) {
    let (up, rank_start, rank_prom) = if us == Colore::Bianco { (8, 1, 6) } else { (-8isize as usize, 6, 1) };
    let rank = from / 8;
    
    // Spinta singola
    let to_1 = if us == Colore::Bianco { from + 8 } else { from - 8 };
    if (occ & (1u64 << to_1)) == 0 {
        if rank == rank_prom {
            aggiungi_promozioni(from, to_1, false, mosse);
        } else {
            mosse.push(Mossa::new_with_flag(Casella(from), Casella(to_1), None, MoveFlag::None));
            // Spinta doppia
            let to_2 = if us == Colore::Bianco { from + 16 } else { from - 16 };
            if rank == rank_start && (occ & (1u64 << to_2)) == 0 {
                mosse.push(Mossa::new_with_flag(Casella(from), Casella(to_2), None, MoveFlag::DoublePawnPush));
            }
        }
    }

    // Catture
    let captures = crate::attacks::pawn_attacks(from, us) & nemici;
    let mut cap_bb = captures;
    while cap_bb != 0 {
        let to = cap_bb.trailing_zeros() as usize;
        cap_bb &= cap_bb - 1;
        if rank == rank_prom {
            aggiungi_promozioni(from, to, true, mosse);
        } else {
            mosse.push(Mossa::new_with_flag(Casella(from), Casella(to), None, MoveFlag::Capture));
        }
    }

    // En Passant
    if let Some(ep_sq) = s.en_passant {
        let ep_bb = 1u64 << ep_sq;
        // Verifica che il pedone possa attaccare la casa en passant
        if (crate::attacks::pawn_attacks(from, us) & ep_bb) != 0 {
            mosse.push(Mossa::new_with_flag(Casella(from), Casella(ep_sq), None, MoveFlag::EnPassant));
        }
    }
}

fn aggiungi_promozioni(from: usize, to: usize, is_capture: bool, mosse: &mut Vec<Mossa>) {
    let flag = if is_capture { MoveFlag::PromotionCapture } else { MoveFlag::Promotion };
    for p in [Pezzo::Regina, Pezzo::Torre, Pezzo::Alfiere, Pezzo::Cavallo] {
        mosse.push(Mossa::new_with_flag(Casella(from), Casella(to), Some(p), flag));
    }
}

fn genera_arrocco(s: &Scacchiera, us: Colore, occ: u64, mosse: &mut Vec<Mossa>) {
    let rights = s.diritti_arrocco_struct();
    let (kingside, queenside) = if us == Colore::Bianco { (rights.bianco_lato_re, rights.bianco_lato_regina) } 
                                else { (rights.nero_lato_re, rights.nero_lato_regina) };
    
    // Controlliamo se il Re è sotto scacco (non si può arroccare sotto scacco)
    if s.re_in_scacco(us) { return; }

    let (k_idx, k_t_sq, q_t_sq) = if us == Colore::Bianco { (4, 6, 2) } else { (60, 62, 58) };

    // Arrocco Corto (King Side)
    if kingside {
        let f = if us == Colore::Bianco { 5 } else { 61 };
        let g = if us == Colore::Bianco { 6 } else { 62 };
        // 1. Percorso libero
        if (occ & ((1 << f) | (1 << g))) == 0 {
            // 2. Case di transito non attaccate
            if !s.casella_attaccata(Casella(f), us.opposto()) && !s.casella_attaccata(Casella(g), us.opposto()) {
                mosse.push(Mossa::new_with_flag(Casella(k_idx), Casella(k_t_sq), None, MoveFlag::CastleKingSide));
            }
        }
    }

    // Arrocco Lungo (Queen Side)
    if queenside {
        let b = if us == Colore::Bianco { 1 } else { 57 };
        let c = if us == Colore::Bianco { 2 } else { 58 };
        let d = if us == Colore::Bianco { 3 } else { 59 };
        // 1. Percorso libero (incluso b che è solo di passaggio per la torre ma deve essere vuoto)
        if (occ & ((1 << b) | (1 << c) | (1 << d))) == 0 {
            // 2. Case di transito Re non attaccate (il Re passa per c e d)
            if !s.casella_attaccata(Casella(d), us.opposto()) && !s.casella_attaccata(Casella(c), us.opposto()) {
                mosse.push(Mossa::new_with_flag(Casella(k_idx), Casella(q_t_sq), None, MoveFlag::CastleQueenSide));
            }
        }
    }
}

// SEE: Static Exchange Evaluation (Risolto errore tipo)
pub fn see(s: &Scacchiera, m: Mossa) -> i32 {
    let mut val = 0;
    let to = m.a().indice();
    let from = m.da().indice();
    
    // Primo pezzo (vittima)
    let captured = if m.flag() == MoveFlag::EnPassant { 
        Some(Pezzo::Pedone) 
    } else { 
        s.get_piece_at(to) 
    };

    if let Some(p) = captured { val = p.valore(); } else { return 0; }

    // Eseguiamo la prima cattura "virtualmente"
    let mut attacker_val = s.get_piece_at(from).unwrap().valore();
    val -= attacker_val;

    let mut occ = s.occupazione();
    let mut side = s.turno().opposto(); // Dopo la mossa tocca all'avversario reagire
    
    // Rimuoviamo i primi pezzi coinvolti
    occ &= !(1u64 << from); 
    
    // Simulazione scambi successivi
    loop {
        val = -val;
        // Qui c'era l'errore di tipo: "let p: Pezzo = p;" lo risolve
        if let Some((sq_att, p)) = s.get_least_valuable_attacker(to, side, occ) {
            let p: Pezzo = p; // Annotazione di tipo esplicita
            occ &= !(1u64 << sq_att);
            attacker_val = p.valore();
            val -= attacker_val;
            side = side.opposto();
            
            // Pruning: se stiamo perdendo troppo, fermiamoci
            if val >= 0 { break; }
        } else {
            break;
        }
    }
    
    val
}