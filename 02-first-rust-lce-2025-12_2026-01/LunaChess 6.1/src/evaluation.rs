use crate::board::{Scacchiera, Colore, Pezzo, Bitboard};
use crate::attacks::{pawn_attacks, knight_attacks, king_attacks, bishop_attacks, rook_attacks};
use std::cmp::{min, max};

// ===== HELPER FUNCTION =====
#[inline(always)]
fn bitboard_pezzo_colore(scacchiera: &Scacchiera, pezzo: Pezzo, colore: Colore) -> Bitboard {
    scacchiera.bitboard_pezzo(pezzo) & scacchiera.bitboard_colore(colore)
}

// ===== CONSTANTS (PeSTO Values) =====
// I valori PeSTO sono ottimizzati per lavorare insieme alle PSQT.
// Non modificare questi valori separatamente dalle tabelle.
const PEDONE_VAL: i32 = 82;
const CAVALLO_VAL: i32 = 337;
const ALFIERE_VAL: i32 = 365;
const TORRE_VAL: i32 = 477;
const REGINA_VAL: i32 = 1025;
const RE_VAL: i32 = 0; // Il re non viene catturato, ma ha valore posizionale

// ===== GAME PHASE WEIGHTS =====
// Pesi per calcolare la fase del gioco (0 = Endgame, 24 = Opening)
const GAME_PHASE_WEIGHTS: [i32; 6] = [0, 1, 1, 2, 4, 0];

// ===== PIECE-SQUARE TABLES (PeSTO) =====
// Tabelle "Flip" automatiche per il nero gestite via indicizzazione

#[rustfmt::skip]
const PEDONE_MG: [i32; 64] = [
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
const PEDONE_EG: [i32; 64] = [
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

#[rustfmt::skip]
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

#[rustfmt::skip]
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

#[rustfmt::skip]
const ALFIERE_EG: [i32; 64] = [
    -14, -21, -11,  -8,  -7,  -9, -17, -24,
     -8,  -4,   7, -12,  -3, -13,  -4, -14,
      2,  -8,   0,  -1,  -2,   6,   0,   4,
     -3,   9,  12,   9,  14,  10,   3,   2,
     -6,   3,  13,  19,   7,  10,  -3,  -9,
    -12,  -3,   8,  10,  13,   3,  -7, -15,
    -14, -18,  -7,  -1,   4,  -9, -15, -27,
    -23,  -9, -23,  -5,  -9, -16,  -5, -17,
];

#[rustfmt::skip]
const TORRE_MG: [i32; 64] = [
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
const TORRE_EG: [i32; 64] = [
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

#[rustfmt::skip]
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

#[rustfmt::skip]
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

#[rustfmt::skip]
const RE_EG: [i32; 64] = [
    -74, -35, -18, -18, -11,  15,   4, -17,
    -12,  17,  14,  17,  17,  38,  23,  11,
     10,  17,  23,  15,  20,  45,  44,  13,
     -8,  22,  24,  27,  26,  33,  26,   3,
    -18,  -4,  21,  24,  27,  23,   9, -11,
    -19,  -3,  11,  21,  23,  16,   7,  -9,
    -27, -11,   4,  13,  14,   4,  -5, -17,
    -53, -34, -21, -11, -28, -14, -24, -43,
];

// ===== BITWISE MASKS =====
const FILE_A: u64 = 0x0101010101010101;

// ===== HELPER FUNCTIONS =====

#[inline(always)]
fn square_idx(square: usize, color: Colore) -> usize {
    if color == Colore::Bianco {
        square ^ 56 // Flip verticale per il Bianco (se le tabelle sono orientate per il Nero o viceversa)
    } else {
        square
    }
}

// Rimuoviamo la necessità di tabelle duplicate in memoria, usiamo l'indicizzazione intelligente.
// Nota: Le tabelle PeSTO sopra sono definite dal punto di vista del BIANCO (rank 0-7).
// Quindi dobbiamo flippare l'indice per il NERO.
#[inline(always)]
fn get_psqt(table: &[i32; 64], square: usize, color: Colore) -> i32 {
    if color == Colore::Bianco {
        table[square ^ 56] // Flip per il bianco perché le costanti sopra sono definite visivamente "dall'alto"
    } else {
        table[square]
    }
}

struct EvaluationAccumulator {
    pub material: [i32; 2],
    pub psqt_mg: [i32; 2],
    pub psqt_eg: [i32; 2],
    pub mobility: [i32; 2],
    pub pawn_structure: [i32; 2],
    pub king_safety: [i32; 2],
    pub threats: [i32; 2],
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
            threats: [0, 0],
            bishop_pair: [0, 0],
            tempo: 0,
        }
    }
    
    fn final_score(&self, phase: i32) -> i32 {
        let mg_score = 
            (self.material[0] + self.psqt_mg[0] + self.mobility[0] + 
             self.pawn_structure[0] + self.king_safety[0] + 
             self.threats[0] + self.bishop_pair[0]) -
            (self.material[1] + self.psqt_mg[1] + self.mobility[1] + 
             self.pawn_structure[1] + self.king_safety[1] + 
             self.threats[1] + self.bishop_pair[1]);
        
        let eg_score = 
            (self.material[0] + self.psqt_eg[0] + self.mobility[0] * 2 + 
             self.pawn_structure[0] * 2 + self.king_safety[0] / 4 + 
             self.threats[0] + self.bishop_pair[0]) -
            (self.material[1] + self.psqt_eg[1] + self.mobility[1] * 2 + 
             self.pawn_structure[1] * 2 + self.king_safety[1] / 4 + 
             self.threats[1] + self.bishop_pair[1]);
        
        // Tapered evaluation: Interpolazione lineare tra MG e EG
        let tapered_score = ((mg_score as i64 * phase as i64 + eg_score as i64 * (24 - phase) as i64) / 24) as i32;
        
        tapered_score + self.tempo
    }
}

// ===== MAIN EVALUATION FUNCTION =====

pub fn valuta_posizione(scacchiera: &Scacchiera) -> i32 {
    let mut acc = EvaluationAccumulator::new();
    
    // 1. Fase del gioco (24 = apertura, 0 = endgame profondo)
    // Nota: PeSTO phase calculation
    let phase = calculate_game_phase(scacchiera);
    
    // 2. Materiale e PSQT
    evaluate_material_and_psqt(scacchiera, &mut acc);
    
    // 3. Struttura Pedoni (Isolati, Doppiati, Passati, Connessi)
    evaluate_pawns(scacchiera, &mut acc);
    
    // 4. Pezzi specifici (Torri su colonne aperte, Alfieri)
    evaluate_pieces_specific(scacchiera, &mut acc);
    
    // 5. Mobilità (Safe mobility)
    evaluate_mobility(scacchiera, &mut acc);
    
    // 6. Sicurezza Re
    if phase > 0 {
        evaluate_king_safety(scacchiera, &mut acc);
    }
    
    // 7. Bonus Tempo
    acc.tempo = if scacchiera.colore_attivo() == Colore::Bianco { 20 } else { -20 };
    
    // 8. Risultato finale interpolato
    acc.final_score(phase)
}

#[inline(always)]
fn calculate_game_phase(scacchiera: &Scacchiera) -> i32 {
    let mut phase_val = 0;
    
    for piece in &[Pezzo::Cavallo, Pezzo::Alfiere, Pezzo::Torre, Pezzo::Regina] {
        let count = scacchiera.bitboard_pezzo(*piece).count_ones() as i32;
        phase_val += count * GAME_PHASE_WEIGHTS[*piece as usize];
    }
    
    // Cap a 24
    min(phase_val, 24)
}

fn evaluate_material_and_psqt(scacchiera: &Scacchiera, acc: &mut EvaluationAccumulator) {
    for (color_idx, &color) in [Colore::Bianco, Colore::Nero].iter().enumerate() {
        for piece in &[Pezzo::Pedone, Pezzo::Cavallo, Pezzo::Alfiere, Pezzo::Torre, Pezzo::Regina, Pezzo::Re] {
            let bb = bitboard_pezzo_colore(scacchiera, *piece, color);
            let mut temp_bb = bb;
            
            while temp_bb != 0 {
                let square = temp_bb.trailing_zeros() as usize;
                temp_bb &= temp_bb - 1;
                
                // Materiale
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
                match piece {
                    Pezzo::Pedone => {
                        acc.psqt_mg[color_idx] += get_psqt(&PEDONE_MG, square, color);
                        acc.psqt_eg[color_idx] += get_psqt(&PEDONE_EG, square, color);
                    }
                    Pezzo::Cavallo => {
                        acc.psqt_mg[color_idx] += get_psqt(&CAVALLO_MG, square, color);
                        acc.psqt_eg[color_idx] += get_psqt(&CAVALLO_EG, square, color);
                    }
                    Pezzo::Alfiere => {
                        acc.psqt_mg[color_idx] += get_psqt(&ALFIERE_MG, square, color);
                        acc.psqt_eg[color_idx] += get_psqt(&ALFIERE_EG, square, color);
                    }
                    Pezzo::Torre => {
                        acc.psqt_mg[color_idx] += get_psqt(&TORRE_MG, square, color);
                        acc.psqt_eg[color_idx] += get_psqt(&TORRE_EG, square, color);
                    }
                    Pezzo::Regina => {
                        acc.psqt_mg[color_idx] += get_psqt(&REGINA_MG, square, color);
                        acc.psqt_eg[color_idx] += get_psqt(&REGINA_EG, square, color);
                    }
                    Pezzo::Re => {
                        acc.psqt_mg[color_idx] += get_psqt(&RE_MG, square, color);
                        acc.psqt_eg[color_idx] += get_psqt(&RE_EG, square, color);
                    }
                }
            }
        }
    }
}

fn evaluate_pawns(scacchiera: &Scacchiera, acc: &mut EvaluationAccumulator) {
    let white_pawns = bitboard_pezzo_colore(scacchiera, Pezzo::Pedone, Colore::Bianco);
    let black_pawns = bitboard_pezzo_colore(scacchiera, Pezzo::Pedone, Colore::Nero);

    for (color_idx, &color) in [Colore::Bianco, Colore::Nero].iter().enumerate() {
        let our_pawns = if color == Colore::Bianco { white_pawns } else { black_pawns };
        let their_pawns = if color == Colore::Bianco { black_pawns } else { white_pawns };
        let mut temp_bb = our_pawns;

        while temp_bb != 0 {
            let square = temp_bb.trailing_zeros() as usize;
            temp_bb &= temp_bb - 1;
            
            let file = square % 8;
            let rank = square / 8;

            // 1. Pedone Isolato (nessun pedone amico sulle colonne adiacenti)
            let file_mask_left = if file > 0 { FILE_A << (file - 1) } else { 0 };
            let file_mask_right = if file < 7 { FILE_A << (file + 1) } else { 0 };
            let isolated_mask = file_mask_left | file_mask_right;
            
            if (our_pawns & isolated_mask) == 0 {
                acc.pawn_structure[color_idx] -= 15;
            }

            // 2. Pedone Doppiato (pedoni amici sulla stessa colonna)
            // (Nota: controlliamo solo davanti per evitare double counting o usiamo count)
            let file_mask = FILE_A << file;
            if (our_pawns & file_mask).count_ones() > 1 {
                 // Piccola penalità, verrà contata per ogni pedone nella colonna
                acc.pawn_structure[color_idx] -= 8; 
            }

            // 3. Pedone Passato
            // Nessun pedone nemico sulla stessa colonna davanti, ne su quelle adiacenti davanti
            let forward_mask = if color == Colore::Bianco {
                !((1u64 << (square + 1)) - 1) // Bit da square+1 a 63
            } else {
                (1u64 << square) - 1 // Bit da 0 a square-1
            };

            let passed_check_mask = (file_mask | isolated_mask) & forward_mask;
            
            if (their_pawns & passed_check_mask) == 0 {
                let rank_bonus = if color == Colore::Bianco { rank as i32 } else { 7 - rank as i32 };
                // Bonus quadratico o esponenziale per avanzamento
                acc.pawn_structure[color_idx] += rank_bonus * rank_bonus * 5; 
            }

            // 4. Pedoni Connessi (Supportato da pedone amico)
            // Un pedone è connesso se è difeso da un altro pedone
            // Questo include catene di pedoni. Molto solido.
            let attacks = pawn_attacks(square, color);
            if (attacks & our_pawns) != 0 {
                // Questo pedone ne difende un altro?
                 // No, check if *is defended* by another. 
                 // Backwards check: squares that attack THIS square.
                 let defenders = pawn_attacks(square, color.opposto()); // Hack: attacchi inversi = difensori
                 if (defenders & our_pawns) != 0 {
                     let rank_bonus = if color == Colore::Bianco { rank as i32 } else { 7 - rank as i32 };
                     acc.pawn_structure[color_idx] += 10 + (rank_bonus * 4);
                 }
            }
        }
    }
}

fn evaluate_pieces_specific(scacchiera: &Scacchiera, acc: &mut EvaluationAccumulator) {
    let white_pawns = bitboard_pezzo_colore(scacchiera, Pezzo::Pedone, Colore::Bianco);
    let black_pawns = bitboard_pezzo_colore(scacchiera, Pezzo::Pedone, Colore::Nero);

    for (color_idx, &color) in [Colore::Bianco, Colore::Nero].iter().enumerate() {
        let our_pawns = if color == Colore::Bianco { white_pawns } else { black_pawns };
        let their_pawns = if color == Colore::Bianco { black_pawns } else { white_pawns };

        // --- TORRI ---
        let rooks = bitboard_pezzo_colore(scacchiera, Pezzo::Torre, color);
        let mut temp_rooks = rooks;
        while temp_rooks != 0 {
            let square = temp_rooks.trailing_zeros() as usize;
            temp_rooks &= temp_rooks - 1;
            let file_mask = FILE_A << (square % 8);

            // Colonna Semi-Aperta (nessun pedone nostro)
            if (our_pawns & file_mask) == 0 {
                acc.threats[color_idx] += 20;
                
                // Colonna Aperta (nessun pedone di nessuno)
                if (their_pawns & file_mask) == 0 {
                    acc.threats[color_idx] += 25; // Totale 45
                }
            }
        }

        // --- ALFIERI (Coppia) ---
        let bishops = bitboard_pezzo_colore(scacchiera, Pezzo::Alfiere, color);
        if bishops.count_ones() >= 2 {
            acc.bishop_pair[color_idx] += 50;
        }
    }
}

fn evaluate_mobility(scacchiera: &Scacchiera, acc: &mut EvaluationAccumulator) {
    let occupancy = scacchiera.occupazione();
    let white_pawns_attacks = get_pawn_attacks_set(scacchiera, Colore::Bianco);
    let black_pawns_attacks = get_pawn_attacks_set(scacchiera, Colore::Nero);

    for (color_idx, &color) in [Colore::Bianco, Colore::Nero].iter().enumerate() {
        // Area sicura: tutte le caselle tranne quelle attaccate dai pedoni nemici
        // (Per semplicità qui valutiamo mobilità generale, ma escludiamo caselle occupate dai nostri pedoni)
        let our_pieces = scacchiera.bitboard_colore(color);
        let their_pawn_attacks = if color == Colore::Bianco { black_pawns_attacks } else { white_pawns_attacks };
        
        // Non vogliamo muovere pezzi dove ci sono i nostri pezzi (ovvio) 
        // o dove i pedoni nemici mangiano (per pezzi leggeri/pesanti)
        let _safe_squares = !scacchiera.bitboard_colore(color) & !their_pawn_attacks;

        // --- CAVALLI ---
        let knights = bitboard_pezzo_colore(scacchiera, Pezzo::Cavallo, color);
        let mut temp_bb = knights;
        while temp_bb != 0 {
            let square = temp_bb.trailing_zeros() as usize;
            temp_bb &= temp_bb - 1;
            let attacks = knight_attacks(square);
            // Mobilità pura
            let mob = (attacks & !our_pieces).count_ones() as i32;
            acc.mobility[color_idx] += mob * 4;
        }

        // --- ALFIERI ---
        let bishops = bitboard_pezzo_colore(scacchiera, Pezzo::Alfiere, color);
        temp_bb = bishops;
        while temp_bb != 0 {
            let square = temp_bb.trailing_zeros() as usize;
            temp_bb &= temp_bb - 1;
            let attacks = bishop_attacks(square, occupancy);
            let mob = (attacks & !our_pieces).count_ones() as i32;
            acc.mobility[color_idx] += mob * 3;
        }

        // --- TORRI ---
        let rooks = bitboard_pezzo_colore(scacchiera, Pezzo::Torre, color);
        temp_bb = rooks;
        while temp_bb != 0 {
            let square = temp_bb.trailing_zeros() as usize;
            temp_bb &= temp_bb - 1;
            let attacks = rook_attacks(square, occupancy);
            let mob = (attacks & !our_pieces).count_ones() as i32;
            acc.mobility[color_idx] += mob * 2;
        }
        
        // --- REGINA ---
        // La regina ha meno valore marginale per mobilità perché ha già tanto raggio
        let queens = bitboard_pezzo_colore(scacchiera, Pezzo::Regina, color);
        temp_bb = queens;
        while temp_bb != 0 {
            let square = temp_bb.trailing_zeros() as usize;
            temp_bb &= temp_bb - 1;
            // Uniamo attacchi
            let attacks_diag = bishop_attacks(square, occupancy);
            let attacks_orth = rook_attacks(square, occupancy);
            let mob = ((attacks_diag | attacks_orth) & !our_pieces).count_ones() as i32;
            acc.mobility[color_idx] += mob;
        }
    }
}

// Ritorna bitboard di tutte le caselle attaccate dai pedoni di un colore
fn get_pawn_attacks_set(scacchiera: &Scacchiera, color: Colore) -> Bitboard {
    let pawns = bitboard_pezzo_colore(scacchiera, Pezzo::Pedone, color);
    let mut attacks = 0;
    if color == Colore::Bianco {
        // Attacchi a sinistra (shift 7 se non file H, ma bitboard gestisce overflow shiftando tutto)
        // Nota: logica semplificata, meglio usare lookup o shift precisi
        let not_a_file = !0x0101010101010101u64;
        let not_h_file = !0x8080808080808080u64;
        attacks |= (pawns & not_a_file) << 7; // Nord-Ovest
        attacks |= (pawns & not_h_file) << 9; // Nord-Est
    } else {
        let not_a_file = !0x0101010101010101u64;
        let not_h_file = !0x8080808080808080u64;
        attacks |= (pawns & not_h_file) >> 7; // Sud-Est
        attacks |= (pawns & not_a_file) >> 9; // Sud-Ovest
    }
    attacks
}

fn evaluate_king_safety(scacchiera: &Scacchiera, acc: &mut EvaluationAccumulator) {
    let white_pawns = bitboard_pezzo_colore(scacchiera, Pezzo::Pedone, Colore::Bianco);
    let black_pawns = bitboard_pezzo_colore(scacchiera, Pezzo::Pedone, Colore::Nero);

    for (color_idx, &color) in [Colore::Bianco, Colore::Nero].iter().enumerate() {
        let king_bb = bitboard_pezzo_colore(scacchiera, Pezzo::Re, color);
        if king_bb == 0 { continue; }
        let king_sq = king_bb.trailing_zeros() as usize;
        
        let our_pawns = if color == Colore::Bianco { white_pawns } else { black_pawns };
        let their_pawns = if color == Colore::Bianco { black_pawns } else { white_pawns };

        // 1. Pawn Shield (Scudo Pedoni)
        // Controlliamo le 3 caselle davanti al re
        let rank = king_sq / 8;
        let file = king_sq % 8;
        let shield_rank = if color == Colore::Bianco { min(rank + 1, 7) } else { max(rank as i32 - 1, 0) as usize };
        
        let mut shield_score = 0;
        
        for f_offset in -1..=1 {
            let check_file = file as i32 + f_offset;
            if check_file >= 0 && check_file <= 7 {
                let shield_sq = shield_rank * 8 + check_file as usize;
                let shield_bit = 1u64 << shield_sq;
                let file_mask = FILE_A << check_file;

                // Pedone presente a scudo?
                if (our_pawns & shield_bit) != 0 {
                    shield_score += 20; // Ottimo
                } else if (our_pawns & ((if color == Colore::Bianco {shield_bit << 8} else {shield_bit >> 8}))) != 0 {
                    shield_score += 10; // Avanzato di 1
                } else {
                    // Colonna aperta vicino al re? Pericolo!
                    if (our_pawns & file_mask) == 0 && (their_pawns & file_mask) == 0 {
                         shield_score -= 30; // Colonna completamente aperta
                    } else if (our_pawns & file_mask) == 0 {
                         shield_score -= 15; // Colonna semi-aperta
                    }
                }
            }
        }
        
        acc.king_safety[color_idx] += shield_score;
        
        // 2. Penalità Re Esposto (Attacchi vicini)
        // Calcolo semplificato: quanti attacchi nemici colpiscono l'area del re?
        // (Per performance usiamo una stima basata sugli attacchi calcolati in Mobility se possibile, 
        // ma qui per semplicità omettiamo il ricalcolo pesante degli attacchi nemici specifici sul re 
        // in favore dello shield score e PeSTO che penalizza il re al centro)
    }
}

// ===== PUBLIC INTERFACE SIMPLIFIED =====

pub fn valuta_posizione_semplice(scacchiera: &Scacchiera) -> i32 {
    // Utile per debug
    let mut score = 0;
    for &color in &[Colore::Bianco, Colore::Nero] {
        let sign = if color == Colore::Bianco { 1 } else { -1 };
        for piece in &[Pezzo::Pedone, Pezzo::Cavallo, Pezzo::Alfiere, Pezzo::Torre, Pezzo::Regina] {
             let count = bitboard_pezzo_colore(scacchiera, *piece, color).count_ones() as i32;
             let val = match piece {
                 Pezzo::Pedone => PEDONE_VAL,
                 Pezzo::Cavallo => CAVALLO_VAL,
                 Pezzo::Alfiere => ALFIERE_VAL,
                 Pezzo::Torre => TORRE_VAL,
                 Pezzo::Regina => REGINA_VAL,
                 _ => 0
             };
             score += sign * count * val;
        }
    }
    score
}