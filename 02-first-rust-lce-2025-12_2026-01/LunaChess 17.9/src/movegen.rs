use crate::board::{Scacchiera, Mossa, MoveFlag, Pezzo, Colore, Bitboard};

/// Genera tutte le mosse pseudo-legali per la posizione corrente.
pub fn genera_mosse(s: &Scacchiera) -> Vec<Mossa> {
    let mut moves = Vec::with_capacity(128);
    let us = s.turno;
    let occ = s.occupazione();
    let amici = s.colori[us.indice()];
    let nemici = s.colori[us.opposto().indice()];
    let vuote = !occ;

    // 1. GENERAZIONE MOSSE PEDONI
    let pedoni = s.pezzi[0] & amici;
    genera_mosse_pedone(s, pedoni, us, vuote, nemici, &mut moves);

    // 2. PEZZI A SALTO (Cavallo e Re)
    // Cavalli
    let mut bb_cavalli = s.pezzi[1] & amici;
    while bb_cavalli != 0 {
        let from = bb_cavalli.trailing_zeros() as usize;
        let mut attacchi = crate::attacks::knight_attacks(from) & !amici;
        push_standard_moves(from, attacchi, nemici, &mut moves);
        bb_cavalli &= bb_cavalli - 1;
    }

    // Re
    let from_re = (s.pezzi[5] & amici).trailing_zeros() as usize;
    if from_re < 64 {
        let attacchi_re = crate::attacks::king_attacks(from_re) & !amici;
        push_standard_moves(from_re, attacchi_re, nemici, &mut moves);
    }

    // 3. PEZZI SLIDERS (Alfiere, Torre, Regina)
    // Alfieri
    let mut bb_alfieri = s.pezzi[2] & amici;
    while bb_alfieri != 0 {
        let from = bb_alfieri.trailing_zeros() as usize;
        let attacchi = crate::attacks::bishop_attacks(from, occ) & !amici;
        push_standard_moves(from, attacchi, nemici, &mut moves);
        bb_alfieri &= bb_alfieri - 1;
    }

    // Torri
    let mut bb_torri = s.pezzi[3] & amici;
    while bb_torri != 0 {
        let from = bb_torri.trailing_zeros() as usize;
        let attacchi = crate::attacks::rook_attacks(from, occ) & !amici;
        push_standard_moves(from, attacchi, nemici, &mut moves);
        bb_torri &= bb_torri - 1;
    }

    // Regine
    let mut bb_regine = s.pezzi[4] & amici;
    while bb_regine != 0 {
        let from = bb_regine.trailing_zeros() as usize;
        let attacchi = (crate::attacks::bishop_attacks(from, occ) | 
                        crate::attacks::rook_attacks(from, occ)) & !amici;
        push_standard_moves(from, attacchi, nemici, &mut moves);
        bb_regine &= bb_regine - 1;
    }

    // 4. ARROCCO
    genera_arrocco(s, us, occ, &mut moves);

    moves
}

/// Helper per aggiungere mosse normali e catture da una bitboard di attacchi.
#[inline(always)]
fn push_standard_moves(from: usize, mut attacchi: Bitboard, nemici: Bitboard, moves: &mut Vec<Mossa>) {
    while attacchi != 0 {
        let to = attacchi.trailing_zeros() as usize;
        let flag = if (nemici & (1u64 << to)) != 0 { MoveFlag::Capture } else { MoveFlag::None };
        moves.push(Mossa::new(from, to, flag));
        attacchi &= attacchi - 1;
    }
}

fn genera_mosse_pedone(s: &Scacchiera, mut bb: Bitboard, us: Colore, vuote: Bitboard, nemici: Bitboard, moves: &mut Vec<Mossa>) {
    let (shift, rank_7, rank_2) = if us == Colore::Bianco {
        (8i32, 0xFF00000000000000u64, 0x000000000000FF00u64)
    } else {
        (-8i32, 0x000000000000FF00u64, 0xFF00000000000000u64)
    };

    while bb != 0 {
        let from = bb.trailing_zeros() as usize;
        let from_bb = 1u64 << from;
        
        // Spinta singola
        let to = (from as i32 + shift) as usize;
        if (vuote & (1u64 << to)) != 0 {
            if (1u64 << to) & rank_7 != 0 {
                moves.push(Mossa::new(from, to, MoveFlag::Promotion));
            } else {
                moves.push(Mossa::new(from, to, MoveFlag::None));
                // Doppia spinta
                let to2 = (from as i32 + shift * 2) as usize;
                if (from_bb & rank_2 != 0) && (vuote & (1u64 << to2)) != 0 {
                    moves.push(Mossa::new(from, to2, MoveFlag::DoublePawnPush));
                }
            }
        }

        // Catture
        let mut attacchi = crate::attacks::pawn_attacks(from, us) & nemici;
        while attacchi != 0 {
            let to_cap = attacchi.trailing_zeros() as usize;
            if (1u64 << to_cap) & rank_7 != 0 {
                moves.push(Mossa::new(from, to_cap, MoveFlag::Promotion));
            } else {
                moves.push(Mossa::new(from, to_cap, MoveFlag::Capture));
            }
            attacchi &= attacchi - 1;
        }

        // En Passant
        if let Some(ep_sq) = s.ep_square {
            if (crate::attacks::pawn_attacks(from, us) & (1u64 << ep_sq)) != 0 {
                moves.push(Mossa::new(from, ep_sq, MoveFlag::EnPassant));
            }
        }

        bb &= bb - 1;
    }
}



fn genera_arrocco(s: &Scacchiera, us: Colore, occ: Bitboard, moves: &mut Vec<Mossa>) {
    if s.re_in_scacco(us) { return; }

    if us == Colore::Bianco {
        // Corto
        if (s.diritti_arrocco & 1) != 0 && (occ & 0x60) == 0 
           && !s.casella_attaccata(5, Colore::Nero) && !s.casella_attaccata(6, Colore::Nero) {
            moves.push(Mossa::new(4, 6, MoveFlag::Castle));
        }
        // Lungo
        if (s.diritti_arrocco & 2) != 0 && (occ & 0xE) == 0 
           && !s.casella_attaccata(3, Colore::Nero) && !s.casella_attaccata(2, Colore::Nero) {
            moves.push(Mossa::new(4, 2, MoveFlag::Castle));
        }
    } else {
        // Corto
        if (s.diritti_arrocco & 4) != 0 && (occ & (0x60u64 << 56)) == 0 
           && !s.casella_attaccata(61, Colore::Bianco) && !s.casella_attaccata(62, Colore::Bianco) {
            moves.push(Mossa::new(60, 62, MoveFlag::Castle));
        }
        // Lungo
        if (s.diritti_arrocco & 8) != 0 && (occ & (0xEu64 << 56)) == 0 
           && !s.casella_attaccata(59, Colore::Bianco) && !s.casella_attaccata(58, Colore::Bianco) {
            moves.push(Mossa::new(60, 58, MoveFlag::Castle));
        }
    }
}