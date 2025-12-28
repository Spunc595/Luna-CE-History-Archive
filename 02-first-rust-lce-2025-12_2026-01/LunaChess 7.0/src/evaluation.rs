// src/evaluation.rs

use crate::board::{Scacchiera, Pezzo, Colore, Bitboard};

// ============================================
// COSTANTI E PESI (Tuning Manuale + PeSTO)
// ============================================

// Bonus Posizionali (MiddleGame, EndGame)
const BISHOP_PAIR: [i32; 2] = [30, 50]; // Bonus per avere la coppia
const ROOK_OPEN_FILE: [i32; 2] = [25, 15]; // Bonus torre colonna aperta
const ROOK_SEMI_OPEN_FILE: [i32; 2] = [10, 10]; // Bonus torre colonna semi-aperta

// Penalità Pedoni
const PAWN_ISOLATED: [i32; 2] = [-10, -20];
const PAWN_DOUBLED: [i32; 2] = [-10, -20];
const PAWN_PASSED: [i32; 8] = [ 0, 5, 10, 20, 40, 80, 150, 0 ]; // Bonus per rank

// Sicurezza Re (Solo MiddleGame)
const KING_SHIELD_MISSING: i32 = -20; // Penalità per pedone mancante davanti al re
const KING_OPEN_FILE: i32 = -30;      // Penalità grave se il re è su una colonna aperta

// --- TABELLE PeSTO (Ottimizzate) ---
#[rustfmt::skip]
const MG_PAWN: [i32; 64] = [
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
const EG_PAWN: [i32; 64] = [
     0,   0,   0,   0,   0,   0,   0,   0,
   178, 173, 158, 134, 147, 132, 165, 187,
    94, 100,  85,  67,  56,  53,  82,  84,
    32,  24,  13,   5,  -2,   4,  17,  17,
    13,   9,  -3,  -7,  -7,  -8,   3,  -1,
     4,   7,  -6,   1,   0,  -5,  -1,  -8,
    13,   8,   8,  10,  13,   0,   2,  -7,
     0,   0,   0,   0,   0,   0,   0,   0,
];

#[rustfmt::skip]
const MG_KNIGHT: [i32; 64] = [
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
const EG_KNIGHT: [i32; 64] = [
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
const MG_BISHOP: [i32; 64] = [
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
const EG_BISHOP: [i32; 64] = [
    -14, -21, -11,  -8, -7,  -9, -17, -24,
     -8,  -4,   7, -12, -3, -13,  -4, -14,
      2,  -8,   0,  -1, -2,   6,   0,   4,
     -3,   9,  12,   9, 14,  10,   3,   2,
     -6,   3,  13,  19,  7,  10,  -3,  -9,
    -12,  -3,   5,  10,  5,   6,   0,  -7,
    -15, -10, -10,  -5, -4,   0,  -8, -23,
    -23,  -9, -23,  -5, -9, -16,  -5, -17,
];

#[rustfmt::skip]
const MG_ROOK: [i32; 64] = [
     32,  42,  32,  51, 63,   9,  31,  43,
     27,  32,  58,  62, 80,  67,  26,  44,
     -5,  19,  26,  36, 17,  45,  61,  16,
    -24, -11,   7,  26, 24,  35,  -8, -20,
    -36, -26, -12,  -1,  9,  -7,   6, -23,
    -45, -25, -16, -17,  3,   0,  -5, -33,
    -44, -16, -20,  -9, -1,  11,  -6, -71,
    -19, -13,   1,  17, 16,   7, -37, -26,
];

#[rustfmt::skip]
const EG_ROOK: [i32; 64] = [
     13,  10,  18,  15,  12,  12,   8,   5,
     11,  13,  13,  11,  -3,   3,   8,   3,
      7,   7,   7,   5,   4,  -3,  -5,  -3,
      4,   3,  13,   1,   2,   1,  -1,   2,
      3,   5,   8,   4,  -5,  -6,  -8, -11,
     -4,   0,  -5,  -1,  -7, -12,  -8, -16,
     -6,  -6,   0,   2,  -9,  -9, -11,  -3,
     -9,   2,   3,  -1,  -5, -13,   4, -20,
];

#[rustfmt::skip]
const MG_QUEEN: [i32; 64] = [
    -28,   0,  29,  12,  59,  44,  43,  45,
    -24, -39,  -5,   1, -16,  57,  28,  54,
    -13, -17,   7,   8,  29,  56,  47,  57,
    -27, -27, -16, -16,  -1,  17,  -2,   1,
     -9, -26, -28, -10,  -2, -11,  33, -10,
    -14,   2, -11,  -2,  -5,   2,  14,   5,
    -35,  -8,  11,   2,   8,  15,  -3,   1,
     -1, -18,  -9,  10, -15, -25, -31, -50,
];

#[rustfmt::skip]
const EG_QUEEN: [i32; 64] = [
     -9,  22,  22,  27,  27,  19,  10,  20,
    -17,  20,  32,  41,  58,  25,  30,   0,
    -20,   6,   9,  49,  47,  35,  19,   9,
      3,  22,  24,  45,  57,  40,  57,  36,
    -18,  28,  19,  47,  31,  34,  39,  23,
    -16, -27,  15,   6,   9,  17,  10,   5,
    -22, -23, -30, -16, -16,  23,   0, -36,
    -14,  -5, -15, -10, -10, -10, -10,  -2,
];

#[rustfmt::skip]
const MG_KING: [i32; 64] = [
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
const EG_KING: [i32; 64] = [
    -74, -35, -18, -18, -11,  15,   4, -17,
    -12,  17,  14,  17,  17,  38,  23,  11,
     10,  17,  23,  15,  20,  45,  44,  13,
     -8,  22,  24,  27,  26,  33,  26,   3,
    -18,  -4,  21,  24,  27,  23,   9, -11,
    -19,  -3,  11,  21,  23,  16,   7,  -9,
    -27, -11,   4,  13,  14,   4,  -5, -17,
    -53, -34, -21, -11, -28, -14, -24, -43,
];

// Valori Base Materiale
const MG_VALUE: [i32; 6] = [ 82, 337, 365, 477, 1025,  0];
const EG_VALUE: [i32; 6] = [ 94, 281, 297, 512,  936,  0];

// Fasi per Tapered Eval
const GAME_PHASE_INC: [i32; 6] = [0, 1, 1, 2, 4, 0];

// Maschere delle colonne (file 0..7)
const FILE_MASKS: [Bitboard; 8] = [
    0x0101010101010101, 0x0202020202020202, 0x0404040404040404, 0x0808080808080808,
    0x1010101010101010, 0x2020202020202020, 0x4040404040404040, 0x8080808080808080
];

// Maschere adiacenti (per pedoni isolati)
const ADJACENT_FILES: [Bitboard; 8] = [
    0x0202020202020202, 
    0x0505050505050505,
    0x0A0A0A0A0A0A0A0A,
    0x1414141414141414,
    0x2828282828282828,
    0x5050505050505050,
    0xA0A0A0A0A0A0A0A0,
    0x4040404040404040
];

// ============================================
// VALUTAZIONE PRINCIPALE
// ============================================

pub fn valuta_posizione(s: &Scacchiera) -> i32 {
    let mut mg = [0; 2];
    let mut eg = [0; 2];
    let mut phase = 0;

    let white_pawns = s.bitboard_pezzo_colore(Pezzo::Pedone, Colore::Bianco);
    let black_pawns = s.bitboard_pezzo_colore(Pezzo::Pedone, Colore::Nero);
    let all_pawns = white_pawns | black_pawns;

    // --- LOOP PEZZI & PeSTO ---
    for p in [Pezzo::Pedone, Pezzo::Cavallo, Pezzo::Alfiere, Pezzo::Torre, Pezzo::Regina, Pezzo::Re] {
        let mut w = s.bitboard_pezzo_colore(p, Colore::Bianco);
        let mut b = s.bitboard_pezzo_colore(p, Colore::Nero);
        
        let p_idx = p.indice();
        let phase_val = GAME_PHASE_INC[p_idx];
        let mat_mg = MG_VALUE[p_idx];
        let mat_eg = EG_VALUE[p_idx];

        // Conteggi per bonus speciali
        let w_count = w.count_ones();
        let b_count = b.count_ones();

        // BIANCO
        while w != 0 {
            let sq = w.trailing_zeros() as usize;
            w &= w - 1;
            let f = sq ^ 56; // Flip per PeSTO bianco
            
            mg[0] += mat_mg + get_mg(p, f);
            eg[0] += mat_eg + get_eg(p, f);
            phase += phase_val;

            // Logica specifica per pezzo
            match p {
                Pezzo::Pedone => evaluate_pawn(sq, Colore::Bianco, white_pawns, black_pawns, &mut mg, &mut eg),
                Pezzo::Torre => evaluate_rook(sq, white_pawns, black_pawns, &mut mg, &mut eg, 0),
                Pezzo::Re => evaluate_king_safety(sq, Colore::Bianco, white_pawns, &mut mg),
                _ => {}
            }
        }

        // NERO
        while b != 0 {
            let sq = b.trailing_zeros() as usize;
            b &= b - 1;
            // Nero usa indice normale per PeSTO simmetrico
            
            mg[1] += mat_mg + get_mg(p, sq);
            eg[1] += mat_eg + get_eg(p, sq);
            phase += phase_val;

            match p {
                Pezzo::Pedone => evaluate_pawn(sq, Colore::Nero, black_pawns, white_pawns, &mut mg, &mut eg),
                Pezzo::Torre => evaluate_rook(sq, black_pawns, white_pawns, &mut mg, &mut eg, 1),
                Pezzo::Re => evaluate_king_safety(sq, Colore::Nero, black_pawns, &mut mg),
                _ => {}
            }
        }

        // Bonus Coppia Alfieri
        if p == Pezzo::Alfiere {
            if w_count >= 2 { mg[0] += BISHOP_PAIR[0]; eg[0] += BISHOP_PAIR[1]; }
            if b_count >= 2 { mg[1] += BISHOP_PAIR[0]; eg[1] += BISHOP_PAIR[1]; }
        }
    }

    // --- TAPERED EVAL FINAL ---
    let p_clamped = phase.min(24);
    let mg_tot = mg[0] - mg[1];
    let eg_tot = eg[0] - eg[1];
    let score = (mg_tot * p_clamped + eg_tot * (24 - p_clamped)) / 24;

    // Bonus Tempo (piccolo vantaggio a chi muove)
    let tempo = 10;
    
    if s.turno() == Colore::Bianco {
        score + tempo
    } else {
        -score + tempo
    }
}

// ============================================
// LOGICA SPECIFICA PEZZI
// ============================================

fn evaluate_pawn(sq: usize, color: Colore, my_pawns: Bitboard, opp_pawns: Bitboard, mg: &mut [i32; 2], eg: &mut [i32; 2]) {
    let idx = color.indice();
    let file = sq % 8;
    let rank = sq / 8;
    let file_mask = FILE_MASKS[file];
    let adj_mask = ADJACENT_FILES[file];

    // 1. Pedone Isolato (Nessun amico nelle colonne adiacenti)
    if (my_pawns & adj_mask) == 0 {
        mg[idx] += PAWN_ISOLATED[0];
        eg[idx] += PAWN_ISOLATED[1];
    }

    // 2. Pedone Doppiato (Amico sulla stessa colonna davanti)
    // Semplificazione: controlla se ci sono altri pedoni sulla stessa file_mask
    // (count_ones > 1 sulla colonna). Per velocità lo facciamo semplice:
    if (my_pawns & file_mask).count_ones() > 1 {
        // Applica penalità solo una volta per colonna o per pedone? Qui per pedone.
        // Riduce penalità se doppiato, per non contarla due volte troppo pesante
        mg[idx] += PAWN_DOUBLED[0] / 2; 
        eg[idx] += PAWN_DOUBLED[1] / 2;
    }

    // 3. Pedone Passato (Nessun nemico davanti sulla stessa colonna o adiacenti)
    let forward_mask = if color == Colore::Bianco {
        // Maschera dei rank davanti
        !((1u64 << (sq + 1)) - 1) // Bitmask trick grezzo, meglio lookup, ma ok per ora
    } else {
         (1u64 << sq) - 1
    };
    
    // Check zona davanti
    let passed_zone = forward_mask & (file_mask | adj_mask);
    
    if (opp_pawns & passed_zone) == 0 {
        // È passato! Bonus basato sul rank
        let relative_rank = if color == Colore::Bianco { rank } else { 7 - rank };
        let bonus = PAWN_PASSED[relative_rank as usize];
        mg[idx] += bonus;
        eg[idx] += bonus * 2; // Vale doppio nel finale
    }
}

fn evaluate_rook(sq: usize, my_pawns: Bitboard, opp_pawns: Bitboard, mg: &mut [i32; 2], eg: &mut [i32; 2], idx: usize) {
    let file = sq % 8;
    let file_mask = FILE_MASKS[file];
    
    // Colonna semi-aperta (nessun pedone amico)
    if (my_pawns & file_mask) == 0 {
        // Colonna aperta (nessun pedone nemico)
        if (opp_pawns & file_mask) == 0 {
            mg[idx] += ROOK_OPEN_FILE[0];
            eg[idx] += ROOK_OPEN_FILE[1];
        } else {
            mg[idx] += ROOK_SEMI_OPEN_FILE[0];
            eg[idx] += ROOK_SEMI_OPEN_FILE[1];
        }
    }
}

fn evaluate_king_safety(sq: usize, color: Colore, my_pawns: Bitboard, mg: &mut [i32; 2]) {
    let idx = color.indice();
    let file = sq % 8;
    let rank = sq / 8;
    
    // Solo nel MiddleGame ci preoccupiamo dello scudo pedonale
    // Bianco in basso (rank < 2), Nero in alto (rank > 5) tipicamente
    if (color == Colore::Bianco && rank > 2) || (color == Colore::Nero && rank < 5) {
        return; // Re fuori posizione arrocco o già in finale attivo
    }

    // Controlla i 3 pedoni davanti (file-1, file, file+1)
    let mut shield_penalty = 0;
    
    // Colonna Re
    let file_mask = FILE_MASKS[file];
    if (my_pawns & file_mask) == 0 { shield_penalty += KING_OPEN_FILE; }
    else if is_pawn_shield_missing(sq, file_mask, my_pawns, color) { shield_penalty += KING_SHIELD_MISSING; }

    // Colonna Sinistra (se esiste)
    if file > 0 {
        let left_mask = FILE_MASKS[file - 1];
        if (my_pawns & left_mask) == 0 { shield_penalty += KING_OPEN_FILE / 2; }
        else if is_pawn_shield_missing(sq, left_mask, my_pawns, color) { shield_penalty += KING_SHIELD_MISSING / 2; }
    }

    // Colonna Destra (se esiste)
    if file < 7 {
        let right_mask = FILE_MASKS[file + 1];
        if (my_pawns & right_mask) == 0 { shield_penalty += KING_OPEN_FILE / 2; }
        else if is_pawn_shield_missing(sq, right_mask, my_pawns, color) { shield_penalty += KING_SHIELD_MISSING / 2; }
    }

    mg[idx] += shield_penalty;
}

fn is_pawn_shield_missing(king_sq: usize, file_mask: Bitboard, pawns: Bitboard, color: Colore) -> bool {
    // Controlla se c'è un pedone immediatamente davanti o di 2 passi davanti
    // Semplificato: controlla solo l'esistenza sulla colonna "vicino" al re
    let relevant_pawns = pawns & file_mask;
    if relevant_pawns == 0 { return true; }
    
    // Cerca il pedone più vicino
    if color == Colore::Bianco {
        // Cerca primo pedone sopra il re
        let forward_mask = !((1u64 << (king_sq + 1)) - 1);
        (relevant_pawns & forward_mask) == 0 // Nessun pedone davanti
    } else {
        let forward_mask = (1u64 << king_sq) - 1;
        (relevant_pawns & forward_mask) == 0
    }
}

// Helpers PeSTO
fn get_mg(p: Pezzo, s: usize) -> i32 { match p { Pezzo::Pedone=>MG_PAWN[s], Pezzo::Cavallo=>MG_KNIGHT[s], Pezzo::Alfiere=>MG_BISHOP[s], Pezzo::Torre=>MG_ROOK[s], Pezzo::Regina=>MG_QUEEN[s], Pezzo::Re=>MG_KING[s] } }
fn get_eg(p: Pezzo, s: usize) -> i32 { match p { Pezzo::Pedone=>EG_PAWN[s], Pezzo::Cavallo=>EG_KNIGHT[s], Pezzo::Alfiere=>EG_BISHOP[s], Pezzo::Torre=>EG_ROOK[s], Pezzo::Regina=>EG_QUEEN[s], Pezzo::Re=>EG_KING[s] } }