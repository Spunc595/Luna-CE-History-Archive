use crate::board::{Scacchiera, Pezzo, Colore, Bitboard, Casella};

// ============================================
// COSTANTI MATERIALE & PeSTO
// ============================================

const MG_VALUE: [i32; 6] = [ 82, 337, 365, 477, 1025, 0];
const EG_VALUE: [i32; 6] = [ 94, 281, 297, 512,  936, 0];

// Bonus per i Pedoni Passati (Rank 0-7, anche se 0 e 7 non usati per pedoni)
const PASSED_PAWN_BONUS: [i32; 8] = [ 0, 5, 10, 20, 35, 60, 100, 0 ];

#[rustfmt::skip]
const PESTO_TABLES: [[[i32; 64]; 6]; 2] = [
    // MIDDLEGAME (MG)
    [
        [ 0,   0,   0,   0,   0,   0,   0,   0,  98, 134,  61,  95,  68, 126,  34, -11,  -6,   7,  26,  31,  65,  56,  25, -20, -14,  13,   6,  21,  23,  12,  17, -23, -27,  -2,  -5,  12,  17,   6,  10, -25, -26,  -4,  -4, -10,   3,   3,  33, -12, -35,  -1, -20, -23, -15,  24,  38, -22,   0,   0,   0,   0,   0,   0,   0,   0], // Pedone
        [-167, -89, -34, -49,  61, -97, -15, -107, -73, -41,  72,  36,  23,  62,   7, -17, -47,  60,  37,  65,  84, 129,  73,  44,  -9,  17,  19,  53,  37,  69,  18,  22, -13,   4,  16,  13,  28,  19,  21,  -8, -23,  -9,  12,  10,  19,  17,  25, -16, -29, -53, -12,  -3,  -1,  18, -14, -19, -105, -21, -58, -33, -17, -28, -19, -23], // Cavallo
        [ -29,   4, -82, -37, -25, -42,   7,  -8, -26,  16, -18, -13,  30,  59,  18, -47, -16,  37,  43,  40,  35,  50,  37,  -2,  -4,   5,  19,  50,  37,  37,   7,  -2,  -6,  13,  13,  26,  34,  12,  10,   4,   0,  15,  15,  15,  14,  27,  18,  10,   4,  15,  16,   0,   7,  21,  33,   1, -33,  -3, -14, -21, -13, -12, -39, -21], // Alfiere
        [  32,  42,  32,  51,  63,   9,  31,  43,  27,  32,  58,  62,  80,  67,  26,  44,  -5,  19,  26,  36,  17,  45,  61,  16, -24, -11,   7,  26,  24,  35,  -8, -20, -36, -26, -12,  -1,   9,  -7,   6, -23, -45, -25, -16, -17,   3,   0,  -5, -33, -44, -16, -20,  -9,  -1,  11,  -6, -71, -19, -13,   1,  17,  16,   7, -37, -26], // Torre
        [ -28,   0,  29,  12,  59,  44,  43,  45, -24, -39,  -5,   1, -16,  57,  28,  54, -13, -17,   7,   8,  29,  56,  47,  57, -27, -27, -16, -16,  -1,  17,  -2,   1,  -9, -26, -28, -10,  -2, -11,  33, -10, -14,   2, -11,  -2,  -5,   2,  14,   5, -35,  -8,  11,   2,   8,  15,  -3,   1,  -1, -18,  -9,  10, -15, -25, -31, -50], // Regina
        [ -65,  23,  16, -15, -56, -34,   2,  13,  29,  -1, -20,  -7,  -8,  -4, -38, -29,  -9,  24,   2, -16, -20,   6,  22, -22, -17, -20, -12, -27, -30, -25, -14, -36, -49,  -1, -27, -39, -46, -44, -33, -51, -14, -14, -22, -46, -44, -30, -15, -27,   1,   7,  -8, -64, -43, -16,   9,   8, -15,  36,  12, -54,   8, -28,  24,  14], // Re
    ],
    // ENDGAME (EG)
    [
        [ 0,   0,   0,   0,   0,   0,   0,   0, 178, 173, 158, 134, 147, 132, 165, 187,  94, 100,  85,  67,  56,  53,  82,  84,  32,  24,  13,   5,  -2,   4,  17,  17,  13,   9,  -3,  -7,  -7,  -8,   3,  -1,   4,   7,  -6,   1,   0,  -5,  -1,  -8,  13,   8,   8,  10,  13,   0,   2,  -7,   0,   0,   0,   0,   0,   0,   0,   0], // Pedone
        [ -58, -38, -13, -28, -31, -27, -63, -99, -25,  -8, -25,  -2,  -9, -25, -24, -52, -24, -20,  10,   9,  -1,  -9, -19, -41, -17,   3,  22,  22,  22,  11,   8, -18, -18,  -6,  16,  25,  16,  17,   4, -18, -23,  -3,  -1,  15,  10,  -3, -20, -22, -42, -20, -10,  -5,  -2, -20, -23, -44, -29, -51, -23, -15, -22, -18, -50, -64], // Cavallo
        [ -14, -21, -11,  -8,  -7,  -9, -17, -24,  -8,  -4,   7, -12,  -3, -13,  -4, -14,   2,  -8,   0,  -1,  -2,   6,   0,   4,  -3,   9,  12,   9,  14,  10,   3,   2,  -6,   3,  13,  19,   7,  10,  -3,  -9, -12,  -3,   5,  10,   5,   6,   0,  -7, -15, -10, -10,  -5,  -4,   0,  -8, -23, -23,  -9, -23,  -5,  -9, -16,  -5, -17], // Alfiere
        [  13,  10,  18,  15,  12,  12,   8,   5,  11,  13,  13,  11,  -3,   3,   8,   3,   7,   7,   7,   5,   4,  -3,  -5,  -3,   4,   3,  13,   1,   2,   1,  -1,   2,   3,   5,   8,   4,  -5,  -6,  -8, -11,  -4,   0,  -5,  -1,  -7, -12,  -8, -16,  -6,  -6,   0,   2,  -9,  -9, -11,  -3,  -9,   2,   3,  -1,  -5, -13,   4, -20], // Torre
        [  -9,  22,  22,  27,  27,  19,  10,  20, -17,  20,  32,  41,  58,  25,  30,   0, -20,   6,   9,  49,  47,  35,  19,   9,   3,  22,  24,  45,  57,  40,  57,  36, -18,  28,  19,  47,  31,  34,  39,  23, -16, -27,  15,   6,   9,  17,  10,   5, -22, -23, -30, -16, -16,  23,   0, -36, -14,  -5, -15, -10, -10, -10, -10,  -2], // Regina
        [ -74, -35, -18, -18, -11,  15,   4, -17, -12,  17,  14,  17,  17,  38,  23,  11,  10,  17,  23,  15,  20,  45,  44,  13,  -8,  22,  24,  27,  26,  33,  26,   3, -18,  -4,  21,  24,  27,  23,   9, -11, -19,  -3,  11,  21,  23,  16,   7,  -9, -27, -11,   4,  13,  14,   4,  -5, -17, -53, -34, -21, -11, -28, -14, -24, -43], // Re
    ]
];

// Phase weights per Tapered Eval
const PHASE_WEIGHTS: [i32; 6] = [0, 1, 1, 2, 4, 0];
const MAX_PHASE: i32 = 24;

// Maschere per i pedoni passati (pre-calcolate mentalmente o runtime)
fn get_passed_pawn_mask(sq: usize, col: Colore) -> Bitboard {
    let mut mask = 0u64;
    let file = sq % 8;
    let rank = sq / 8;
    
    // File A, B, C...
    let mut files_bb = 0u64;
    files_bb |= 0x0101010101010101 << file; // File centrale
    if file > 0 { files_bb |= 0x0101010101010101 << (file - 1); } // File sx
    if file < 7 { files_bb |= 0x0101010101010101 << (file + 1); } // File dx
    
    if col == Colore::Bianco {
        // Tutte le caselle DAVANTI al pedone
        let ranks_ahead = !((1u64 << ((rank + 1) * 8)) - 1); 
        mask = files_bb & ranks_ahead;
    } else {
        // Tutte le caselle DAVANTI al pedone (verso il basso)
        let ranks_ahead = (1u64 << (rank * 8)) - 1; 
        mask = files_bb & ranks_ahead;
    }
    mask
}

pub fn valuta_posizione(s: &Scacchiera) -> i32 {
    let mut mg = [0; 2];
    let mut eg = [0; 2];
    let mut phase = 0;

    let occ = s.occupazione();
    let mut kings = [0usize; 2]; // Per tracciare la posizione dei Re

    // 1. Materiale e PeSTO
    for color_idx in 0..2 {
        let us = if color_idx == 0 { Colore::Bianco } else { Colore::Nero };
        let enemy = us.opposto();
        let enemy_pawns = s.bitboard_pezzo_colore(Pezzo::Pedone, enemy);

        for p_type in [Pezzo::Pedone, Pezzo::Cavallo, Pezzo::Alfiere, Pezzo::Torre, Pezzo::Regina, Pezzo::Re] {
            let mut bb = s.bitboard_pezzo_colore(p_type, us);
            let p_idx = p_type.indice();

            while bb != 0 {
                let sq = bb.trailing_zeros() as usize;
                bb &= bb - 1;

                if p_type == Pezzo::Re { kings[color_idx] = sq; }

                // Flip per il Nero
                let pesto_sq = if us == Colore::Bianco { sq ^ 56 } else { sq };
                
                let mut bonus_mg = 0;
                let mut bonus_eg = 0;

                // --- VALUTAZIONE PEDONI PASSATI ---
                if p_type == Pezzo::Pedone {
                    let mask = get_passed_pawn_mask(sq, us);
                    if (mask & enemy_pawns) == 0 {
                        // È passato!
                        let rank = if us == Colore::Bianco { sq / 8 } else { 7 - (sq / 8) };
                        bonus_eg += PASSED_PAWN_BONUS[rank];
                        // Bonus extra se il pedone è difeso o se il re nemico è lontano (semplificato)
                        bonus_mg += PASSED_PAWN_BONUS[rank] / 2;
                    }
                }

                mg[color_idx] += MG_VALUE[p_idx] + PESTO_TABLES[0][p_idx][pesto_sq] + bonus_mg;
                eg[color_idx] += EG_VALUE[p_idx] + PESTO_TABLES[1][p_idx][pesto_sq] + bonus_eg;
                phase += PHASE_WEIGHTS[p_idx];
            }
        }
    }

    // 2. Tapered Eval Calculation
    let p = phase.min(MAX_PHASE);
    let mg_score = mg[0] - mg[1];
    let eg_score = eg[0] - eg[1];
    
    let mut final_score = (mg_score * p + eg_score * (MAX_PHASE - p)) / MAX_PHASE;

    // 3. Mop-Up Evaluation (Solo se siamo in netto vantaggio nel finale)
    // Se non ci sono pedoni, aiuta il motore a dare matto spingendo il Re nemico all'angolo.
    if p < 5 { // Endgame profondo
        let score_abs = final_score.abs();
        if score_abs > 500 { // Vantaggio significativo
            let winning_side = if final_score > 0 { 0 } else { 1 };
            let losing_side = 1 - winning_side;
            
            let wk = kings[winning_side];
            let lk = kings[losing_side];

            // Distanza dal centro (vogliamo che il perdente sia lontano dal centro)
            let center_dist = distance_from_center(lk);
            // Distanza tra i Re (vogliamo avvicinare il nostro Re per aiutare il matto)
            let king_dist = distance_between(wk, lk);

            let mop_up = (center_dist * 10) + (14 - king_dist) * 4;
            
            if final_score > 0 { final_score += mop_up; }
            else { final_score -= mop_up; }
        }
    }

    if s.turno() == Colore::Bianco { final_score + 10 } else { -final_score + 10 }
}

fn distance_from_center(sq: usize) -> i32 {
    let r = (sq / 8) as i32;
    let f = (sq % 8) as i32;
    // Distanza da 3.5 (centro tra 3 e 4)
    let dr = (2 * r - 7).abs();
    let df = (2 * f - 7).abs();
    dr + df
}

fn distance_between(sq1: usize, sq2: usize) -> i32 {
    let r1 = (sq1 / 8) as i32; let f1 = (sq1 % 8) as i32;
    let r2 = (sq2 / 8) as i32; let f2 = (sq2 % 8) as i32;
    (r1 - r2).abs().max((f1 - f2).abs())
}