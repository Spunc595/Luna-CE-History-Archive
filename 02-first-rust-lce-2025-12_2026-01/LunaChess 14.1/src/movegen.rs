use crate::board::{Scacchiera, Mossa, MoveFlag, Pezzo, Colore};
use crate::attacks::pawn_attacks;

// Array di appoggio per iterare i pezzi
const PEZZI_ITER: [Pezzo; 6] = [
    Pezzo::Pedone, Pezzo::Cavallo, Pezzo::Alfiere, 
    Pezzo::Torre, Pezzo::Regina, Pezzo::Re
];

pub fn genera_mosse(s: &Scacchiera) -> Vec<Mossa> {
    let mut moves = Vec::with_capacity(256);
    genera_tutte_mosse_pseudo_legali(s, &mut moves);
    moves
}

pub fn genera_tutte_mosse_pseudo_legali(s: &Scacchiera, moves: &mut Vec<Mossa>) {
    let us = s.turno;
    let occ = s.occupazione();
    let nemici = s.colori[us.opposto().indice()];
    let vuote = !occ;

    for &pezzo in &PEZZI_ITER {
        // Ottieni bitboard dei pezzi di quel tipo e colore
        let mut bb = s.pezzi[pezzo.indice()] & s.colori[us.indice()];

        while bb != 0 {
            let from = bb.trailing_zeros() as usize;
            bb &= bb - 1; // Rimuove il bit processato (LSB reset)

            if pezzo == Pezzo::Pedone {
                genera_mosse_pedone(s, from, us, vuote, nemici, moves);
            } else {
                // Generazione pezzi normali (N, B, R, Q, K)
                // ORA FUNZIONA: s.get_attacks_for_piece è stato aggiunto in board.rs
                let mut attacchi = s.get_attacks_for_piece(pezzo, from, occ);
                
                // Non possiamo mangiare i nostri pezzi
                attacchi &= !s.colori[us.indice()];

                while attacchi != 0 {
                    let to = attacchi.trailing_zeros() as usize;
                    attacchi &= attacchi - 1;

                    let flag = if (nemici & (1 << to)) != 0 { 
                        MoveFlag::Capture 
                    } else { 
                        MoveFlag::None 
                    };
                    moves.push(Mossa::new(from, to, flag));
                }
            }
        }
    }

    genera_arrocchi(s, us, occ, moves);
}

fn genera_mosse_pedone(s: &Scacchiera, from: usize, us: Colore, vuote: u64, nemici: u64, moves: &mut Vec<Mossa>) {
    let riga = from / 8;
    
    // Definiamo direzione e righe di promozione/partenza
    let (up, start_rank, promo_rank) = match us {
        Colore::Bianco => (8isize, 1, 6), // +8, riga 1 (partenza), riga 6 (diventa 7->promozione)
        Colore::Nero => (-8isize, 6, 1),  // -8, riga 6 (partenza), riga 1 (diventa 0->promozione)
    };

    let to_push = (from as isize + up) as usize;
    
    // 1. Spinta singola
    if (vuote & (1 << to_push)) != 0 {
        if riga == promo_rank {
            // Promozione
            moves.push(Mossa::new(from, to_push, MoveFlag::Promotion));
        } else {
            // Mossa normale
            moves.push(Mossa::new(from, to_push, MoveFlag::None));
            
            // 2. Spinta doppia (solo se la singola è libera e siamo in start_rank)
            let to_double = (from as isize + up * 2) as usize;
            if riga == start_rank && (vuote & (1 << to_double)) != 0 {
                moves.push(Mossa::new(from, to_double, MoveFlag::DoublePawnPush));
            }
        }
    }

    // 3. Catture
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

    // 4. En Passant
    if let Some(ep_sq) = s.ep_square {
        // Se c'è una casella EP valida, verifichiamo se il pedone la attacca
        let ep_attacks = pawn_attacks(from, us) & (1 << ep_sq);
        if ep_attacks != 0 {
            moves.push(Mossa::new(from, ep_sq as usize, MoveFlag::EnPassant));
        }
    }
}

fn genera_arrocchi(s: &Scacchiera, us: Colore, occ: u64, moves: &mut Vec<Mossa>) {
    if s.re_in_scacco(us) { return; }

    // Logica Arrocco: Verifica diritti, caselle vuote tra Re e Torre, e caselle non attaccate
    if us == Colore::Bianco {
        // Arrocco Corto (K)
        if (s.diritti_arrocco & 1) != 0 
           && (occ & 0x60) == 0 // f1(bit 5), g1(bit 6) vuote
           && !s.casella_attaccata(5, Colore::Nero) 
           && !s.casella_attaccata(6, Colore::Nero) {
            moves.push(Mossa::new(4, 6, MoveFlag::Castle));
        }
        // Arrocco Lungo (Q)
        if (s.diritti_arrocco & 2) != 0 
           && (occ & 0xE) == 0 // b1, c1, d1 vuote
           && !s.casella_attaccata(3, Colore::Nero) 
           && !s.casella_attaccata(2, Colore::Nero) {
            moves.push(Mossa::new(4, 2, MoveFlag::Castle));
        }
    } else {
        // Nero
        // Arrocco Corto (k)
        if (s.diritti_arrocco & 4) != 0 
           && (occ & (0x60 << 56)) == 0 // f8, g8
           && !s.casella_attaccata(61, Colore::Bianco) 
           && !s.casella_attaccata(62, Colore::Bianco) {
            moves.push(Mossa::new(60, 62, MoveFlag::Castle));
        }
        // Arrocco Lungo (q)
        if (s.diritti_arrocco & 8) != 0 
           && (occ & (0xE << 56)) == 0 // b8, c8, d8
           && !s.casella_attaccata(59, Colore::Bianco) 
           && !s.casella_attaccata(58, Colore::Bianco) {
            moves.push(Mossa::new(60, 58, MoveFlag::Castle));
        }
    }
}