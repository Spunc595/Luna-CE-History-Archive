use crate::board::{Scacchiera, Pezzo, Colore, Casella}; // Tolto Bitboard inutile
use crate::nnue::NNUE; // <--- CORRETTO: Importa da nnue.rs

const MG_VAL: [i32; 6] = [ 82, 337, 365, 477, 1025, 0 ];
const EG_VAL: [i32; 6] = [ 94, 281, 297, 512,  936, 0 ];

// [ ... Qui sotto lascia tutte le tabelle COSTANTI (MG_TABLE, EG_TABLE) invariate ... ]
// Per brevità non le ricopio, ma DEVONO ESSERCI!
// Se le hai cancellate, recuperale dal codice precedente.

// Placeholder per le costanti se non le hai copiate (Toglili se hai già il codice completo)
#[rustfmt::skip] const MG_TABLE: [[i32; 64]; 6] = [[0;64];6]; 
#[rustfmt::skip] const EG_TABLE: [[i32; 64]; 6] = [[0;64];6];
const PASSED_PAWN_BONUS: [i32; 8] = [0; 8]; 
// FINE PLACEHOLDER -> Rimetti le tue tabelle vere!

pub fn valuta_posizione(s: &Scacchiera) -> i32 {
    // 1. TENTATIVO NNUE
    // Ora il compilatore sa cos'è NNUE e NnueNetwork
    if let Some(net) = &*NNUE {
        return net.evaluate(s);
    }

    // 2. FALLBACK CLASSICO
    let mut mg = [0, 0];
    let mut eg = [0, 0];
    let mut phase = 0;

    let white_pawns = s.bitboard_pezzo_colore(Pezzo::Pedone, Colore::Bianco);
    let black_pawns = s.bitboard_pezzo_colore(Pezzo::Pedone, Colore::Nero);

    for sq in 0..64 {
        if let Some((p, c)) = s.pezzo_su_casella(Casella(sq)) {
            let pi = p.indice();
            let ci = c.indice();
            
            phase += match p {
                Pezzo::Cavallo | Pezzo::Alfiere => 1,
                Pezzo::Torre => 2,
                Pezzo::Regina => 4,
                _ => 0,
            };

            let pst_idx = if c == Colore::Bianco { sq ^ 56 } else { sq };
            
            // ATTENZIONE: Assicurati che MG_TABLE e EG_TABLE siano definiti sopra!
            mg[ci] += MG_VAL[pi] + MG_TABLE[pi][pst_idx];
            eg[ci] += EG_VAL[pi] + EG_TABLE[pi][pst_idx];

            if p == Pezzo::Pedone {
                if is_passed_pawn(sq, c, white_pawns, black_pawns) {
                    let rank = if c == Colore::Bianco { sq / 8 } else { 7 - (sq / 8) };
                    mg[ci] += PASSED_PAWN_BONUS[rank]; 
                    eg[ci] += PASSED_PAWN_BONUS[rank] * 2; 
                }
            }
        }
    }

    let mut king_safety = [0, 0];
    for c in [Colore::Bianco, Colore::Nero] {
        king_safety[c.indice()] = valuta_sicurezza_re(s, c);
    }

    mg[0] += king_safety[0];
    mg[1] += king_safety[1];

    let p = phase.min(24);
    let mg_d = mg[0] - mg[1];
    let eg_d = eg[0] - eg[1];
    let score = (mg_d * p + eg_d * (24 - p)) / 24;

    if s.turno() == Colore::Bianco { score } else { -score }
}

fn valuta_sicurezza_re(s: &Scacchiera, c: Colore) -> i32 {
    let king_bb = s.bitboard_pezzo_colore(Pezzo::Re, c);
    if king_bb == 0 { return 0; }
    
    let k_sq = king_bb.trailing_zeros() as usize;
    let mut penalty = 0;
    
    let is_kingside = if c == Colore::Bianco { k_sq == 6 || k_sq == 7 } else { k_sq == 62 || k_sq == 63 };
    let is_queenside = if c == Colore::Bianco { k_sq == 1 || k_sq == 2 } else { k_sq == 57 || k_sq == 58 };

    if is_kingside {
        let (f, g, h) = if c == Colore::Bianco { (13, 14, 15) } else { (53, 54, 55) };
        penalty += valuta_scudo(s, c, f, g, h);
    } else if is_queenside {
        let (a, b, c_sq) = if c == Colore::Bianco { (8, 9, 10) } else { (48, 49, 50) };
        penalty += valuta_scudo(s, c, a, b, c_sq);
    }

    penalty
}

fn valuta_scudo(s: &Scacchiera, c: Colore, sq1: usize, sq2: usize, sq3: usize) -> i32 {
    let pawns = s.bitboard_pezzo_colore(Pezzo::Pedone, c);
    let mut penalty = 0;
    if (pawns & (1u64 << sq2)) == 0 { penalty -= 50; }
    if (pawns & (1u64 << sq1)) == 0 { penalty -= 20; }
    if (pawns & (1u64 << sq3)) == 0 { penalty -= 20; }
    penalty
}

fn is_passed_pawn(sq: usize, c: Colore, white_pawns: u64, black_pawns: u64) -> bool {
    let file = sq % 8;
    let rank = sq / 8;
    let enemy_pawns = if c == Colore::Bianco { black_pawns } else { white_pawns };
    let mut mask = 0u64;
    
    if c == Colore::Bianco {
        for r in (rank + 1)..8 {
            mask |= 1u64 << (r * 8 + file);
            if file > 0 { mask |= 1u64 << (r * 8 + (file - 1)); }
            if file < 7 { mask |= 1u64 << (r * 8 + (file + 1)); }
        }
    } else {
        for r in 0..rank {
            mask |= 1u64 << (r * 8 + file);
            if file > 0 { mask |= 1u64 << (r * 8 + (file - 1)); }
            if file < 7 { mask |= 1u64 << (r * 8 + (file + 1)); }
        }
    }
    (enemy_pawns & mask) == 0
}