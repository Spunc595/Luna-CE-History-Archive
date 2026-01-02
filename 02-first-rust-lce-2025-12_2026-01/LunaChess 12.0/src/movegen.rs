use crate::board::{Scacchiera, Pezzo, Colore, Casella, Mossa, MoveFlag, Bitboard};
use crate::attacks::pawn_attacks;

#[derive(Clone, Copy)]
pub struct MossaConInfo { pub mossa: Mossa, pub score: i32 }

pub fn genera_tutte_mosse_pseudo_legali(s: &Scacchiera, mosse: &mut Vec<Mossa>) {
    let us = s.turno();
    let occ = s.occupazione();
    let amici = s.bitboard_colore(us);
    let nemici = s.bitboard_colore(us.opposto());

    for p_idx in 0..6 {
        let pezzo = match p_idx { 0=>Pezzo::Pedone, 1=>Pezzo::Cavallo, 2=>Pezzo::Alfiere, 3=>Pezzo::Torre, 4=>Pezzo::Regina, _=>Pezzo::Re };
        let mut bb = s.bitboard_pezzo_colore(pezzo, us);
        while bb != 0 {
            let from = bb.trailing_zeros() as usize;
            bb &= bb - 1;
            if pezzo == Pezzo::Pedone {
                genera_mosse_pedone(s, from, us, occ, nemici, mosse);
            } else {
                let mut targets = s.get_attacks_for_piece(pezzo, from, occ) & !amici;
                while targets != 0 {
                    let to = targets.trailing_zeros() as usize;
                    targets &= targets - 1;
                    let is_cap = (nemici & (1 << to)) != 0;
                    mosse.push(Mossa::new_with_flag(Casella(from), Casella(to), None, if is_cap { MoveFlag::Capture } else { MoveFlag::None }));
                }
            }
        }
    }
    genera_arrocco(s, us, occ, mosse);
}

pub fn genera_solo_catture(s: &Scacchiera, mosse: &mut Vec<Mossa>) {
    let us = s.turno();
    let nemici = s.bitboard_colore(us.opposto());
    let occ = s.occupazione();

    for p_idx in 0..6 {
        let pezzo = match p_idx { 0=>Pezzo::Pedone, 1=>Pezzo::Cavallo, 2=>Pezzo::Alfiere, 3=>Pezzo::Torre, 4=>Pezzo::Regina, _=>Pezzo::Re };
        let mut bb = s.bitboard_pezzo_colore(pezzo, us);
        while bb != 0 {
            let from = bb.trailing_zeros() as usize;
            bb &= bb - 1;
            let mut targets = s.get_attacks_for_piece(pezzo, from, occ) & nemici;
            while targets != 0 {
                let to = targets.trailing_zeros() as usize;
                targets &= targets - 1;
                if pezzo == Pezzo::Pedone && (to / 8 == 0 || to / 8 == 7) {
                    aggiungi_promozioni(from, to, true, mosse);
                } else {
                    mosse.push(Mossa::new_with_flag(Casella(from), Casella(to), None, MoveFlag::Capture));
                }
            }
        }
    }
}

pub fn genera_mosse_pedone(s: &Scacchiera, from: usize, us: Colore, occ: Bitboard, nemici: Bitboard, mosse: &mut Vec<Mossa>) {
    let rank_start = if us == Colore::Bianco { 1 } else { 6 };
    let rank_prom = if us == Colore::Bianco { 6 } else { 1 };
    let to_1 = if us == Colore::Bianco { from + 8 } else { from - 8 };

    if (occ & (1 << to_1)) == 0 {
        if from / 8 == rank_prom { aggiungi_promozioni(from, to_1, false, mosse); }
        else {
            mosse.push(Mossa::new_with_flag(Casella(from), Casella(to_1), None, MoveFlag::None));
            let to_2 = if us == Colore::Bianco { from + 16 } else { from - 16 };
            if from / 8 == rank_start && (occ & (1 << to_2)) == 0 {
                mosse.push(Mossa::new_with_flag(Casella(from), Casella(to_2), None, MoveFlag::DoublePawnPush));
            }
        }
    }
    let mut caps = pawn_attacks(from, us) & nemici;
    if let Some(ep) = s.en_passant { caps |= (1 << ep) & pawn_attacks(from, us); }
    while caps != 0 {
        let to = caps.trailing_zeros() as usize;
        caps &= caps - 1;
        let flag = if Some(to) == s.en_passant { MoveFlag::EnPassant } else { MoveFlag::Capture };
        if from / 8 == rank_prom { aggiungi_promozioni(from, to, true, mosse); }
        else { mosse.push(Mossa::new_with_flag(Casella(from), Casella(to), None, flag)); }
    }
}

pub fn aggiungi_promozioni(from: usize, to: usize, is_cap: bool, mosse: &mut Vec<Mossa>) {
    let f = if is_cap { MoveFlag::PromotionCapture } else { MoveFlag::Promotion };
    for p in [Pezzo::Regina, Pezzo::Torre, Pezzo::Alfiere, Pezzo::Cavallo] {
        mosse.push(Mossa::new_with_flag(Casella(from), Casella(to), Some(p), f));
    }
}

pub fn genera_arrocco(s: &Scacchiera, us: Colore, occ: u64, mosse: &mut Vec<Mossa>) {
    let r = s.diritti_arrocco_struct();
    let (ks, qs) = if us == Colore::Bianco { (r.bianco_lato_re, r.bianco_lato_regina) } else { (r.nero_lato_re, r.nero_lato_regina) };
    if s.re_in_scacco(us) { return; }
    let (k_idx, k_t, q_t) = if us == Colore::Bianco { (4, 6, 2) } else { (60, 62, 58) };
    if ks {
        let (f, g) = if us == Colore::Bianco { (5, 6) } else { (61, 62) };
        if (occ & (1 << f | 1 << g)) == 0 && !s.casella_attaccata(Casella(f), us.opposto()) && !s.casella_attaccata(Casella(g), us.opposto()) {
            mosse.push(Mossa::new_with_flag(Casella(k_idx), Casella(k_t), None, MoveFlag::CastleKingSide));
        }
    }
    if qs {
        let (b, c, d) = if us == Colore::Bianco { (1, 2, 3) } else { (57, 58, 59) };
        if (occ & (1 << b | 1 << c | 1 << d)) == 0 && !s.casella_attaccata(Casella(d), us.opposto()) && !s.casella_attaccata(Casella(c), us.opposto()) {
            mosse.push(Mossa::new_with_flag(Casella(k_idx), Casella(q_t), None, MoveFlag::CastleQueenSide));
        }
    }
}