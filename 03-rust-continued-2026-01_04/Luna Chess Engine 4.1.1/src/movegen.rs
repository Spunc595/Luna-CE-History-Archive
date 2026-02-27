use crate::board::{Scacchiera, Mossa, Pezzo, MoveFlag, Colore};
use crate::search::SearchInfo;
use crate::attacks;  // Aggiunto per chiarezza

// -----------------------------
//   PST per MOSSE TRANQUILLE
// -----------------------------
pub const PST_PAWN: [i32; 64] = [
     0,  0,  0,  0,  0,  0,  0,  0,
    50, 50, 50, 50, 50, 50, 50, 50,
    10, 10, 20, 30, 30, 20, 10, 10,
     5,  5, 10, 25, 25, 10,  5,  5,
     0,  0,  0, 20, 20,  0,  0,  0,
     5, -5,-10,  0,  0,-10, -5,  5,
     5, 10, 10,-20,-20, 10, 10,  5,
     0,  0,  0,  0,  0,  0,  0,  0
];

pub const PST_KNIGHT: [i32; 64] = [
    -50,-40,-30,-30,-30,-30,-40,-50,
    -40,-20,  0,  0,  0,  0,-20,-40,
    -30,  0, 10, 15, 15, 10,  0,-30,
    -30,  5, 15, 20, 20, 15,  5,-30,
    -30,  0, 15, 20, 20, 15,  0,-30,
    -30,  5, 10, 15, 15, 10,  5,-30,
    -40,-20,  0,  5,  5,  0,-20,-40,
    -50,-40,-30,-30,-30,-30,-40,-50
];

pub const PST_BISHOP: [i32; 64] = [
    -20,-10,-10,-10,-10,-10,-10,-20,
    -10,  0,  0,  0,  0,  0,  0,-10,
    -10,  0,  5, 10, 10,  5,  0,-10,
    -10,  5,  5, 10, 10,  5,  5,-10,
    -10,  0, 10, 10, 10, 10,  0,-10,
    -10, 10, 10, 10, 10, 10, 10,-10,
    -10,  5,  0,  0,  0,  0,  5,-10,
    -20,-10,-10,-10,-10,-10,-10,-20
];

pub const PST_ROOK: [i32; 64] = [
     0,  0,  0,  0,  0,  0,  0,  0,
     5, 10, 10, 10, 10, 10, 10,  5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
     0,  0,  0,  5,  5,  0,  0,  0
];

pub const PST_QUEEN: [i32; 64] = [
    -20,-10,-10, -5, -5,-10,-10,-20,
    -10,  0,  0,  0,  0,  0,  0,-10,
    -10,  0,  5,  5,  5,  5,  0,-10,
     -5,  0,  5,  5,  5,  5,  0, -5,
      0,  0,  5,  5,  5,  5,  0, -5,
    -10,  5,  5,  5,  5,  5,  0,-10,
    -10,  0,  5,  0,  0,  0,  0,-10,
    -20,-10,-10, -5, -5,-10,-10,-20
];

pub const PST_KING: [i32; 64] = [
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -20,-30,-30,-40,-40,-30,-30,-20,
    -10,-20,-20,-20,-20,-20,-20,-10,
     20, 20,  0,  0,  0,  0, 20, 20,
     20, 30, 10,  0,  0, 10, 30, 20
];

// ---------------------------------------------------
//   GENERAZIONE MOSSE PSEUDO-LEGALI
// ---------------------------------------------------
pub fn genera_mosse(s: &Scacchiera) -> Vec<Mossa> {
    let mut mosse = Vec::with_capacity(64);
    let us = s.turno;
    let them = us.opposto();

    let our_pieces = s.colori[us.indice()];
    let their_pieces = s.colori[them.indice()];
    let all = our_pieces | their_pieces;

    // ------------------------
    //         PEDONI
    // ------------------------
    let pawns = s.pezzi[Pezzo::Pedone.indice()] & our_pieces;
    let (start_rank, prom_rank) =
        if us == Colore::Bianco { (1, 7) } else { (6, 0) };

    let mut temp = pawns;
    while temp != 0 {
        let sq = temp.trailing_zeros() as usize;
        temp &= temp - 1;

        let to = if us == Colore::Bianco { sq + 8 } else { sq - 8 };

        // PUSH 1
        if to < 64 && (all & (1 << to)) == 0 {
            add_pawn_move(sq, to, prom_rank, &mut mosse);

            // PUSH 2
            let to2 = if us == Colore::Bianco { sq + 16 } else { sq - 16 };
            if (sq / 8) == start_rank && (all & (1 << to2)) == 0 {
                mosse.push(Mossa::new(sq, to2, MoveFlag::DoublePawnPush, None));
            }
        }

        // CATTURE
        let attacks = crate::attacks::pawn_attacks(sq, us);
        let mut caps = attacks & their_pieces;
        while caps != 0 {
            let t = caps.trailing_zeros() as usize;
            caps &= caps - 1;
            add_capture_move(sq, t, prom_rank, &mut mosse);
        }

        // EP
        if let Some(ep) = s.ep_square {
            if (attacks & (1 << ep)) != 0 {
                mosse.push(Mossa::new(sq, ep, MoveFlag::EnPassant, None));
            }
        }
    }

    // ------------------------
    //    CAVALLI, ALFIERI,
    //    TORRI, REGINE, RE
    // ------------------------
    for p in [
        Pezzo::Cavallo,
        Pezzo::Alfiere,
        Pezzo::Torre,
        Pezzo::Regina,
        Pezzo::Re,
    ] {
        let mut bb = s.pezzi[p.indice()] & our_pieces;

        while bb != 0 {
            let sq = bb.trailing_zeros() as usize;
            bb &= bb - 1;

            // AGGIORNATO: Usa le nuove funzioni Magic Bitboards
            let attacks = match p {
                Pezzo::Cavallo => crate::attacks::knight_attacks(sq),
                Pezzo::Alfiere => crate::attacks::bishop_attacks(sq, all),  // Magic!
                Pezzo::Torre   => crate::attacks::rook_attacks(sq, all),    // Magic!
                Pezzo::Regina  => crate::attacks::queen_attacks(sq, all),   // Magic!
                Pezzo::Re      => crate::attacks::king_attacks(sq),
                _ => 0,
            };

            // QUIET
            let mut quiet = attacks & !all;
            while quiet != 0 {
                let t = quiet.trailing_zeros() as usize;
                quiet &= quiet - 1;
                mosse.push(Mossa::new(sq, t, MoveFlag::None, None));
            }

            // CAPTURE
            let mut cap = attacks & their_pieces;
            while cap != 0 {
                let t = cap.trailing_zeros() as usize;
                cap &= cap - 1;
                mosse.push(Mossa::new(sq, t, MoveFlag::Capture, None));
            }
        }
    }

    // ------------------------
    //       ARROCCO
    // ------------------------
    genera_arrocco(s, &mut mosse, all);

    mosse
}

// --------------------------------------------
//               ARROCCO
// --------------------------------------------
fn genera_arrocco(s: &Scacchiera, mosse: &mut Vec<Mossa>, all: u64) {
    let us = s.turno;
    if s.in_scacco() {
        return;
    }

    if us == Colore::Bianco {
        // Corto
        if (s.castling & 1) != 0 && (all & 0x60) == 0 {
            if !crate::attacks::square_attacked(s, 5, Colore::Nero)
                && !crate::attacks::square_attacked(s, 6, Colore::Nero)
            {
                mosse.push(Mossa::new(4, 6, MoveFlag::Castle, None));
            }
        }

        // Lungo
        if (s.castling & 2) != 0 && (all & 0x0E) == 0 {
            if !crate::attacks::square_attacked(s, 3, Colore::Nero)
                && !crate::attacks::square_attacked(s, 2, Colore::Nero)
            {
                mosse.push(Mossa::new(4, 2, MoveFlag::Castle, None));
            }
        }
    } else {
        // Corto
        if (s.castling & 4) != 0 && (all & 0x6000000000000000) == 0 {
            if !crate::attacks::square_attacked(s, 61, Colore::Bianco)
                && !crate::attacks::square_attacked(s, 62, Colore::Bianco)
            {
                mosse.push(Mossa::new(60, 62, MoveFlag::Castle, None));
            }
        }

        // Lungo
        if (s.castling & 8) != 0 && (all & 0x0E00000000000000) == 0 {
            if !crate::attacks::square_attacked(s, 59, Colore::Bianco)
                && !crate::attacks::square_attacked(s, 58, Colore::Bianco)
            {
                mosse.push(Mossa::new(60, 58, MoveFlag::Castle, None));
            }
        }
    }
}

// --------------------------------------------
//      PROMOZIONI E CATTURE PEDONE
// --------------------------------------------
fn add_pawn_move(from: usize, to: usize, prom_rank: usize, list: &mut Vec<Mossa>) {
    let rank = to / 8;
    if rank == prom_rank {
        for p in [Pezzo::Regina, Pezzo::Torre, Pezzo::Alfiere, Pezzo::Cavallo] {
            list.push(Mossa::new(from, to, MoveFlag::Promotion, Some(p)));
        }
    } else {
        list.push(Mossa::new(from, to, MoveFlag::None, None));
    }
}

fn add_capture_move(from: usize, to: usize, prom_rank: usize, list: &mut Vec<Mossa>) {
    let rank = to / 8;
    if rank == prom_rank {
        for p in [Pezzo::Regina, Pezzo::Torre, Pezzo::Alfiere, Pezzo::Cavallo] {
            list.push(Mossa::new(from, to, MoveFlag::PromotionCapture, Some(p)));
        }
    } else {
        list.push(Mossa::new(from, to, MoveFlag::Capture, None));
    }
}

// -------------------------------------------------------
//           ORDINAMENTO MOSSE (MOLTO IMPORTANTE)
// -------------------------------------------------------
pub fn ordina_mosse(
    mosse: &mut Vec<Mossa>,
    board: &Scacchiera,
    tt_move: Mossa,
    depth: i32,
    info: &SearchInfo,
) {
    mosse.sort_by_cached_key(|m| -score_move(m, board, tt_move, depth, info));
}

fn score_move(
    m: &Mossa,
    board: &Scacchiera,
    tt_move: Mossa,
    depth: i32,
    info: &SearchInfo,
) -> i32 {
    // 1. TT move = priorità assoluta
    if m.data == tt_move.data && !m.is_null() {
        return 100000;
    }

    // 2. Catture (MVV-LVA) - Possiamo ottimizzare con tabella precalcolata
    if m.is_cattura() {
        let att_idx = board.pezzo_in(m.da()).unwrap_or(0);
        let vic_val = if m.move_flag() == MoveFlag::EnPassant {
            100
        } else {
            board
                .pezzo_in(m.a())
                .map(|p| Pezzo::from_index(p).valore())
                .unwrap_or(0)
        };
        let att_val = Pezzo::from_index(att_idx).valore();

        return 50000 + vic_val * 10 - att_val;
    }

    // 3. Promozioni
    if m.is_promozione() {
        return 40000 + m.pezzo_promosso().unwrap().valore();
    }

    // 4. Killer moves
    let d = board.ply as usize;
    if d < 64 {
        if m.data == info.killer[d][0].data {
            return 9000;
        }
        if m.data == info.killer[d][1].data {
            return 8000;
        }
    }

    // 5. History + PST
    let p_idx = board.pezzo_in(m.da()).unwrap_or(0);
    let to = m.a();

    let table_idx = if board.turno == Colore::Bianco {
        to
    } else {
        to ^ 56
    };

    let pst_bonus = match p_idx {
        0 => PST_PAWN[table_idx],
        1 => PST_KNIGHT[table_idx],
        2 => PST_BISHOP[table_idx],
        3 => PST_ROOK[table_idx],
        4 => PST_QUEEN[table_idx],
        5 => PST_KING[table_idx],
        _ => 0,
    };

    let history = info.history[board.turno.indice()][m.da()][m.a()] / 128;

    1000 + pst_bonus + history
}