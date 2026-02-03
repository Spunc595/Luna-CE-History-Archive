use crate::board::{Scacchiera, Mossa, MoveFlag, Colore, Pezzo, Bitboard};

/// Genera tutte le mosse pseudo-legali per la posizione corrente.
pub fn genera_mosse(s: &Scacchiera) -> Vec<Mossa> {
    let mut moves = Vec::with_capacity(64);
    
    let us = s.turno;
    let them = us.opposto();
    let us_idx = us.indice();
    let them_idx = them.indice();
    
    let occ = s.occupazione();
    let amici = s.colori[us_idx];
    let nemici = s.colori[them_idx];
    let vuote = !occ;

    // 1. PEDONI (Catture e Promozioni prima per l'ordinamento)
    let pedoni = s.pezzi[Pezzo::Pedone.indice()] & amici;
    if pedoni != 0 {
        genera_mosse_pedone(s, pedoni, us, vuote, nemici, &mut moves);
    }

    // 2. PEZZI (Catture)
    genera_mosse_pezzi(s, us, occ, nemici, true, &mut moves);

    // 3. PEZZI (Quiet)
    genera_mosse_pezzi(s, us, occ, vuote, false, &mut moves);

    // 4. RE (Quiet e Catture)
    let re_bb = s.pezzi[Pezzo::Re.indice()] & amici;
    if re_bb != 0 {
        let from = re_bb.trailing_zeros() as usize;
        let attacchi = crate::attacks::king_attacks(from);
        
        // Catture re
        push_standard_moves(from, attacchi & nemici, MoveFlag::Capture, &mut moves);
        // Quiet re
        push_standard_moves(from, attacchi & vuote, MoveFlag::None, &mut moves);
    }

    // 5. ARROCCO
    genera_arrocco(s, us, occ, &mut moves);
    
    moves
}

/// Genera mosse legali filtrando le pseudo-legali
pub fn genera_mosse_legali(s: &Scacchiera) -> Vec<Mossa> {
    let mut mosse = genera_mosse(s);
    let keys = crate::zobrist::ZobristKeys::default(); // Usa default coerente
    
    mosse.retain(|m| {
        let mut copia = s.clone();
        copia.esegui_mossa(m, &keys)
    });
    
    mosse
}

#[inline(always)]
fn push_standard_moves(from: usize, mut targets: Bitboard, flag: MoveFlag, moves: &mut Vec<Mossa>) {
    while targets != 0 {
        let to = targets.trailing_zeros() as usize;
        moves.push(Mossa::new(from, to, flag, None));
        targets &= targets - 1;
    }
}

fn genera_mosse_pezzi(s: &Scacchiera, us: Colore, occ: Bitboard, targets: Bitboard, is_capture: bool, moves: &mut Vec<Mossa>) {
    let flag = if is_capture { MoveFlag::Capture } else { MoveFlag::None };
    let amici = s.colori[us.indice()];

    // Cavalli
    let mut bb = s.pezzi[Pezzo::Cavallo.indice()] & amici;
    while bb != 0 {
        let from = bb.trailing_zeros() as usize;
        push_standard_moves(from, crate::attacks::knight_attacks(from) & targets, flag, moves);
        bb &= bb - 1;
    }

    // Alfieri
    let mut bb = s.pezzi[Pezzo::Alfiere.indice()] & amici;
    while bb != 0 {
        let from = bb.trailing_zeros() as usize;
        push_standard_moves(from, crate::attacks::bishop_attacks(from, occ) & targets, flag, moves);
        bb &= bb - 1;
    }

    // Torri
    let mut bb = s.pezzi[Pezzo::Torre.indice()] & amici;
    while bb != 0 {
        let from = bb.trailing_zeros() as usize;
        push_standard_moves(from, crate::attacks::rook_attacks(from, occ) & targets, flag, moves);
        bb &= bb - 1;
    }

    // Regine
    let mut bb = s.pezzi[Pezzo::Regina.indice()] & amici;
    while bb != 0 {
        let from = bb.trailing_zeros() as usize;
        let attacchi = crate::attacks::bishop_attacks(from, occ) | crate::attacks::rook_attacks(from, occ);
        push_standard_moves(from, attacchi & targets, flag, moves);
        bb &= bb - 1;
    }
}

fn genera_mosse_pedone(s: &Scacchiera, mut bb: Bitboard, us: Colore, vuote: Bitboard, nemici: Bitboard, moves: &mut Vec<Mossa>) {
    let push_dir = if us == Colore::Bianco { 8i32 } else { -8i32 };
    let start_mask = if us == Colore::Bianco { 0x000000000000FF00u64 } else { 0x00FF000000000000u64 };
    let prom_mask = if us == Colore::Bianco { 0xFF00000000000000u64 } else { 0x00000000000000FFu64 };

    while bb != 0 {
        let from = bb.trailing_zeros() as usize;
        let from_bb = 1u64 << from;
        
        // 1. Spinte
        let to = (from as i32 + push_dir) as usize;
        if to < 64 && (vuote & (1u64 << to)) != 0 {
            if (1u64 << to & prom_mask) != 0 {
                genera_promozioni(from, to, false, moves);
            } else {
                moves.push(Mossa::new(from, to, MoveFlag::None, None));
                // Doppia spinta (solo se la prima casa è vuota)
                if (from_bb & start_mask) != 0 {
                    let to2 = (to as i32 + push_dir) as usize;
                    if (vuote & (1u64 << to2)) != 0 {
                        moves.push(Mossa::new(from, to2, MoveFlag::DoublePawnPush, None));
                    }
                }
            }
        }

        // 2. Catture
        let mut attacchi = crate::attacks::pawn_attacks(from, us) & nemici;
        while attacchi != 0 {
            let to_cap = attacchi.trailing_zeros() as usize;
            if (1u64 << to_cap & prom_mask) != 0 {
                genera_promozioni(from, to_cap, true, moves);
            } else {
                moves.push(Mossa::new(from, to_cap, MoveFlag::Capture, None));
            }
            attacchi &= attacchi - 1;
        }

        // 3. En Passant
        if let Some(ep_sq) = s.ep_square {
            if (crate::attacks::pawn_attacks(from, us) & (1u64 << ep_sq)) != 0 {
                moves.push(Mossa::new(from, ep_sq, MoveFlag::EnPassant, None));
            }
        }

        bb &= bb - 1;
    }
}

fn genera_promozioni(from: usize, to: usize, is_capture: bool, moves: &mut Vec<Mossa>) {
    let flag = if is_capture { MoveFlag::PromotionCapture } else { MoveFlag::Promotion };
    for &pezzo in &[Pezzo::Regina, Pezzo::Torre, Pezzo::Alfiere, Pezzo::Cavallo] {
        moves.push(Mossa::new(from, to, flag, Some(pezzo)));
    }
}

fn genera_arrocco(s: &Scacchiera, us: Colore, occ: Bitboard, moves: &mut Vec<Mossa>) {
    if s.re_in_scacco(us) { return; }
    let them = us.opposto();

    if us == Colore::Bianco {
        // Corto (K)
        if (s.diritti_arrocco & 1) != 0 && (occ & 0x60) == 0 {
            if !s.casella_attaccata(5, them) && !s.casella_attaccata(6, them) {
                moves.push(Mossa::new(4, 6, MoveFlag::Castle, None));
            }
        }
        // Lungo (Q)
        if (s.diritti_arrocco & 2) != 0 && (occ & 0xE) == 0 {
            if !s.casella_attaccata(3, them) && !s.casella_attaccata(2, them) {
                moves.push(Mossa::new(4, 2, MoveFlag::Castle, None));
            }
        }
    } else {
        // Corto (k)
        if (s.diritti_arrocco & 4) != 0 && (occ & 0x6000000000000000) == 0 {
            if !s.casella_attaccata(61, them) && !s.casella_attaccata(62, them) {
                moves.push(Mossa::new(60, 62, MoveFlag::Castle, None));
            }
        }
        // Lungo (q)
        if (s.diritti_arrocco & 8) != 0 && (occ & 0xE000000000000000) == 0 {
            if !s.casella_attaccata(59, them) && !s.casella_attaccata(58, them) {
                moves.push(Mossa::new(60, 58, MoveFlag::Castle, None));
            }
        }
    }
}

pub fn ordina_mosse(mosse: &mut [Mossa], board: &Scacchiera) {
    mosse.sort_by_key(|m| -m.priority(board));
}