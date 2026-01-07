use crate::board::{Scacchiera, Pezzo, Colore};

// --- PESI MATERIALE (MG = Middlegame, EG = Endgame) ---
const MATERIAL_MG: [i32; 6] = [82, 337, 365, 477, 1025, 0]; 
const MATERIAL_EG: [i32; 6] = [94, 281, 297, 512, 936, 0];

// --- TABELLE PEZZO-CASELLA (PSQT) ---
// Orientamento: Rank 1 (A1..H1) è indice 0..7.
// Il motore userà flip verticale per il Nero.

#[rustfmt::skip]
const PAWN_MG: [i32; 64] = [
      0,   0,   0,   0,   0,   0,   0,   0,
      5,  10,  10, -20, -20,  10,  10,   5, // Rank 2
      5,  -5, -10,   0,   0, -10,  -5,   5,
      0,   0,   0,  20,  20,   0,   0,   0,
      5,   5,  10,  25,  25,  10,   5,   5,
     10,  10,  20,  30,  30,  20,  10,  10,
     50,  50,  50,  50,  50,  50,  50,  50,
      0,   0,   0,   0,   0,   0,   0,   0,
];

#[rustfmt::skip]
const PAWN_EG: [i32; 64] = [
      0,   0,   0,   0,   0,   0,   0,   0,
     20,  20,  20,  20,  20,  20,  20,  20,
     25,  25,  25,  25,  25,  25,  25,  25, // Incentiva avanzamento
     30,  30,  30,  30,  30,  30,  30,  30,
     40,  40,  40,  40,  40,  40,  40,  40,
     50,  50,  50,  50,  50,  50,  50,  50,
     80,  80,  80,  80,  80,  80,  80,  80,
      0,   0,   0,   0,   0,   0,   0,   0,
];

#[rustfmt::skip]
const KNIGHT_MG: [i32; 64] = [
    -50, -40, -30, -30, -30, -30, -40, -50,
    -40, -20,   0,   0,   0,   0, -20, -40,
    -30,   0,  10,  15,  15,  10,   0, -30, // c3/f3 sono qui (indice 18 e 21 circa)
    -30,   5,  15,  20,  20,  15,   5, -30,
    -30,   0,  15,  20,  20,  15,   0, -30,
    -30,   5,  10,  15,  15,  10,   5, -30,
    -40, -20,   0,   5,   5,   0, -20, -40,
    -50, -40, -30, -30, -30, -30, -40, -50,
];

#[rustfmt::skip]
const KNIGHT_EG: [i32; 64] = [
    -50, -40, -30, -30, -30, -30, -40, -50,
    -40, -20,   0,   0,   0,   0, -20, -40,
    -30,   0,  10,  15,  15,  10,   0, -30,
    -30,   5,  15,  20,  20,  15,   5, -30,
    -30,   0,  15,  20,  20,  15,   0, -30,
    -30,   5,  10,  15,  15,  10,   5, -30,
    -40, -20,   0,   5,   5,   0, -20, -40,
    -50, -40, -30, -30, -30, -30, -40, -50,
];

#[rustfmt::skip]
const BISHOP_MG: [i32; 64] = [
    -20, -10, -10, -10, -10, -10, -10, -20,
    -10,   0,   0,   0,   0,   0,   0, -10,
    -10,   0,   5,  10,  10,   5,   0, -10,
    -10,   5,   5,  10,  10,   5,   5, -10,
    -10,   0,  10,  10,  10,  10,   0, -10,
    -10,  10,  10,  10,  10,  10,  10, -10, // Fianchetto support
    -10,   5,   0,   0,   0,   0,   5, -10,
    -20, -10, -10, -10, -10, -10, -10, -20,
];

#[rustfmt::skip]
const BISHOP_EG: [i32; 64] = [
    -20, -10, -10, -10, -10, -10, -10, -20,
    -10,   0,   0,   0,   0,   0,   0, -10,
    -10,   0,   5,  10,  10,   5,   0, -10,
    -10,   5,   5,  10,  10,   5,   5, -10,
    -10,   0,  10,  10,  10,  10,   0, -10,
    -10,  10,  10,  10,  10,  10,  10, -10,
    -10,   5,   0,   0,   0,   0,   5, -10,
    -20, -10, -10, -10, -10, -10, -10, -20,
];

#[rustfmt::skip]
const ROOK_MG: [i32; 64] = [
      0,   0,   0,   0,   0,   0,   0,   0,
      5,  10,  10,  10,  10,  10,  10,   5,
     -5,   0,   0,   0,   0,   0,   0,  -5,
     -5,   0,   0,   0,   0,   0,   0,  -5,
     -5,   0,   0,   0,   0,   0,   0,  -5,
     -5,   0,   0,   0,   0,   0,   0,  -5,
     -5,   0,   0,   0,   0,   0,   0,  -5,
      0,   0,   0,   5,   5,   0,   0,   0,
];

#[rustfmt::skip]
const ROOK_EG: [i32; 64] = [
      0,   0,   0,   0,   0,   0,   0,   0,
      5,  10,  10,  10,  10,  10,  10,   5,
     -5,   0,   0,   0,   0,   0,   0,  -5,
     -5,   0,   0,   0,   0,   0,   0,  -5,
     -5,   0,   0,   0,   0,   0,   0,  -5,
     -5,   0,   0,   0,   0,   0,   0,  -5,
     -5,   0,   0,   0,   0,   0,   0,  -5,
      0,   0,   0,   5,   5,   0,   0,   0,
];

#[rustfmt::skip]
const QUEEN_MG: [i32; 64] = [
    -20, -10, -10, -5, -5, -10, -10, -20,
    -10,   0,   0,  0,  0,   0,   0, -10,
    -10,   0,   5,  5,  5,   5,   0, -10,
     -5,   0,   5,  5,  5,   5,   0,  -5,
      0,   0,   5,  5,  5,   5,   0,  -5,
    -10,   5,   5,  5,  5,   5,   0, -10,
    -10,   0,   5,  0,  0,   0,   0, -10,
    -20, -10, -10, -5, -5, -10, -10, -20,
];

#[rustfmt::skip]
const QUEEN_EG: [i32; 64] = [
    -20, -10, -10, -5, -5, -10, -10, -20,
    -10,   0,   0,  0,  0,   0,   0, -10,
    -10,   0,   5,  5,  5,   5,   0, -10,
     -5,   0,   5,  5,  5,   5,   0,  -5,
     -5,   0,   5,  5,  5,   5,   0,  -5,
    -10,   0,   5,  5,  5,   5,   0, -10,
    -10,   0,   0,  0,  0,   0,   0, -10,
    -20, -10, -10, -5, -5, -10, -10, -20,
];

#[rustfmt::skip]
const KING_MG: [i32; 64] = [
     20,  30,  10,   0,   0,  10,  30,  20,
     20,  20,   0,   0,   0,   0,  20,  20,
    -10, -20, -20, -20, -20, -20, -20, -10,
    -20, -30, -30, -40, -40, -30, -30, -20,
    -30, -40, -40, -50, -50, -40, -40, -30,
    -30, -40, -40, -50, -50, -40, -40, -30,
    -30, -40, -40, -50, -50, -40, -40, -30,
    -30, -40, -40, -50, -50, -40, -40, -30,
];

#[rustfmt::skip]
const KING_EG: [i32; 64] = [
    -50, -40, -30, -20, -20, -30, -40, -50,
    -30, -20, -10,   0,   0, -10, -20, -30,
    -30, -10,  20,  30,  30,  20, -10, -30,
    -30, -10,  30,  40,  40,  30, -10, -30,
    -30, -10,  30,  40,  40,  30, -10, -30,
    -30, -10,  20,  30,  30,  20, -10, -30,
    -30, -30,   0,   0,   0,   0, -30, -30,
    -50, -40, -30, -30, -30, -30, -40, -50,
];

// Pesi delle fasi
const PHASE_WEIGHTS: [i32; 6] = [0, 1, 1, 2, 4, 0];
const TOTAL_PHASE: i32 = 24; 

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

            // Se bianco, indice normale. Se nero, flip verticale (sq ^ 56)
            let psqt_idx = if color == Colore::Bianco { sq } else { sq ^ 56 };

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

            phase -= PHASE_WEIGHTS[p_type];
            
            // --- HEURISTICA SVILUPPO REGINA ---
            // Se siamo in apertura (phase alta) e la regina non è sulla casa di partenza (d1/d8), penalizza.
            if p_type == Pezzo::Regina.indice() && phase > 16 {
                let start_sq = if color == Colore::Bianco { 3 } else { 59 };
                if sq != start_sq {
                    let penalty = 40; // Penalità pesante
                    if color == Colore::Bianco { mg_score -= penalty; } else { mg_score += penalty; }
                }
            }
            
            // --- BONUS SVILUPPO CAVALLI ---
            // Se siamo in apertura e i cavalli sono su c3/f3 (Bianco) o c6/f6 (Nero)
            if p_type == Pezzo::Cavallo.indice() && phase > 20 {
                // Per il bianco c3(18), f3(21). Per il nero c6(42), f6(45)
                let is_good_sq = if color == Colore::Bianco { sq == 18 || sq == 21 } else { sq == 42 || sq == 45 };
                if is_good_sq {
                    let bonus = 20; 
                    if color == Colore::Bianco { mg_score += bonus; } else { mg_score -= bonus; }
                }
            }
        }
    }

    // --- CORREZIONI STRATEGICHE ---
    
    // Penalità Pedoni Doppiati
    for i in 0..8 {
        let file_mask = 0x0101010101010101u64 << i;
        
        let wp = (board.pezzi[Pezzo::Pedone.indice()] & board.colori[Colore::Bianco.indice()] & file_mask).count_ones();
        if wp > 1 { mg_score -= 15 * (wp as i32 - 1); eg_score -= 20 * (wp as i32 - 1); }

        let bp = (board.pezzi[Pezzo::Pedone.indice()] & board.colori[Colore::Nero.indice()] & file_mask).count_ones();
        if bp > 1 { mg_score += 15 * (bp as i32 - 1); eg_score += 20 * (bp as i32 - 1); }
    }

    // Bonus Coppia degli Alfieri
    let w_bishops = (board.pezzi[Pezzo::Alfiere.indice()] & board.colori[Colore::Bianco.indice()]).count_ones();
    let b_bishops = (board.pezzi[Pezzo::Alfiere.indice()] & board.colori[Colore::Nero.indice()]).count_ones();
    
    if w_bishops >= 2 { mg_score += 30; eg_score += 50; }
    if b_bishops >= 2 { mg_score -= 30; eg_score -= 50; }

    // --- INTERPOLAZIONE FASE ---
    phase = phase.clamp(0, TOTAL_PHASE);
    let final_score = (mg_score * phase + eg_score * (TOTAL_PHASE - phase)) / TOTAL_PHASE;

    if board.turno == Colore::Bianco { final_score } else { -final_score }
}