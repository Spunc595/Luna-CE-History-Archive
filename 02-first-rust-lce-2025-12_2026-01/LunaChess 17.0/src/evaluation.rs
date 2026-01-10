use crate::board::{Scacchiera, Colore, Pezzo};

// Valori dei pezzi per Mediogioco (MG) e Finale (EG)
const MG_VAL: [i32; 6] = [100, 320, 330, 500, 900, 0];
const EG_VAL: [i32; 6] = [120, 310, 340, 520, 950, 0];

// Tabelle Posizionali (PST) differenziate
// Il Re in finale (EG) deve essere incoraggiato ad andare verso il centro
const RE_MG: [i32; 64] = [
    -30, -40, -40, -50, -50, -40, -40, -30,
    -30, -40, -40, -50, -50, -40, -40, -30,
    -30, -40, -40, -50, -50, -40, -40, -30,
    -30, -40, -40, -50, -50, -40, -40, -30,
    -20, -30, -30, -40, -40, -30, -30, -20,
    -10, -20, -20, -20, -20, -20, -20, -10,
     20,  20,   0,   0,   0,   0,  20,  20,
     20,  30,  10,   0,   0,  10,  30,  20,
];

const RE_EG: [i32; 64] = [
    -50, -40, -30, -20, -20, -30, -40, -50,
    -30, -20, -10,   0,   0, -10, -20, -30,
    -30, -10,  20,  30,  30,  20, -10, -30,
    -30, -10,  30,  40,  40,  30, -10, -30,
    -30, -10,  30,  40,  40,  30, -10, -30,
    -30, -10,  20,  30,  30,  20, -10, -30,
    -30, -30,   0,   0,   0,   0, -30, -30,
    -50, -30, -30, -30, -30, -30, -30, -50,
];

// Pesi per determinare la fase (Pawn=0, Knight=1, Bishop=1, Rook=2, Queen=4)
const PHASE_WEIGHTS: [i32; 6] = [0, 1, 1, 2, 4, 0];
const MAX_PHASE: i32 = 24; // 2*(4*1 + 2*1 + 2*2 + 1*4)

pub fn evaluate(board: &Scacchiera) -> i32 {
    let mut mg = [0; 2];
    let mut eg = [0; 2];
    let mut game_phase = 0;

    for colore in 0..2 {
        for pezzo_tipo in 0..6 {
            let mut bb = board.pezzi[pezzo_tipo] & board.colori[colore];
            while bb != 0 {
                let sq = bb.trailing_zeros() as usize;
                let rel_sq = if colore == 0 { sq ^ 56 } else { sq };

                // Materiale
                mg[colore] += MG_VAL[pezzo_tipo];
                eg[colore] += EG_VAL[pezzo_tipo];
                
                // Fase del gioco
                game_phase += PHASE_WEIGHTS[pezzo_tipo];

                // PST specifiche per il Re
                if pezzo_tipo == Pezzo::Re.indice() {
                    mg[colore] += RE_MG[rel_sq];
                    eg[colore] += RE_EG[rel_sq];
                }
                
                // Qui andrebbero aggiunte PST per Pedoni, Cavalli, etc.
                
                bb &= bb - 1;
            }
        }
    }

    // Calcolo punteggi MG ed EG
    let mg_score = mg[0] - mg[1];
    let eg_score = eg[0] - eg[1];

    // Interpolazione della fase
    let phase = (game_phase * 256 + (MAX_PHASE / 2)) / MAX_PHASE;
    let mut score = ((mg_score * phase) + (eg_score * (256 - phase))) / 256;

    if board.turno == Colore::Nero { score = -score; }
    score
}