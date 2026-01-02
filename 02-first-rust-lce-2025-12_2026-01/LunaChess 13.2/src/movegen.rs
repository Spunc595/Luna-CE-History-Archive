use crate::board::{Scacchiera, Pezzo, Colore, Mossa, MoveFlag, Bitboard};

pub fn genera_tutte_mosse_pseudo_legali(s: &Scacchiera, mosse: &mut Vec<Mossa>) {
    let us = s.turno;
    let amici = s.colori[us.indice()];
    let nemici = s.colori[us.opposto().indice()];
    let occ = s.occupazione();

    // 1. GENERAZIONE PEDONI
    let mut pedoni_nostri = s.pezzi[Pezzo::Pedone.indice()] & amici;
    while pedoni_nostri != 0 {
        let from = pedoni_nostri.trailing_zeros() as usize;
        pedoni_nostri &= pedoni_nostri - 1;
        genera_mosse_pedone(from, us, occ, nemici, mosse);
    }

    // 2. PEZZI (Cavallo, Alfiere, Torre, Regina, Re)
    for p_idx in 1..6 {
        let pezzo: Pezzo = unsafe { std::mem::transmute(p_idx as u8) };
        let mut bb = s.pezzi[p_idx] & amici;
        while bb != 0 {
            let from = bb.trailing_zeros() as usize;
            bb &= bb - 1;
            
            // Filtro: mosse possibili meno le case occupate da pezzi amici
            let mut targets = s.get_attacks_for_piece(pezzo, from, occ) & !amici;
            while targets != 0 {
                let to = targets.trailing_zeros() as usize;
                targets &= targets - 1;
                
                let is_cap = (nemici & (1u64 << to)) != 0;
                mosse.push(Mossa::new(from, to, if is_cap { MoveFlag::Capture } else { MoveFlag::None }));
            }
        }
    }

    // 3. ARROCCO (Logica di sicurezza)
    if us == Colore::Bianco {
        // Corto (O-O)
        if s.diritti_arrocco & 1 != 0 && (occ & 0x60) == 0 {
            if !s.casella_attaccata(4, Colore::Nero) && !s.casella_attaccata(5, Colore::Nero) {
                mosse.push(Mossa::new(4, 6, MoveFlag::Castle));
            }
        }
        // Lungo (O-O-O)
        if s.diritti_arrocco & 2 != 0 && (occ & 0xE) == 0 {
            if !s.casella_attaccata(4, Colore::Nero) && !s.casella_attaccata(3, Colore::Nero) {
                mosse.push(Mossa::new(4, 2, MoveFlag::Castle));
            }
        }
    } else {
        // Nero Corto (O-O)
        if s.diritti_arrocco & 4 != 0 && (occ & 0x6000000000000000) == 0 {
            if !s.casella_attaccata(60, Colore::Bianco) && !s.casella_attaccata(61, Colore::Bianco) {
                mosse.push(Mossa::new(60, 62, MoveFlag::Castle));
            }
        }
        // Nero Lungo (O-O-O)
        if s.diritti_arrocco & 8 != 0 && (occ & 0xE00000000000000) == 0 {
            if !s.casella_attaccata(60, Colore::Bianco) && !s.casella_attaccata(59, Colore::Bianco) {
                mosse.push(Mossa::new(60, 58, MoveFlag::Castle));
            }
        }
    }
}

fn genera_mosse_pedone(from: usize, us: Colore, occ: Bitboard, nemici: Bitboard, mosse: &mut Vec<Mossa>) {
    let bit = 1u64 << from;
    let rank = from / 8;

    if us == Colore::Bianco {
        let to = from + 8;
        if to < 64 && (occ & (1u64 << to)) == 0 {
            let flag = if to / 8 == 7 { MoveFlag::Promotion } else { MoveFlag::None };
            mosse.push(Mossa::new(from, to, flag));
            // Doppia spinta da riga 2
            if rank == 1 && (occ & (1u64 << (from + 16))) == 0 {
                mosse.push(Mossa::new(from, from + 16, MoveFlag::DoublePawnPush));
            }
        }
        // Catture Bianco
        let mut caps = crate::attacks::pawn_attacks(from, Colore::Bianco) & nemici;
        while caps != 0 {
            let to = caps.trailing_zeros() as usize;
            caps &= caps - 1;
            let flag = if to / 8 == 7 { MoveFlag::Promotion } else { MoveFlag::Capture };
            mosse.push(Mossa::new(from, to, flag));
        }
    } else {
        let to = from as i8 - 8;
        if to >= 0 {
            let to_idx = to as usize;
            if (occ & (1u64 << to_idx)) == 0 {
                let flag = if to_idx / 8 == 0 { MoveFlag::Promotion } else { MoveFlag::None };
                mosse.push(Mossa::new(from, to_idx, flag));
                // Doppia spinta da riga 7
                if rank == 6 && (occ & (1u64 << (from - 16))) == 0 {
                    mosse.push(Mossa::new(from, from - 16, MoveFlag::DoublePawnPush));
                }
            }
        }
        // Catture Nero
        let mut caps = crate::attacks::pawn_attacks(from, Colore::Nero) & nemici;
        while caps != 0 {
            let to = caps.trailing_zeros() as usize;
            caps &= caps - 1;
            let flag = if to / 8 == 0 { MoveFlag::Promotion } else { MoveFlag::Capture };
            mosse.push(Mossa::new(from, to, flag));
        }
    }
}