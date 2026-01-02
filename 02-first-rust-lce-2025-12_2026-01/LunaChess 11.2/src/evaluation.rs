use crate::board::{Scacchiera, Pezzo, Colore};

// --- VALORI MATERIALE ---
// [Pedone, Cavallo, Alfiere, Torre, Regina, Re]
const MG_VAL: [i32; 6] = [ 82, 337, 365, 477, 1025, 0 ];
const EG_VAL: [i32; 6] = [ 94, 281, 297, 512,  936, 0 ];

// --- BONUS PEDONI PASSATI ---
const PASSED_MG: [i32; 8] = [ 0, 5, 10, 20, 35, 60, 100, 0 ];
const PASSED_EG: [i32; 8] = [ 0, 10, 20, 40, 70, 130, 200, 0 ];

// --- PENALITÀ SICUREZZA RE (Middlegame) ---
// Penalità se il Re è su una colonna semi-aperta (nessun pedone amico davanti)
const KING_SEMI_OPEN_FILE_PENALTY: i32 = 25;
// Penalità se il Re è su una colonna completamente aperta (nessun pedone)
const KING_OPEN_FILE_PENALTY: i32 = 50;

// --- PIECE-SQUARE TABLES (PST) ---
#[rustfmt::skip]
const MG_TABLE: [[i32; 64]; 6] = [
    // Pedone
    [0,0,0,0,0,0,0,0, 98,134,61,95,68,126,34,-11, -6,7,26,31,65,56,25,-20, -40,13,6,21,23,12,17,-40, -40,-2,-5,12,17,6,10,-40, -26,-4,-4,-10,3,3,33,-12, -35,-1,-20,-23,-15,24,38,-22, 0,0,0,0,0,0,0,0],
    // Cavallo
    [-167,-89,-34,-49,61,-97,-15,-107, -73,-41,72,36,23,62,7,-17, -47,60,37,65,84,129,73,44, -9,17,19,53,37,69,18,22, -13,4,16,13,28,19,21,-8, -23,-9,12,10,19,17,25,-16, -29,-53,-12,-3,-1,18,-14,-19, -105,-21,-58,-33,-17,-28,-19,-23],
    // Alfiere
    [-29,4,-82,-37,-25,-42,7,-8, -26,16,-18,-13,30,59,18,-47, -16,37,43,40,35,50,37,-2, -4,5,19,50,37,37,7,-2, -6,13,13,26,34,12,10,4, 0,15,15,15,14,27,18,10, 4,15,16,0,7,21,33,1, -33,-3,-14,-21,-13,-12,-39,-21],
    // Torre
    [32,42,32,51,63,9,31,43, 27,32,58,62,80,67,26,44, -5,19,26,36,17,45,61,16, -24,-11,7,26,24,35,-8,-20, -36,-26,-12,-1,9,-7,6,-23, -45,-25,-16,-17,3,0,-5,-33, -44,-16,-20,-9,-1,11,-6,-71, -19,-13,1,17,16,7,-37,-26],
    // Regina
    [-50,-40,-30,-20,-20,-30,-40,-50, -40,-20,0,0,0,0,-20,-40, -30,0,10,10,10,10,0,-30, -20,0,10,15,15,10,0,-20, -20,0,10,15,15,10,0,-20, -30,5,10,10,10,10,5,-30, -40,-20,0,5,5,0,-20,-40, -50,-40,-30,-20,-20,-30,-40,-50],
    // Re (Ho reso il centro molto più pericoloso: -70 invece di -50)
    [-65,23,16,-15,-56,-34,2,13, 29,-1,-20,-7,-8,-4,-38,-29, -9,24,2,16,-20,6,22,-22, -17,-20,-12,-27,-30,-25,-14,-36, -49,-1,-27,-39,-46,-44,-33,-51, -14,-14,-22,-46,-44,-30,-15,-27, 1,7,-8,-70,-70,-16,9,8, -15,36,12,-70,-70,-28,24,14]
];

#[rustfmt::skip]
const EG_TABLE: [[i32; 64]; 6] = [
    // Pedone
    [0,0,0,0,0,0,0,0, 178,173,158,134,147,132,165,187, 94,100,85,67,56,53,82,84, 32,24,13,5,-2,4,17,17, 13,9,-3,-7,-7,-8,3,-1, 4,7,-6,1,0,-5,-1,-8, 13,8,8,10,13,0,2,-7, 0,0,0,0,0,0,0,0],
    // Cavallo
    [-58,-38,-13,-28,-31,-27,-63,-99, -25,-8,-25,-2,-9,-25,-24,-52, -24,-20,10,9,-1,-9,-19,-41, -17,3,22,22,22,11,8,-18, -18,-6,16,25,16,17,4,-18, -23,-3,-1,15,10,-3,-20,-22, -42,-20,-10,-5,-2,-20,-23,-44, -29,-51,-23,-15,-22,-18,-50,-64],
    // Alfiere
    [-14,-21,-11,-8,-7,-9,-17,-24, -8,-4,7,-12,-3,-13,-4,-14, 2,-8,0,-1,-2,6,0,4, -3,9,12,9,14,10,3,2, -6,3,13,19,7,10,-3,-9, -12,-3,5,10,5,6,0,-7, -15,-10,-10,-5,-4,0,-8,-23, -23,-9,-23,-5,-9,-16,-5,-17],
    // Torre
    [13,10,18,15,12,12,8,5, 11,13,13,11,-3,3,8,3, 7,7,7,5,4,-3,-5,-3, 4,3,13,1,2,1,-1,2, 3,5,8,4,-5,-6,-8,-11, -4,0,-5,-1,-7,-12,-8,-16, -6,-6,0,2,-9,-9,-11,-3, -9,2,3,-1,-5,-13,4,-20],
    // Regina
    [-9,22,22,27,27,19,10,20, -17,20,32,41,58,25,30,0, -20,6,9,49,47,35,19,9, 3,22,24,45,57,40,57,36, -18,28,19,47,31,34,39,23, -16,-27,15,6,9,17,10,5, -22,-23,-30,-16,-16,23,0,-36, -14,-5,-15,-10,-10,-10,-10,-2],
    // Re
    [-74,-35,-18,-18,-11,15,4,-17, -12,17,14,17,17,38,23,11, 10,17,23,15,20,45,44,13, -8,22,24,27,26,33,26,3, -18,-4,21,24,27,23,9,-11, -19,-3,11,21,23,16,7,-9, -27,-11,4,13,14,4,-5,-17, -53,-34,-21,-11,-28,-14,-24,-43]
];

const FILE_A: u64 = 0x0101010101010101;

pub fn valuta_posizione(s: &Scacchiera) -> i32 {
    let mut mg = [0, 0];
    let mut eg = [0, 0];
    let mut phase = 0;

    let occ_w = s.bitboard_colore(Colore::Bianco);
    let occ_b = s.bitboard_colore(Colore::Nero);
    let wp = s.bitboard_pezzo_colore(Pezzo::Pedone, Colore::Bianco);
    let bp = s.bitboard_pezzo_colore(Pezzo::Pedone, Colore::Nero);

    for sq in 0..64 {
        let bit = 1u64 << sq;
        let colore = if (occ_w & bit) != 0 { Some(Colore::Bianco) } 
                      else if (occ_b & bit) != 0 { Some(Colore::Nero) } else { None };

        if let Some(c) = colore {
            if let Some(p) = s.get_piece_at(sq) {
                let pi = p.indice();
                let ci = c.indice();
                
                // Aggiornamento fase (Tapered Eval)
                phase += match p {
                    Pezzo::Cavallo | Pezzo::Alfiere => 1,
                    Pezzo::Torre => 2,
                    Pezzo::Regina => 4,
                    _ => 0,
                };

                let pst_idx = if c == Colore::Bianco { sq ^ 56 } else { sq };
                
                let mut bonus_mg = 0;
                let mut bonus_eg = 0;

                // --- 1. PEDONI PASSATI ---
                if p == Pezzo::Pedone {
                    let file = sq % 8;
                    let rank = sq / 8;
                    
                    let mut file_mask = FILE_A << file;
                    if file > 0 { file_mask |= FILE_A << (file - 1); }
                    if file < 7 { file_mask |= FILE_A << (file + 1); }

                    let is_passed = if c == Colore::Bianco {
                        let forward_mask = !((1u64 << ((rank + 1) * 8)) - 1);
                        let passed_mask = forward_mask & file_mask;
                        (passed_mask & bp) == 0
                    } else {
                        let backward_mask = (1u64 << (rank * 8)) - 1;
                        let passed_mask = backward_mask & file_mask;
                        (passed_mask & wp) == 0
                    };

                    if is_passed {
                        let relative_rank = if c == Colore::Bianco { rank } else { 7 - rank };
                        bonus_mg += PASSED_MG[relative_rank];
                        bonus_eg += PASSED_EG[relative_rank];
                    }
                }

                // --- 2. SICUREZZA RE (KING SAFETY) ---
                // Penalizziamo il Re se è esposto nel mediogioco (MG)
                if p == Pezzo::Re {
                    let file = sq % 8;
                    let file_mask = FILE_A << file;
                    
                    // Controlla se ci sono pedoni amici sulla colonna del Re (Scudo frontale)
                    let friendly_pawns_in_front = if c == Colore::Bianco {
                         (file_mask & wp) != 0
                    } else {
                         (file_mask & bp) != 0
                    };

                    // Controlla se ci sono pedoni nemici sulla colonna del Re
                    let enemy_pawns_on_file = if c == Colore::Bianco {
                        (file_mask & bp) != 0
                    } else {
                        (file_mask & wp) != 0
                    };

                    if !friendly_pawns_in_front {
                        // Se non ha pedoni amici davanti, è pericoloso
                        if !enemy_pawns_on_file {
                            // Colonna completamente aperta: PERICOLO ESTREMO
                            bonus_mg -= KING_OPEN_FILE_PENALTY;
                        } else {
                            // Colonna semi-aperta: Pericolo moderato
                            bonus_mg -= KING_SEMI_OPEN_FILE_PENALTY;
                        }
                    }
                }

                mg[ci] += MG_VAL[pi] + MG_TABLE[pi][pst_idx] + bonus_mg;
                eg[ci] += EG_VAL[pi] + EG_TABLE[pi][pst_idx] + bonus_eg;
            }
        }
    }

    let p_clamped = phase.min(24);
    let mg_diff = mg[0] - mg[1];
    let eg_diff = eg[0] - eg[1];
    
    let score = (mg_diff * p_clamped + eg_diff * (24 - p_clamped)) / 24;

    if s.turno() == Colore::Bianco { score } else { -score }
}