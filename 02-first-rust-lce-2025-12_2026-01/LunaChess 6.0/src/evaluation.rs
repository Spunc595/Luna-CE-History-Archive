use crate::board::{Scacchiera, Colore, Pezzo, Casella, Bitboard};
use crate::attacks::{pawn_attacks, knight_attacks, king_attacks, bishop_attacks, rook_attacks};
use std::cmp::{min, max};

// ===== HELPER FUNCTION =====
// Funzione helper per ottenere bitboard di un pezzo di un determinato colore
#[inline(always)]
fn bitboard_pezzo_colore(scacchiera: &Scacchiera, pezzo: Pezzo, colore: Colore) -> Bitboard {
    scacchiera.bitboard_pezzo(pezzo) & scacchiera.bitboard_colore(colore)
}

// ===== CONSTANTS =====
const PEDONE_VAL: i32 = 100;
const CAVALLO_VAL: i32 = 320;
const ALFIERE_VAL: i32 = 330;
const TORRE_VAL: i32 = 500;
const REGINA_VAL: i32 = 900;
const RE_VAL: i32 = 20000;

// ===== GAME PHASE WEIGHTS =====
const GAME_PHASE_WEIGHTS: [i32; 6] = [0, 1, 1, 2, 4, 0]; // P, N, B, R, Q, K

// ===== PIECE-SQUARE TABLES (Tapered Evaluation) =====

// Opening tables (middlegame)
const PEDONE_BIANCO_MG: [i32; 64] = [
    0,   0,   0,   0,   0,   0,   0,   0,
    98, 134,  61,  95,  68, 126,  34, -11,
    -6,   7,  26,  31,  65,  56,  25, -20,
    -14,  13,   6,  21,  23,  12,  17, -23,
    -27,  -2,  -5,  12,  17,   6,  10, -25,
    -26,  -4,  -4, -10,   3,   3,  33, -12,
    -35,  -1, -20, -23, -15,  24,  38, -22,
    0,   0,   0,   0,   0,   0,   0,   0,
];

const PEDONE_NERO_MG: [i32; 64] = {
    let mut table = [0; 64];
    let mut i = 0;
    while i < 64 {
        table[i] = PEDONE_BIANCO_MG[63 - i];
        i += 1;
    }
    table
};

const CAVALLO_MG: [i32; 64] = [
    -167, -89, -34, -49,  61, -97, -15, -107,
    -73, -41,  72,  36,  23,  62,   7,  -17,
    -47,  60,  37,  65,  84, 129,  73,   44,
    -9,  17,  19,  53,  37,  69,  18,   22,
    -13,   4,  16,  13,  28,  19,  21,   -8,
    -23,  -9,  12,  10,  19,  17,  25,  -16,
    -29, -53, -12,  -3,  -1,  18, -14,  -19,
    -105, -21, -58, -33, -17, -28, -19,  -23,
];

const ALFIERE_MG: [i32; 64] = [
    -29,   4, -82, -37, -25, -42,   7,  -8,
    -26,  16, -18, -13,  30,  59,  18, -47,
    -16,  37,  43,  40,  35,  50,  37,  -2,
    -4,   5,  19,  50,  37,  37,   7,  -2,
    -6,  13,  13,  26,  34,  12,  10,   4,
    0,  15,  15,  15,  14,  27,  18,  10,
    4,  15,  16,   0,   7,  21,  33,   1,
    -33,  -3, -14, -21, -13, -12, -39, -21,
];

const TORRE_MG: [i32; 64] = [
    32,  42,  32,  51, 63,  9,  31,  43,
    27,  32,  58,  62, 80, 67,  26,  44,
    -5,  19,  26,  36, 17, 45,  61,  16,
    -24, -11,   7,  26, 24, 35,  -8, -20,
    -36, -26, -12,  -1,  9, -7,   6, -23,
    -45, -25, -16, -17,  3,  0,  -5, -33,
    -44, -16, -20,  -9, -1, 11,  -6, -71,
    -19, -13,   1,  17, 16,  7, -37, -26,
];

const REGINA_MG: [i32; 64] = [
    -28,   0,  29,  12,  59,  44,  43,  45,
    -24, -39,  -5,   1, -16,  57,  28,  54,
    -13, -17,   7,   8,  29,  56,  47,  57,
    -27, -27, -16, -16,  -1,  17,  -2,   1,
    -9, -26,  -9, -10,  -2,  -4,   3,  -3,
    -14,   2, -11,  -2,  -5,   2,  14,   5,
    -35,  -8,  11,   2,   8,  15,  -3,   1,
    -1, -18,  -9,  10, -15, -25, -31, -50,
];

const RE_MG: [i32; 64] = [
    -65,  23,  16, -15, -56, -34,   2,  13,
    29,  -1, -20,  -7,  -8,  -4, -38, -29,
    -9,  24,   2, -16, -20,   6,  22, -22,
    -17, -20, -12, -27, -30, -25, -14, -36,
    -49,  -1, -27, -39, -46, -44, -33, -51,
    -14, -14, -22, -46, -44, -30, -15, -27,
    1,   7,  -8, -64, -43, -16,   9,   8,
    -15,  36,  12, -54,   8, -28,  24,  14,
];

// Endgame tables
const PEDONE_BIANCO_EG: [i32; 64] = [
    0,   0,   0,   0,   0,   0,   0,   0,
    178, 173, 158, 134, 147, 132, 165, 187,
    94, 100,  85,  67,  56,  53,  82,  84,
    32,  24,  13,   5,  -2,   4,  17,  17,
    13,   9,  -3,  -7,  -7,  -8,   3,  -1,
    4,   7,  -6,   1,   0,  -5,  -1,  -8,
    13,   8,   8,  10,  13,   0,   2,  -7,
    0,   0,   0,   0,   0,   0,   0,   0,
];

const PEDONE_NERO_EG: [i32; 64] = {
    let mut table = [0; 64];
    let mut i = 0;
    while i < 64 {
        table[i] = PEDONE_BIANCO_EG[63 - i];
        i += 1;
    }
    table
};

const CAVALLO_EG: [i32; 64] = [
    -58, -38, -13, -28, -31, -27, -63, -99,
    -25,  -8, -25,  -2,  -9, -25, -24, -52,
    -24, -20,  10,   9,  -1,  -9, -19, -41,
    -17,   3,  22,  22,  22,  11,   8, -18,
    -18,  -6,  16,  25,  16,  17,   4, -18,
    -23,  -3,  -1,  15,  10,  -3, -20, -22,
    -42, -20, -10,  -5,  -2, -20, -23, -44,
    -29, -51, -23, -15, -22, -18, -50, -64,
];

const ALFIERE_EG: [i32; 64] = [
    -14, -21, -11,  -8, -7,  -9, -17, -24,
    -8,  -4,   7, -12, -3, -13,  -4, -14,
    2,  -8,   0,  -1, -2,   6,   0,   4,
    -3,   9,  12,   9, 14,  10,   3,   2,
    -6,   3,  13,  19,  7,  10,  -3,  -9,
    -12,  -3,   8,  10, 13,   3,  -7, -15,
    -14, -18,  -7,  -1,  4,  -9, -15, -27,
    -23,  -9, -23,  -5, -9, -16,  -5, -17,
];

const TORRE_EG: [i32; 64] = [
    13, 10, 18, 15, 12,  12,   8,   5,
    11, 13, 13, 11, -3,   3,   8,   3,
    7,  7,  7,  5,  4,  -3,  -5,  -3,
    4,  3,  13,  1,  2,   1,  -1,   2,
    3,  5,  8,  4, -5,  -6,  -8, -11,
    -4,  0, -5, -1, -7, -12,  -8, -16,
    -6, -6,  0,  2, -9,  -9, -11,  -3,
    -9,  2,  3, -1, -5, -13,   4, -20,
];

const REGINA_EG: [i32; 64] = [
    -9,  22,  22,  27,  27,  19,  10,  20,
    -17,  20,  32,  41,  58,  25,  30,   0,
    -20,   6,   9,  49,  47,  35,  19,   9,
    3,  22,  24,  45,  57,  40,  57,  36,
    -18,  28,  19,  47,  31,  34,  39,  23,
    -16, -27,  15,   6,   9,  17,  10,   5,
    -22, -23, -30, -16, -16, -23, -36, -32,
    -33, -28, -22, -43,  -5, -32, -20, -41,
];

const RE_EG: [i32; 64] = [
    -74, -35, -18, -18, -11,  15,   4, -17,
    -12,  17,  14,  17,  17,  38,  23,  11,
    10,  17,  23,  15,  20,  45,  44,  13,
    -8,  22,  24,  27,  26,  33,  26,   3,
    -18,  -4,  21,  24,  27,  23,   9, -11,
    -19,  -3,  11,  21,  23,  16,  7,  -9,
    -27, -11,   4,  13,  14,   4,  -5, -17,
    -53, -34, -21, -11, -28, -14, -24, -43,
];

// ===== HELPER FUNCTIONS =====

#[inline(always)]
fn square_index(square: usize, color: Colore) -> usize {
    if color == Colore::Bianco {
        square ^ 56 // XOR con 56 scambia rank 0 con rank 7
    } else {
        square
    }
}

#[inline(always)]
fn mirrortable(table: &[i32; 64]) -> [i32; 64] {
    let mut mirrored = [0; 64];
    for i in 0..64 {
        mirrored[i] = table[i ^ 56];
    }
    mirrored
}

struct EvaluationAccumulator {
    pub material: [i32; 2],
    pub psqt_mg: [i32; 2],
    pub psqt_eg: [i32; 2],
    pub mobility: [i32; 2],
    pub pawn_structure: [i32; 2],
    pub king_safety: [i32; 2],
    pub center_control: [i32; 2],
    pub bishop_pair: [i32; 2],
    pub tempo: i32,
}

impl EvaluationAccumulator {
    fn new() -> Self {
        Self {
            material: [0, 0],
            psqt_mg: [0, 0],
            psqt_eg: [0, 0],
            mobility: [0, 0],
            pawn_structure: [0, 0],
            king_safety: [0, 0],
            center_control: [0, 0],
            bishop_pair: [0, 0],
            tempo: 0,
        }
    }
    
    fn final_score(&self, phase: i32) -> i32 {
        let mg_score = 
            (self.material[0] + self.psqt_mg[0] + self.mobility[0] + 
             self.pawn_structure[0] + self.king_safety[0] + 
             self.center_control[0] + self.bishop_pair[0]) -
            (self.material[1] + self.psqt_mg[1] + self.mobility[1] + 
             self.pawn_structure[1] + self.king_safety[1] + 
             self.center_control[1] + self.bishop_pair[1]);
        
        let eg_score = 
            (self.material[0] + self.psqt_eg[0] + self.mobility[0] * 2 + 
             self.pawn_structure[0] * 2 + self.king_safety[0] / 2 + 
             self.center_control[0] + self.bishop_pair[0]) -
            (self.material[1] + self.psqt_eg[1] + self.mobility[1] * 2 + 
             self.pawn_structure[1] * 2 + self.king_safety[1] / 2 + 
             self.center_control[1] + self.bishop_pair[1]);
        
        // Tapered evaluation
        let tapered_score = ((mg_score as i64 * (24 - phase) as i64 + eg_score as i64 * phase as i64) / 24) as i32;
        
        tapered_score + self.tempo
    }
}

// ===== EVALUATION FUNCTIONS =====

pub fn valuta_posizione(scacchiera: &Scacchiera) -> i32 {
    let mut acc = EvaluationAccumulator::new();
    
    // Calcola game phase
    let phase = calculate_game_phase(scacchiera);
    
    // Valuta materiale e PSQT
    evaluate_material_and_psqt(scacchiera, &mut acc);
    
    // Valuta mobilità
    evaluate_mobility(scacchiera, &mut acc);
    
    // Valuta struttura pedoni
    evaluate_pawn_structure(scacchiera, &mut acc);
    
    // Valuta sicurezza del re
    evaluate_king_safety(scacchiera, &mut acc, phase);
    
    // Valuta controllo del centro
    evaluate_center_control(scacchiera, &mut acc);
    
    // Valuta bishop pair
    evaluate_bishop_pair(scacchiera, &mut acc);
    
    // Bonus per tempo (colore da muovere)
    acc.tempo = if scacchiera.colore_attivo() == Colore::Bianco { 10 } else { -10 };
    
    // Penalità per scacco
    if scacchiera.re_in_scacco(scacchiera.colore_attivo()) {
        let color_idx = if scacchiera.colore_attivo() == Colore::Bianco { 0 } else { 1 };
        acc.king_safety[color_idx] -= 50;
    }
    
    acc.final_score(phase)
}

#[inline(always)]
fn calculate_game_phase(scacchiera: &Scacchiera) -> i32 {
    let mut phase = 24;
    
    // Sottrai valori per pezzi catturati
    for &color in &[Colore::Bianco, Colore::Nero] {
        for piece in &[Pezzo::Cavallo, Pezzo::Alfiere, Pezzo::Torre, Pezzo::Regina] {
            let bb = bitboard_pezzo_colore(scacchiera, *piece, color);
            phase -= (bb.count_ones() as i32) * GAME_PHASE_WEIGHTS[*piece as usize];
        }
    }
    
    phase.clamp(0, 24)
}

fn evaluate_material_and_psqt(scacchiera: &Scacchiera, acc: &mut EvaluationAccumulator) {
    for (color_idx, &color) in [Colore::Bianco, Colore::Nero].iter().enumerate() {
        for piece in &[Pezzo::Pedone, Pezzo::Cavallo, Pezzo::Alfiere, Pezzo::Torre, Pezzo::Regina, Pezzo::Re] {
            let bb = bitboard_pezzo_colore(scacchiera, *piece, color);
            let mut temp_bb = bb;
            
            while temp_bb != 0 {
                let square = temp_bb.trailing_zeros() as usize;
                temp_bb &= temp_bb - 1;
                
                // Valore materiale
                let material_value = match piece {
                    Pezzo::Pedone => PEDONE_VAL,
                    Pezzo::Cavallo => CAVALLO_VAL,
                    Pezzo::Alfiere => ALFIERE_VAL,
                    Pezzo::Torre => TORRE_VAL,
                    Pezzo::Regina => REGINA_VAL,
                    Pezzo::Re => RE_VAL,
                };
                acc.material[color_idx] += material_value;
                
                // PSQT
                let idx = square_index(square, color);
                match piece {
                    Pezzo::Pedone => {
                        acc.psqt_mg[color_idx] += if color == Colore::Bianco { PEDONE_BIANCO_MG[idx] } else { PEDONE_NERO_MG[idx] };
                        acc.psqt_eg[color_idx] += if color == Colore::Bianco { PEDONE_BIANCO_EG[idx] } else { PEDONE_NERO_EG[idx] };
                    }
                    Pezzo::Cavallo => {
                        acc.psqt_mg[color_idx] += CAVALLO_MG[idx];
                        acc.psqt_eg[color_idx] += CAVALLO_EG[idx];
                    }
                    Pezzo::Alfiere => {
                        acc.psqt_mg[color_idx] += ALFIERE_MG[idx];
                        acc.psqt_eg[color_idx] += ALFIERE_EG[idx];
                    }
                    Pezzo::Torre => {
                        acc.psqt_mg[color_idx] += TORRE_MG[idx];
                        acc.psqt_eg[color_idx] += TORRE_EG[idx];
                    }
                    Pezzo::Regina => {
                        acc.psqt_mg[color_idx] += REGINA_MG[idx];
                        acc.psqt_eg[color_idx] += REGINA_EG[idx];
                    }
                    Pezzo::Re => {
                        acc.psqt_mg[color_idx] += RE_MG[idx];
                        acc.psqt_eg[color_idx] += RE_EG[idx];
                    }
                }
            }
        }
    }
}

fn evaluate_mobility(scacchiera: &Scacchiera, acc: &mut EvaluationAccumulator) {
    let occupancy = scacchiera.occupazione();
    
    for (color_idx, &color) in [Colore::Bianco, Colore::Nero].iter().enumerate() {
        // Cavalli
        let knights = bitboard_pezzo_colore(scacchiera, Pezzo::Cavallo, color);
        let mut temp_bb = knights;
        while temp_bb != 0 {
            let square = temp_bb.trailing_zeros() as usize;
            temp_bb &= temp_bb - 1;
            
            let attacks = knight_attacks(square);
            let mobility = (attacks & !scacchiera.bitboard_colore(color)).count_ones() as i32;
            acc.mobility[color_idx] += mobility * 2;
        }
        
        // Alfieri
        let bishops = bitboard_pezzo_colore(scacchiera, Pezzo::Alfiere, color);
        temp_bb = bishops;
        while temp_bb != 0 {
            let square = temp_bb.trailing_zeros() as usize;
            temp_bb &= temp_bb - 1;
            
            let attacks = bishop_attacks(square, occupancy);
            let mobility = (attacks & !scacchiera.bitboard_colore(color)).count_ones() as i32;
            acc.mobility[color_idx] += mobility * 2;
        }
        
        // Torri
        let rooks = bitboard_pezzo_colore(scacchiera, Pezzo::Torre, color);
        temp_bb = rooks;
        while temp_bb != 0 {
            let square = temp_bb.trailing_zeros() as usize;
            temp_bb &= temp_bb - 1;
            
            let attacks = rook_attacks(square, occupancy);
            let mobility = (attacks & !scacchiera.bitboard_colore(color)).count_ones() as i32;
            acc.mobility[color_idx] += mobility;
        }
        
        // Regine
        let queens = bitboard_pezzo_colore(scacchiera, Pezzo::Regina, color);
        temp_bb = queens;
        while temp_bb != 0 {
            let square = temp_bb.trailing_zeros() as usize;
            temp_bb &= temp_bb - 1;
            
            let attacks = bishop_attacks(square, occupancy) | rook_attacks(square, occupancy);
            let mobility = (attacks & !scacchiera.bitboard_colore(color)).count_ones() as i32;
            acc.mobility[color_idx] += mobility;
        }
    }
}

fn evaluate_pawn_structure(scacchiera: &Scacchiera, acc: &mut EvaluationAccumulator) {
    for (color_idx, &color) in [Colore::Bianco, Colore::Nero].iter().enumerate() {
        let pawns = bitboard_pezzo_colore(scacchiera, Pezzo::Pedone, color);
        
        // Bonus per pedoni passati
        acc.pawn_structure[color_idx] += evaluate_passed_pawns(pawns, color) * 30;
        
        // Penalità per pedoni isolati
        acc.pawn_structure[color_idx] -= evaluate_isolated_pawns(pawns) * 20;
        
        // Penalità per pedoni doubled
        acc.pawn_structure[color_idx] -= evaluate_doubled_pawns(pawns) * 10;
        
        // Bonus per pedoni centrali
        let central_pawns = pawns & 0x0000001818000000u64;
        acc.pawn_structure[color_idx] += central_pawns.count_ones() as i32 * 15;
    }
}

fn evaluate_king_safety(scacchiera: &Scacchiera, acc: &mut EvaluationAccumulator, phase: i32) {
    if phase > 16 { // Endgame, king safety meno importante
        return;
    }
    
    for (color_idx, &color) in [Colore::Bianco, Colore::Nero].iter().enumerate() {
        let king_bb = bitboard_pezzo_colore(scacchiera, Pezzo::Re, color);
        if king_bb == 0 {
            continue;
        }
        
        let king_sq = king_bb.trailing_zeros() as usize;
        
        // Bonus per arrocco fatto
        let castling_rights = scacchiera.diritti_arrocco();
        let has_castled = if color == Colore::Bianco {
            (castling_rights & 0b0011) == 0  // Controlla se entrambi i diritti di arrocco bianco sono 0
        } else {
            (castling_rights & 0b1100) == 0  // Controlla se entrambi i diritti di arrocco nero sono 0
        };
        
        if has_castled {
            acc.king_safety[color_idx] += 20;
        }
        
        // Bonus per pawn shield
        acc.king_safety[color_idx] += evaluate_pawn_shield(scacchiera, king_sq, color) * 10;
        
        // Penalità per re esposto
        acc.king_safety[color_idx] -= evaluate_king_exposure(scacchiera, king_sq, color) * 15;
    }
}

fn evaluate_center_control(scacchiera: &Scacchiera, acc: &mut EvaluationAccumulator) {
    let center_squares = 0x0000001818000000u64; // d4, e4, d5, e5
    let extended_center = 0x00003C3C3C3C0000u64; // c3-f6
    
    for (color_idx, &color) in [Colore::Bianco, Colore::Nero].iter().enumerate() {
        // Controllo con pedoni
        let pawns = bitboard_pezzo_colore(scacchiera, Pezzo::Pedone, color);
        let pawn_center_control = (pawns & extended_center).count_ones() as i32;
        acc.center_control[color_idx] += pawn_center_control * 5;
        
        // Controllo con pezzi
        let mut piece_center_control = 0;
        
        for piece in &[Pezzo::Cavallo, Pezzo::Alfiere, Pezzo::Torre, Pezzo::Regina] {
            let pieces = bitboard_pezzo_colore(scacchiera, *piece, color);
            piece_center_control += (pieces & center_squares).count_ones() as i32;
        }
        
        acc.center_control[color_idx] += piece_center_control * 10;
    }
}

fn evaluate_bishop_pair(scacchiera: &Scacchiera, acc: &mut EvaluationAccumulator) {
    for (color_idx, &color) in [Colore::Bianco, Colore::Nero].iter().enumerate() {
        let bishops = bitboard_pezzo_colore(scacchiera, Pezzo::Alfiere, color);
        
        if bishops.count_ones() >= 2 {
            // Bonus extra se sono su diagonali di colore opposto
            let light_squares = 0xAA55AA55AA55AA55u64;
            let dark_squares = !light_squares;
            
            let light_bishops = bishops & light_squares;
            let dark_bishops = bishops & dark_squares;
            
            if light_bishops != 0 && dark_bishops != 0 {
                acc.bishop_pair[color_idx] += 50;
            } else {
                acc.bishop_pair[color_idx] += 30;
            }
        }
    }
}

// ===== PAWN STRUCTURE FUNCTIONS =====

fn evaluate_passed_pawns(pawns: Bitboard, color: Colore) -> i32 {
    let mut score = 0;
    let mut bb = pawns;
    
    while bb != 0 {
        let square = bb.trailing_zeros() as usize;
        bb &= bb - 1;
        
        if is_passed_pawn_sq(square, pawns, color) {
            let rank = square / 8;
            let advancement = if color == Colore::Bianco { rank - 1 } else { 6 - rank };
            score += 1 + advancement as i32;
        }
    }
    
    score
}

fn is_passed_pawn_sq(square: usize, pawns: Bitboard, color: Colore) -> bool {
    let file = square % 8;
    let rank = square / 8;
    
    // Maschera per controllare file adiacenti nelle traverse avanti
    let forward_mask = if color == Colore::Bianco {
        // Tutte le caselle davanti a questo pedone
        !((1u64 << square) - 1) & (0x0101010101010101u64 << file)
    } else {
        // Tutte le caselle dietro a questo pedone
        ((1u64 << square) - 1) & (0x0101010101010101u64 << file)
    };
    
    // Controlla file adiacenti
    for df in -1..=1 {
        if df == 0 {
            continue;
        }
        let check_file = file as i32 + df;
        if check_file >= 0 && check_file < 8 {
            let file_mask = 0x0101010101010101u64 << (check_file as usize);
            if (pawns & file_mask & forward_mask) != 0 {
                return false;
            }
        }
    }
    
    true
}

fn evaluate_isolated_pawns(pawns: Bitboard) -> i32 {
    let mut count = 0;
    let mut bb = pawns;
    
    while bb != 0 {
        let square = bb.trailing_zeros() as usize;
        bb &= bb - 1;
        
        let file = square % 8;
        let mut has_friend = false;
        
        // Controlla file adiacenti
        for df in -1..=1 {
            if df == 0 {
                continue;
            }
            let check_file = file as i32 + df;
            if check_file >= 0 && check_file < 8 {
                let file_mask = 0x0101010101010101u64 << (check_file as usize);
                if (pawns & file_mask) != 0 {
                    has_friend = true;
                    break;
                }
            }
        }
        
        if !has_friend {
            count += 1;
        }
    }
    
    count
}

fn evaluate_doubled_pawns(pawns: Bitboard) -> i32 {
    let mut count = 0;
    
    for file in 0..8 {
        let file_mask = 0x0101010101010101u64 << file;
        let pawns_on_file = (pawns & file_mask).count_ones() as i32;
        if pawns_on_file > 1 {
            count += pawns_on_file - 1;
        }
    }
    
    count
}

fn evaluate_pawn_shield(scacchiera: &Scacchiera, king_sq: usize, color: Colore) -> i32 {
    let king_file = king_sq % 8;
    let king_rank = king_sq / 8;
    let pawns = bitboard_pezzo_colore(scacchiera, Pezzo::Pedone, color);
    let mut shield = 0;
    
    for df in -1..=1 {
        let check_file = king_file as i32 + df;
        if check_file < 0 || check_file >= 8 {
            continue;
        }
        
        for dr in 1..=2 {
            let check_rank = if color == Colore::Bianco {
                king_rank as i32 + dr
            } else {
                king_rank as i32 - dr
            };
            
            if check_rank < 0 || check_rank >= 8 {
                continue;
            }
            
            let check_sq = (check_rank * 8 + check_file) as usize;
            if (pawns & (1u64 << check_sq)) != 0 {
                shield += 3 - dr;
            }
        }
    }
    
    shield
}

fn evaluate_king_exposure(scacchiera: &Scacchiera, king_sq: usize, color: Colore) -> i32 {
    let opponent = color.opposto();
    let mut exposure = 0;
    
    // Attacchi di cavalli avversari vicino al re
    let opponent_knights = bitboard_pezzo_colore(scacchiera, Pezzo::Cavallo, opponent);
    let mut bb = opponent_knights;
    while bb != 0 {
        let square = bb.trailing_zeros() as usize;
        bb &= bb - 1;
        
        let attacks = knight_attacks(square);
        if (attacks & (1u64 << king_sq)) != 0 {
            exposure += 2;
        }
    }
    
    // Attacchi di pezzi maggiori avversari
    let king_area = king_attacks(king_sq) | (1u64 << king_sq);
    for piece in &[Pezzo::Alfiere, Pezzo::Torre, Pezzo::Regina] {
        let pieces = bitboard_pezzo_colore(scacchiera, *piece, opponent);
        let mut bb = pieces;
        
        while bb != 0 {
            let square = bb.trailing_zeros() as usize;
            bb &= bb - 1;
            
            let attacks = match piece {
                Pezzo::Alfiere => bishop_attacks(square, scacchiera.occupazione()),
                Pezzo::Torre => rook_attacks(square, scacchiera.occupazione()),
                Pezzo::Regina => bishop_attacks(square, scacchiera.occupazione()) | 
                                 rook_attacks(square, scacchiera.occupazione()),
                _ => 0,
            };
            
            if (attacks & king_area) != 0 {
                exposure += 1;
            }
        }
    }
    
    exposure
}

// ===== PUBLIC INTERFACE =====

pub fn valuta_posizione_semplice(scacchiera: &Scacchiera) -> i32 {
    // Versione semplificata per debugging
    let mut score = 0;
    
    for &color in &[Colore::Bianco, Colore::Nero] {
        let sign = if color == Colore::Bianco { 1 } else { -1 };
        
        for piece in &[Pezzo::Pedone, Pezzo::Cavallo, Pezzo::Alfiere, Pezzo::Torre, Pezzo::Regina] {
            let bb = bitboard_pezzo_colore(scacchiera, *piece, color);
            score += sign * match piece {
                Pezzo::Pedone => PEDONE_VAL,
                Pezzo::Cavallo => CAVALLO_VAL,
                Pezzo::Alfiere => ALFIERE_VAL,
                Pezzo::Torre => TORRE_VAL,
                Pezzo::Regina => REGINA_VAL,
                _ => 0,
            } * bb.count_ones() as i32;
        }
    }
    
    // Bonus per il turno
    score += if scacchiera.colore_attivo() == Colore::Bianco { 10 } else { -10 };
    
    score
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Scacchiera;
    
    #[test]
    fn test_material_count() {
        let board = Scacchiera::nuova();
        let score = valuta_posizione_semplice(&board);
        assert_eq!(score, 10); // Bianco ha il turno
    }
    
    #[test]
    fn test_pawn_structure() {
        let board = Scacchiera::nuova();
        // Testa una posizione semplice
        let score = valuta_posizione(&board);
        assert!(score.abs() < 100); // La posizione iniziale dovrebbe essere circa pari
    }
}