use crate::board::{Scacchiera, Mossa, MoveFlag, Pezzo, Colore, Bitboard};
use crate::attacks::pawn_attacks;

// Iteriamo solo sui pezzi "non pedone" nel ciclo generico per efficienza
const PEZZI_MAGGIORI: [Pezzo; 5] = [
    Pezzo::Cavallo, Pezzo::Alfiere, 
    Pezzo::Torre, Pezzo::Regina, Pezzo::Re
];

pub fn genera_mosse(s: &Scacchiera) -> Vec<Mossa> {
    // Capacità stimata media per evitare riallocazioni
    let mut moves = Vec::with_capacity(256);
    
    let us = s.turno;
    let occ = s.occupazione();
    let nemici = s.colori[us.opposto().indice()];
    let vuote = !occ;

    // 1. Genera mosse dei PEDONI (logica separata per efficienza)
    let bb_pedoni = s.pezzi[Pezzo::Pedone.indice()] & s.colori[us.indice()];
    if bb_pedoni != 0 {
        genera_mosse_pedone_bulk(s, bb_pedoni, us, vuote, nemici, &mut moves);
    }

    // 2. Genera ARROCCHI
    genera_arrocchi(s, us, occ, &mut moves);

    // 3. Genera mosse per ALTRI PEZZI (Cavallo, Alfiere, Torre, Regina, Re)
    for &pezzo in &PEZZI_MAGGIORI {
        let mut bb = s.pezzi[pezzo.indice()] & s.colori[us.indice()];
        
        while bb != 0 {
            let from = bb.trailing_zeros() as usize;
            bb &= bb - 1;

            let mut attacchi = s.get_attacks_for_piece(pezzo, from, occ);
            // Rimuovi le case occupate dai pezzi amici
            attacchi &= !s.colori[us.indice()]; 

            while attacchi != 0 {
                let to = attacchi.trailing_zeros() as usize;
                attacchi &= attacchi - 1;

                let flag = if (nemici & (1u64 << to)) != 0 { 
                    MoveFlag::Capture 
                } else { 
                    MoveFlag::None 
                };
                
                moves.push(Mossa::new(from, to, flag));
            }
        }
    }

    moves
}

// Funzione ottimizzata che itera sul bitboard dei pedoni
fn genera_mosse_pedone_bulk(
    s: &Scacchiera, 
    mut bb: Bitboard, 
    us: Colore, 
    vuote: Bitboard, 
    nemici: Bitboard, 
    moves: &mut Vec<Mossa>
) {
    let (up, start_rank, promo_rank) = match us {
        Colore::Bianco => (8isize, 1, 6),
        Colore::Nero => (-8isize, 6, 1),
    };

    while bb != 0 {
        let from = bb.trailing_zeros() as usize;
        bb &= bb - 1;
        
        let riga = from / 8;
        let to_push = (from as isize + up) as usize;

        // --- Movimento in avanti (Push) ---
        if (vuote & (1u64 << to_push)) != 0 {
            // Promozione
            if riga == promo_rank {
                // NOTA: Qui si dovrebbero generare 4 mosse (Q, R, B, N).
                // Per ora usiamo il flag generico Promotion (che il motore userà come Regina).
                moves.push(Mossa::new(from, to_push, MoveFlag::Promotion));
            } else {
                // Spinta singola
                moves.push(Mossa::new(from, to_push, MoveFlag::None));
                
                // Spinta doppia (solo se siamo al rango di partenza)
                let to_double = (from as isize + up * 2) as usize;
                if riga == start_rank && (vuote & (1u64 << to_double)) != 0 {
                    moves.push(Mossa::new(from, to_double, MoveFlag::DoublePawnPush));
                }
            }
        }

        // --- Catture ---
        let mut att_bb = pawn_attacks(from, us) & nemici;
        while att_bb != 0 {
            let to = att_bb.trailing_zeros() as usize;
            att_bb &= att_bb - 1;
            
            if riga == promo_rank {
                moves.push(Mossa::new(from, to, MoveFlag::Promotion)); // Capture + Promo
            } else {
                moves.push(Mossa::new(from, to, MoveFlag::Capture));
            }
        }

        // --- En Passant ---
        if let Some(ep_sq) = s.ep_square {
            // Verifica se il pedone attacca la casa EP
            if (pawn_attacks(from, us) & (1u64 << ep_sq)) != 0 {
                moves.push(Mossa::new(from, ep_sq as usize, MoveFlag::EnPassant));
            }
        }
    }
}

fn genera_arrocchi(s: &Scacchiera, us: Colore, occ: Bitboard, moves: &mut Vec<Mossa>) {
    // Non si può arroccare se si è sotto scacco
    if s.re_in_scacco(us) { return; }

    if us == Colore::Bianco {
        // Arrocco Corto (O-O) -> Re da e1(4) a g1(6)
        if (s.diritti_arrocco & 1) != 0 {
            // Verifica vuote f1, g1 (mask 0x60 = 0110_0000)
            if (occ & 0x60) == 0 {
                // Verifica non attaccate f1(5), g1(6)
                if !s.casella_attaccata(5, Colore::Nero) && !s.casella_attaccata(6, Colore::Nero) {
                    moves.push(Mossa::new(4, 6, MoveFlag::Castle));
                }
            }
        }
        // Arrocco Lungo (O-O-O) -> Re da e1(4) a c1(2)
        if (s.diritti_arrocco & 2) != 0 {
            // Verifica vuote d1, c1, b1 (mask 0x0E = 0000_1110)
            if (occ & 0xE) == 0 {
                // Verifica non attaccate d1(3), c1(2)
                if !s.casella_attaccata(3, Colore::Nero) && !s.casella_attaccata(2, Colore::Nero) {
                    moves.push(Mossa::new(4, 2, MoveFlag::Castle));
                }
            }
        }
    } else {
        // Arrocco Corto Nero (O-O) -> Re da e8(60) a g8(62)
        if (s.diritti_arrocco & 4) != 0 {
            if (occ & (0x60u64 << 56)) == 0 {
                if !s.casella_attaccata(61, Colore::Bianco) && !s.casella_attaccata(62, Colore::Bianco) {
                    moves.push(Mossa::new(60, 62, MoveFlag::Castle));
                }
            }
        }
        // Arrocco Lungo Nero (O-O-O) -> Re da e8(60) a c8(58)
        if (s.diritti_arrocco & 8) != 0 {
            if (occ & (0xEu64 << 56)) == 0 {
                if !s.casella_attaccata(59, Colore::Bianco) && !s.casella_attaccata(58, Colore::Bianco) {
                    moves.push(Mossa::new(60, 58, MoveFlag::Castle));
                }
            }
        }
    }
}