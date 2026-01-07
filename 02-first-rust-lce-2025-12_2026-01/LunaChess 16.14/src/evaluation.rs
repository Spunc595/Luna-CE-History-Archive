use crate::board::{Scacchiera, Pezzo, Colore};

// --- PESI MATERIALE (Middlegame, Endgame) ---
// Notare che l'Alfiere vale un po' più del Cavallo nel finale
const MATERIAL_MG: [i32; 6] = [82, 337, 365, 477, 1025, 0]; 
const MATERIAL_EG: [i32; 6] = [94, 281, 297, 512, 936, 0];

// --- TABELLE PEZZO-CASELLA (PSQT) ---
// I valori sono visti dal punto di vista del BIANCO. 
// Per il nero si riflette la scacchiera (flip verticale).

#[rustfmt::skip]
const PAWN_MG: [i32; 64] = [
     0,   0,   0,   0,   0,   0,   0,   0,
    98, 134,  61,  95,  68, 126,  34, -11,
    -6,   7,  26,  31,  65,  56,  25, -20,
   -14,  13,   6,  21,  23,  12,  17, -23,
   -27,  -2,  -5,  12,  17,   6,  10, -25,
   -26,  -4,  -4, -10,   3,   3,  33, -12,
   -35,  -1, -20, -23, -15,  24,  38, -22,
     0,   0,   0,   0,   0,   0,   0,   0,
];

#[rustfmt::skip]
const PAWN_EG: [i32; 64] = [
     0,   0,   0,   0,   0,   0,   0,   0,
   178, 173, 158, 134, 147, 132, 165, 187,
    94, 100,  85,  67,  56,  53,  82,  84,
    32,  24,  13,   5,  -2,   4,  17,  19,
    13,   9,  -3,  -7,  -7,  -8,   3,  -1,
     4,   7,  -6,   1,   0,  -5,  -1,  -8,
    13,   8,   8,  10,  13,   0,   2,  -7,
     0,   0,   0,   0,   0,   0,   0,   0,
];

#[rustfmt::skip]
const KNIGHT_MG: [i32; 64] = [
   -167, -89, -34, -49,  61, -97, -15, -107,
    -73, -41,  72,  36,  23,  62,   7,  -17,
    -47,  60,  37,  65,  84, 129,  73,   44,
     -9,  17,  19,  53,  37,  69,  18,   22,
    -13,   4,  16,  13,  28,  19,  21,   -8,
    -23,  -9,  12,  10,  19,  17,  25,  -16,
    -29, -53, -12,  -3,  -1,  18, -14,  -19,
   -105, -21, -58, -33, -17, -28, -19,  -23,
];

#[rustfmt::skip]
const KNIGHT_EG: [i32; 64] = [
    -58, -38, -13, -28, -31, -27, -63, -99,
    -25,  -8, -25,  -2,  -9, -25, -24, -52,
    -24, -20,  10,   9,  -1,  -9, -19, -41,
    -17,   3,  22,  22,  22,  11,   8, -18,
    -18,  -6,  16,  25,  16,  17,   4, -18,
    -23,  -3,  -1,  15,  10,  -3, -20, -22,
    -42, -20, -10,  -5,  -2, -20, -23, -44,
    -29, -51, -23, -15, -22, -18, -50, -64,
];

#[rustfmt::skip]
const BISHOP_MG: [i32; 64] = [
    -29,   4, -82, -37, -25, -42,   7,  -8,
    -26,  16, -18, -13,  30,  59,  18, -47,
    -16,  37,  43,  40,  35,  50,  37,  -2,
     -4,   5,  19,  50,  37,  37,   7,  -2,
     -6,  13,  13,  26,  34,  12,  10,   4,
      0,  15,  15,  15,  14,  27,  18,  10,
      4,  15,  16,   0,   7,  21,  33,   1,
    -33,  -3, -14, -21, -13, -12, -39, -21,
];

#[rustfmt::skip]
const BISHOP_EG: [i32; 64] = [
    -14, -21, -11,  -8,  -7,  -9, -17, -24,
     -8,  -4,   7, -12, -36, -13,  -5, -16,
     -4,  -8,   8, -16,  -2, -14,  -7, -24,
     -1,   4,   7,  13,   8,   6,  -2, -13,
      4,   3,  13,   1,   2,  -5,   5,  -9,
      7,   4,  12,   9,   8,   5,   4, -10,
     -9,   1,  13,  10,  17,   5,   1,  -7,
    -17, -18, -18, -20, -12,  -7, -18, -32,
];

#[rustfmt::skip]
const ROOK_MG: [i32; 64] = [
     32,  42,  32,  51,  63,   9,  31,  43,
     27,  32,  58,  62,  80,  67,  26,  44,
     -5,  19,  26,  36,  17,  45,  61,  16,
    -24, -11,   7,  26,  24,  35,  -8, -20,
    -36, -26, -12,  -1,   9,  -7,   6, -23,
    -45, -25, -16, -17,   3,   0,  -5, -33,
    -44, -16, -20,  -9,  -1,  11,  -6, -71,
    -19, -13,   1,  17,  16,   7, -37, -26,
];

#[rustfmt::skip]
const ROOK_EG: [i32; 64] = [
     13,  10,  18,  15,  12,  12,   8,   5,
     11,  13,  13,  11,  -3,   3,   8,   3,
      7,   7,   7,   5,   4,  -3,  -5,  -3,
      4,   3,  13,   1,   2,   1,  -1,   2,
      3,   5,   8,   4,  -5,  -6,  -8, -11,
     -4,   0,  -5,  -1,  -7, -12,  -8, -16,
     -6,  -6,   0,   2, - 9,  -9, -11,  -3,
     -9,   2,   3,  -1,  -5, -13,   4, -20,
];

#[rustfmt::skip]
const QUEEN_MG: [i32; 64] = [
    -28,   0,  29,  12,  59,  44,  43,  45,
    -24, -39,  -5,   1, -16,  57,  28,  54,
    -13, -17,   7,   8,  29,  56,  47,  57,
    -27, -27, -16, -16,  -1,  17,  -2,   1,
     -9, -26, - 9, -10,  -2,  -4,   3,  -3,
    -14,   2, -11,  -2,  -5,   2,  14,   5,
    -35,  -8,  11,   2,   8,  15,  -3,   1,
     -1, -18,  -9,  10, -15, -25, -31, -50,
];

#[rustfmt::skip]
const QUEEN_EG: [i32; 64] = [
     -9,  22,  22,  27,  27,  19,  10,  20,
    -17,  20,  32,  41,  58,  25,  30,   0,
    -20,   6,   9,  49,  47,  35,  19,   9,
      3,  22,  24,  45,  57,  40,  57,  36,
    -18,  28,  19,  47,  31,  34,  39,  23,
    -16, -27,  15,   6,   9,  17,  10,   5,
    -22, -23, -30, -16, -16, -23, -36, -32,
    -33, -28, -22, -43,  -5, -32, -20, -41,
];

#[rustfmt::skip]
const KING_MG: [i32; 64] = [
    -65,  23,  16, -15, -56, -34,   2,  13,
     29,  -1, -20,  -7,  -8,  -4, -38, -29,
     -9,  24,   2, -16, -20,   6,  22, -22,
    -17, -20, -12, -27, -30, -25, -14, -36,
    -49,  -1, -27, -39, -46, -44, -33, -51,
    -14, -14, -22, -46, -44, -30, -15, -27,
      1,   7,  -8, -64, -43, -16,   9,   8,
    -15,  36,  12, -54,   8, -28,  24,  14,
];

#[rustfmt::skip]
const KING_EG: [i32; 64] = [
    -74, -35, -18, -18, -11,  15,   4, -17,
    -12,  17,  14,  17,  17,  38,  23,  11,
     10,  17,  23,  15,  20,  45,  44,  13,
     -8,  22,  24,  27,  26,  33,  26,   3,
    -18,  -4,  21,  24,  27,  23,   9, -11,
    -19,  -3,  11,  21,  23,  16,   7,  -9,
    -27, -11,   4,  13,  14,   4,  -5, -17,
    -53, -34, -21, -11, -28, -14, -24, -43
];

// Pesi delle fasi (Pedone=0, Cavallo=1, ecc. per calcolare la fase del gioco)
const PHASE_WEIGHTS: [i32; 6] = [0, 1, 1, 2, 4, 0];
const TOTAL_PHASE: i32 = 24; // Massimo teorico (4N + 4B + 4R + 2Q)

pub fn evaluate_classical(board: &Scacchiera) -> i32 {
    let mut mg_score = 0;
    let mut eg_score = 0;
    let mut phase = TOTAL_PHASE;

    for p_type in 0..6 {
        let mut bb = board.pezzi[p_type];
        while bb != 0 {
            let sq = bb.trailing_zeros() as usize;
            bb &= bb - 1;

            let color = if (board.colori[Colore::Bianco.indice()] & (1u64 << sq)) != 0 {
                Colore::Bianco
            } else {
                Colore::Nero
            };

            // Indice per PSQT (se Nero, flippa verticalmente: sq ^ 56)
            let psqt_idx = if color == Colore::Bianco { sq ^ 56 } else { sq };

            let (mat_mg, mat_eg) = (MATERIAL_MG[p_type], MATERIAL_EG[p_type]);
            let (pst_mg, pst_eg) = match p_type {
                0 => (PAWN_MG[psqt_idx], PAWN_EG[psqt_idx]),
                1 => (KNIGHT_MG[psqt_idx], KNIGHT_EG[psqt_idx]),
                2 => (BISHOP_MG[psqt_idx], BISHOP_EG[psqt_idx]),
                3 => (ROOK_MG[psqt_idx], ROOK_EG[psqt_idx]),
                4 => (QUEEN_MG[psqt_idx], QUEEN_EG[psqt_idx]),
                5 => (KING_MG[psqt_idx], KING_EG[psqt_idx]),
                _ => (0, 0),
            };

            if color == Colore::Bianco {
                mg_score += mat_mg + pst_mg;
                eg_score += mat_eg + pst_eg;
            } else {
                mg_score -= mat_mg + pst_mg;
                eg_score -= mat_eg + pst_eg;
            }

            // Aggiorna Fase
            phase -= PHASE_WEIGHTS[p_type];
        }
    }

    // --- CORREZIONI STRATEGICHE ---
    
    // 1. Penalità Pedoni Doppiati (Doubled Pawns)
    // Semplificazione: se ci sono più pedoni sulla stessa colonna
    for i in 0..8 {
        let file_mask = 0x0101010101010101u64 << i;
        
        let wp = (board.pezzi[Pezzo::Pedone.indice()] & board.colori[Colore::Bianco.indice()] & file_mask).count_ones();
        if wp > 1 { mg_score -= 20 * (wp as i32 - 1); eg_score -= 30 * (wp as i32 - 1); }

        let bp = (board.pezzi[Pezzo::Pedone.indice()] & board.colori[Colore::Nero.indice()] & file_mask).count_ones();
        if bp > 1 { mg_score += 20 * (bp as i32 - 1); eg_score += 30 * (bp as i32 - 1); }
    }

    // 2. Bonus Coppia degli Alfieri (Bishop Pair)
    let w_bishops = (board.pezzi[Pezzo::Alfiere.indice()] & board.colori[Colore::Bianco.indice()]).count_ones();
    let b_bishops = (board.pezzi[Pezzo::Alfiere.indice()] & board.colori[Colore::Nero.indice()]).count_ones();
    
    if w_bishops >= 2 { mg_score += 30; eg_score += 50; }
    if b_bishops >= 2 { mg_score -= 30; eg_score -= 50; }

    // 3. Mobilità (Bonus per le mosse disponibili)
    // Per non rallentare troppo, usiamo un'approssimazione basata sul numero di mosse generabili pseudo-legali
    // Nota: questo è costoso, lo facciamo leggero o lo saltiamo se vogliamo max speed.
    // Qui lo omettiamo per mantenere 15M NPS, le PSQT migliorate fanno già il 90% del lavoro di posizionamento.

    // --- INTERPOLAZIONE FASE ---
    phase = phase.clamp(0, TOTAL_PHASE);
    let final_score = (mg_score * phase + eg_score * (TOTAL_PHASE - phase)) / TOTAL_PHASE;

    if board.turno == Colore::Bianco { final_score } else { -final_score }
}