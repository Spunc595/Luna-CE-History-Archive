use crate::board::{Bitboard, Colore, Scacchiera, Pezzo};
use std::sync::OnceLock;

// -----------------------------------------------------
//              TABELLE DI ATTACCO STATICHE
//        (Pawn, Knight, King) – NON SLIDER
// -----------------------------------------------------

pub struct AttackTables {
    pub pawn: [[Bitboard; 64]; 2],
    pub knight: [Bitboard; 64],
    pub king: [Bitboard; 64],
}

static TABLES: OnceLock<AttackTables> = OnceLock::new();

#[inline(always)]
pub fn tables() -> &'static AttackTables {
    TABLES.get_or_init(|| {
        let mut t = AttackTables {
            pawn: [[0; 64]; 2],
            knight: [0; 64],
            king: [0; 64],
        };
        init_tables(&mut t);
        t
    })
}

#[inline(always)]
pub fn pawn_attacks(sq: usize, side: Colore) -> Bitboard {
    tables().pawn[side.indice()][sq]
}

#[inline(always)]
pub fn knight_attacks(sq: usize) -> Bitboard {
    tables().knight[sq]
}

#[inline(always)]
pub fn king_attacks(sq: usize) -> Bitboard {
    tables().king[sq]
}

// -----------------------------------------------------
//                 ATTACCHI SLIDER (DINAMICI)
// -----------------------------------------------------

#[inline(always)]
pub fn bishop_attacks(sq: usize, occ: Bitboard) -> Bitboard {
    let mut atk = 0u64;
    let r = (sq / 8) as i32;
    let f = (sq % 8) as i32;

    for &(dr, df) in &[(1, 1), (1, -1), (-1, 1), (-1, -1)] {
        let mut nr = r + dr;
        let mut nf = f + df;

        while nr >= 0 && nr < 8 && nf >= 0 && nf < 8 {
            let bit = 1u64 << (nr * 8 + nf);
            atk |= bit;
            if (occ & bit) != 0 {
                break;
            }
            nr += dr;
            nf += df;
        }
    }

    atk
}

#[inline(always)]
pub fn rook_attacks(sq: usize, occ: Bitboard) -> Bitboard {
    let mut atk = 0u64;
    let r = (sq / 8) as i32;
    let f = (sq % 8) as i32;

    for &(dr, df) in &[(1, 0), (-1, 0), (0, 1), (0, -1)] {
        let mut nr = r + dr;
        let mut nf = f + df;

        while nr >= 0 && nr < 8 && nf >= 0 && nf < 8 {
            let bit = 1u64 << (nr * 8 + nf);
            atk |= bit;
            if (occ & bit) != 0 {
                break;
            }
            nr += dr;
            nf += df;
        }
    }

    atk
}

#[inline(always)]
pub fn queen_attacks(sq: usize, occ: Bitboard) -> Bitboard {
    bishop_attacks(sq, occ) | rook_attacks(sq, occ)
}

// -----------------------------------------------------
//        square_attacked(): controlla se sq è attaccata
// -----------------------------------------------------
pub fn square_attacked(board: &Scacchiera, sq: usize, side: Colore) -> bool {
    let occ = board.occupazione();
    let pieces = board.colori[side.indice()];

    // Pedoni (attacco inverso)
    if (pawn_attacks(sq, side.opposto())
        & board.pezzi[Pezzo::Pedone.indice()]
        & pieces)
        != 0
    {
        return true;
    }

    // Cavalli
    if (knight_attacks(sq)
        & board.pezzi[Pezzo::Cavallo.indice()]
        & pieces)
        != 0
    {
        return true;
    }

    // Re
    if (king_attacks(sq)
        & board.pezzi[Pezzo::Re.indice()]
        & pieces)
        != 0
    {
        return true;
    }

    // Alfiere / Regina (diagonali)
    if (bishop_attacks(sq, occ)
        & (board.pezzi[Pezzo::Alfiere.indice()] | board.pezzi[Pezzo::Regina.indice()])
        & pieces)
        != 0
    {
        return true;
    }

    // Torre / Regina (linee)
    if (rook_attacks(sq, occ)
        & (board.pezzi[Pezzo::Torre.indice()] | board.pezzi[Pezzo::Regina.indice()])
        & pieces)
        != 0
    {
        return true;
    }

    false
}

// -----------------------------------------------------
//      INIZIALIZZAZIONE DELLE ATTACK TABLES STATICHE
// -----------------------------------------------------
fn init_tables(t: &mut AttackTables) {
    for sq in 0..64 {
        let r = (sq / 8) as i32;
        let f = (sq % 8) as i32;

        let b = 1u64 << sq;

        // PAWN WHITE (attacca NE/NW)
        if r < 7 {
            if f > 0 {
                t.pawn[0][sq] |= b << 7;
            }
            if f < 7 {
                t.pawn[0][sq] |= b << 9;
            }
        }

        // PAWN BLACK (attacca SE/SW)
        if r > 0 {
            if f > 0 {
                t.pawn[1][sq] |= b >> 9;
            }
            if f < 7 {
                t.pawn[1][sq] |= b >> 7;
            }
        }

        // KNIGHT
        for &(dr, df) in &[
            (2, 1),
            (2, -1),
            (-2, 1),
            (-2, -1),
            (1, 2),
            (1, -2),
            (-1, 2),
            (-1, -2),
        ] {
            let nr = r + dr;
            let nf = f + df;
            if nr >= 0 && nr < 8 && nf >= 0 && nf < 8 {
                t.knight[sq] |= 1u64 << (nr * 8 + nf);
            }
        }

        // KING
        for dr in -1..=1 {
            for df in -1..=1 {
                if dr == 0 && df == 0 {
                    continue;
                }
                let nr = r + dr;
                let nf = f + df;
                if nr >= 0 && nr < 8 && nf >= 0 && nf < 8 {
                    t.king[sq] |= 1u64 << (nr * 8 + nf);
                }
            }
        }
    }
}