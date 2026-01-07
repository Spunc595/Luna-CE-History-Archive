use crate::board::{Scacchiera, Mossa, MoveFlag, Pezzo, Colore, Bitboard};
use crate::attacks::pawn_attacks;

const PEZZI_ITER: [Pezzo; 6] = [
    Pezzo::Pedone, Pezzo::Cavallo, Pezzo::Alfiere, 
    Pezzo::Torre, Pezzo::Regina, Pezzo::Re
];

pub fn genera_mosse(s: &Scacchiera) -> Vec<Mossa> {
    let mut moves = Vec::with_capacity(256);
    let us = s.turno;
    let occ = s.occupazione();
    let nemici = s.colori[us.opposto().indice()];
    let vuote = !occ;

    for &pezzo in &PEZZI_ITER {
        let mut bb = s.pezzi[pezzo.indice()] & s.colori[us.indice()];
        while bb != 0 {
            let from = bb.trailing_zeros() as usize;
            bb &= bb - 1;

            if pezzo == Pezzo::Pedone {
                genera_mosse_pedone(s, from, us, vuote, nemici, &mut moves);
            } else {
                let mut attacchi = s.get_attacks_for_piece(pezzo, from, occ);
                attacchi &= !s.colori[us.indice()]; // Non mangiare i propri

                while attacchi != 0 {
                    let to = attacchi.trailing_zeros() as usize;
                    attacchi &= attacchi - 1;
                    let flag = if (nemici & (1u64 << to)) != 0 { MoveFlag::Capture } else { MoveFlag::None };
                    moves.push(Mossa::new(from, to, flag));
                }
            }
        }
    }
    genera_arrocchi(s, us, occ, &mut moves);
    moves
}

fn genera_mosse_pedone(s: &Scacchiera, from: usize, us: Colore, vuote: Bitboard, nemici: Bitboard, moves: &mut Vec<Mossa>) {
    let riga = from / 8;
    let (up, start_rank, promo_rank) = match us {
        Colore::Bianco => (8isize, 1, 6),
        Colore::Nero => (-8isize, 6, 1),
    };

    let to_push = (from as isize + up) as usize;
    if (vuote & (1u64 << to_push)) != 0 {
        if riga == promo_rank {
            moves.push(Mossa::new(from, to_push, MoveFlag::Promotion));
        } else {
            moves.push(Mossa::new(from, to_push, MoveFlag::None));
            let to_double = (from as isize + up * 2) as usize;
            if riga == start_rank && (vuote & (1u64 << to_double)) != 0 {
                moves.push(Mossa::new(from, to_double, MoveFlag::DoublePawnPush));
            }
        }
    }

    let mut att_bb = pawn_attacks(from, us) & nemici;
    while att_bb != 0 {
        let to = att_bb.trailing_zeros() as usize;
        att_bb &= att_bb - 1;
        if riga == promo_rank {
            moves.push(Mossa::new(from, to, MoveFlag::Promotion));
        } else {
            moves.push(Mossa::new(from, to, MoveFlag::Capture));
        }
    }

    if let Some(ep_sq) = s.ep_square {
        if (pawn_attacks(from, us) & (1u64 << ep_sq)) != 0 {
            moves.push(Mossa::new(from, ep_sq as usize, MoveFlag::EnPassant));
        }
    }
}

fn genera_arrocchi(s: &Scacchiera, us: Colore, occ: Bitboard, moves: &mut Vec<Mossa>) {
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