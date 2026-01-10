use crate::board::{Scacchiera, Mossa, MoveFlag, Pezzo, Colore, Bitboard};

pub fn genera_mosse(s: &Scacchiera) -> Vec<Mossa> {
    let mut moves = Vec::with_capacity(256);
    let us = s.turno;
    let occ = s.occupazione();
    let nemici = s.colori[us.opposto().indice()];
    let vuote = !occ;

    for p_idx in 0..6 {
        let mut bb = s.pezzi[p_idx] & s.colori[us.indice()];
        let pezzo = match p_idx {
            0 => Pezzo::Pedone, 1 => Pezzo::Cavallo, 2 => Pezzo::Alfiere,
            3 => Pezzo::Torre, 4 => Pezzo::Regina, _ => Pezzo::Re,
        };

        if pezzo == Pezzo::Pedone {
            genera_mosse_pedone(s, bb, us, vuote, nemici, &mut moves);
            continue;
        }

        while bb != 0 {
            let from = bb.trailing_zeros() as usize;
            let mut attacchi = s.get_attacks_for_piece(pezzo, from, occ);
            attacchi &= !s.colori[us.indice()];

            while attacchi != 0 {
                let to = attacchi.trailing_zeros() as usize;
                let is_cap = (nemici & (1u64 << to)) != 0;
                moves.push(Mossa::new(from, to, if is_cap { MoveFlag::Capture } else { MoveFlag::None }));
                attacchi &= attacchi - 1;
            }
            bb &= bb - 1;
        }
    }
    genera_arrocco(s, us, occ, &mut moves);
    moves
}

fn genera_mosse_pedone(_s: &Scacchiera, mut bb: Bitboard, us: Colore, vuote: Bitboard, nemici: Bitboard, moves: &mut Vec<Mossa>) {
    while bb != 0 {
        let from = bb.trailing_zeros() as usize;
        let riga = from / 8;
        
        // Spinta singola
        let to = if us == Colore::Bianco { from + 8 } else { from - 8 };
        if (vuote & (1u64 << to)) != 0 {
            if (us == Colore::Bianco && to >= 56) || (us == Colore::Nero && to <= 7) {
                moves.push(Mossa::new(from, to, MoveFlag::Promotion));
            } else {
                moves.push(Mossa::new(from, to, MoveFlag::None));
                // Doppia spinta
                let to2 = if us == Colore::Bianco { from + 16 } else { from - 16 };
                if ((us == Colore::Bianco && riga == 1) || (us == Colore::Nero && riga == 6)) && (vuote & (1u64 << to2)) != 0 {
                    moves.push(Mossa::new(from, to2, MoveFlag::DoublePawnPush));
                }
            }
        }
        
        // Catture
        let attacchi = crate::attacks::pawn_attacks(from, us) & nemici;
        let mut att_bb = attacchi;
        while att_bb != 0 {
            let to_cap = att_bb.trailing_zeros() as usize;
            if (us == Colore::Bianco && to_cap >= 56) || (us == Colore::Nero && to_cap <= 7) {
                moves.push(Mossa::new(from, to_cap, MoveFlag::Promotion));
            } else {
                moves.push(Mossa::new(from, to_cap, MoveFlag::Capture));
            }
            att_bb &= att_bb - 1;
        }
        bb &= bb - 1;
    }
}

fn genera_arrocco(s: &Scacchiera, us: Colore, occ: Bitboard, moves: &mut Vec<Mossa>) {
    if s.re_in_scacco(us) { return; }
    if us == Colore::Bianco {
        if (s.diritti_arrocco & 1) != 0 && (occ & 0x60) == 0 && !s.casella_attaccata(5, Colore::Nero) && !s.casella_attaccata(6, Colore::Nero) {
            moves.push(Mossa::new(4, 6, MoveFlag::Castle));
        }
        if (s.diritti_arrocco & 2) != 0 && (occ & 0xE) == 0 && !s.casella_attaccata(3, Colore::Nero) && !s.casella_attaccata(2, Colore::Nero) {
            moves.push(Mossa::new(4, 2, MoveFlag::Castle));
        }
    } else {
        if (s.diritti_arrocco & 4) != 0 && (occ & (0x60u64 << 56)) == 0 && !s.casella_attaccata(61, Colore::Bianco) && !s.casella_attaccata(62, Colore::Bianco) {
            moves.push(Mossa::new(60, 62, MoveFlag::Castle));
        }
        if (s.diritti_arrocco & 8) != 0 && (occ & (0xEu64 << 56)) == 0 && !s.casella_attaccata(59, Colore::Bianco) && !s.casella_attaccata(58, Colore::Bianco) {
            moves.push(Mossa::new(60, 58, MoveFlag::Castle));
        }
    }
}