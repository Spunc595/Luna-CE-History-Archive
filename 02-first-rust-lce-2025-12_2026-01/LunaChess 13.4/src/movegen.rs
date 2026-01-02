use crate::board::{Scacchiera, Mossa, MoveFlag, Bitboard, Pezzo, Colore};
use crate::attacks::*;

pub fn genera_tutte_mosse_pseudo_legali(s: &Scacchiera, moves: &mut Vec<Mossa>) {
    let us = s.turno;
    let occ = s.occupazione();
    let nemici = s.colori[us.opposto().indice()];
    let vuote = !occ;

    for p_idx in 0..6 {
        let pezzo = unsafe { std::mem::transmute::<u8, Pezzo>(p_idx as u8) };
        let mut bb = s.pezzi[p_idx] & s.colori[us.indice()];

        while bb != 0 {
            let from = bb.trailing_zeros() as usize;
            
            if pezzo == Pezzo::Pedone {
                genera_mosse_pedone(s, from, us, vuote, nemici, moves);
            } else {
                let mut attacchi = s.get_attacks_for_piece(pezzo, from, occ);
                attacchi &= !s.colori[us.indice()];

                while attacchi != 0 {
                    let to = attacchi.trailing_zeros() as usize;
                    let flag = if (nemici & (1 << to)) != 0 { MoveFlag::Capture } else { MoveFlag::None };
                    moves.push(Mossa::new(from, to, flag));
                    attacchi &= attacchi - 1;
                }
            }
            bb &= bb - 1;
        }
    }
    genera_arrocchi(s, us, occ, moves);
}

fn genera_mosse_pedone(s: &Scacchiera, from: usize, us: Colore, vuote: Bitboard, nemici: Bitboard, moves: &mut Vec<Mossa>) {
    let riga = from / 8;
    if us == Colore::Bianco {
        let to = from + 8;
        if to < 64 && (vuote & (1 << to)) != 0 {
            if riga == 6 { moves.push(Mossa::new(from, to, MoveFlag::Promotion)); }
            else {
                moves.push(Mossa::new(from, to, MoveFlag::None));
                let to2 = from + 16;
                if riga == 1 && (vuote & (1 << to2)) != 0 { moves.push(Mossa::new(from, to2, MoveFlag::DoublePawnPush)); }
            }
        }
        let mut att_bb = pawn_attacks(from, Colore::Bianco) & nemici;
        while att_bb != 0 {
            let to = att_bb.trailing_zeros() as usize;
            if riga == 6 { moves.push(Mossa::new(from, to, MoveFlag::Promotion)); }
            else { moves.push(Mossa::new(from, to, MoveFlag::Capture)); }
            att_bb &= att_bb - 1;
        }
    } else {
        let to = from - 8;
        if (vuote & (1 << to)) != 0 {
            if riga == 1 { moves.push(Mossa::new(from, to, MoveFlag::Promotion)); }
            else {
                moves.push(Mossa::new(from, to, MoveFlag::None));
                let to2 = from - 16;
                if riga == 6 && (vuote & (1 << to2)) != 0 { moves.push(Mossa::new(from, to2, MoveFlag::DoublePawnPush)); }
            }
        }
        let mut att_bb = pawn_attacks(from, Colore::Nero) & nemici;
        while att_bb != 0 {
            let to = att_bb.trailing_zeros() as usize;
            if riga == 1 { moves.push(Mossa::new(from, to, MoveFlag::Promotion)); }
            else { moves.push(Mossa::new(from, to, MoveFlag::Capture)); }
            att_bb &= att_bb - 1;
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
        if (s.diritti_arrocco & 4) != 0 && (occ & (0x60 << 56)) == 0 && !s.casella_attaccata(61, Colore::Bianco) && !s.casella_attaccata(62, Colore::Bianco) {
            moves.push(Mossa::new(60, 62, MoveFlag::Castle));
        }
        if (s.diritti_arrocco & 8) != 0 && (occ & (0xE << 56)) == 0 && !s.casella_attaccata(59, Colore::Bianco) && !s.casella_attaccata(58, Colore::Bianco) {
            moves.push(Mossa::new(60, 58, MoveFlag::Castle));
        }
    }
}