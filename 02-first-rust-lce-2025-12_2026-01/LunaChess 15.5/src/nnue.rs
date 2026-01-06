use crate::board::{Colore, Pezzo, Scacchiera};

// --- STRUTTURA FAKE PER COMPILARE BOARD.RS ---
// Board.rs ha bisogno di vedere questi vettori per gestire gli aggiornamenti incrementali.
// Noi li riempiremo di zeri, così l'accumulatore sarà sempre zero,
// ma tanto la funzione evaluate_nnue sotto li ignorerà completamente.

#[derive(Clone)]
pub struct Network {
    pub feature_weights: Vec<i16>,
    pub feature_bias: Vec<i16>,
    // Output weights non servono a board.rs, quindi possiamo ometterli o lasciarli
}

impl Network {
    pub fn carica(_path: &str) -> Self {
        // Creiamo vettori pieni di zero della dimensione giusta per non far crashare board.rs
        // board.rs si aspetta NNUE_LAYER1_SIZE = 256
        let hidden = 256; 
        let inputs = 768; // 64 case * 12 pezzi

        Network {
            // Riempiamo di zeri. Board.rs farà calcoli inutili su questi zeri,
            // ma non crasherà e non darà errori di compilazione.
            feature_weights: vec![0; inputs * hidden],
            feature_bias: vec![0; hidden],
        }
    }
}

// Questa serve a board.rs per sapere quale peso aggiornare
pub fn get_feature_index(pezzo: Pezzo, colore: Colore, sq: usize) -> usize {
    let piece_offset = pezzo.indice();
    let color_offset = if colore == Colore::Bianco { 0 } else { 6 };
    (color_offset + piece_offset) * 64 + sq
}

// --- VALUTAZIONE CLASSICA (HCE - Hand Crafted Evaluation) ---
// Questa funzione IGNORA la rete neurale (che è piena di zeri)
// e calcola il punteggio usando regole di scacchi vere.

pub fn evaluate_nnue(board: &Scacchiera, _net: &Network) -> i32 {
    let mut score = 0;
    
    // 1. Calcolo Materiale e Posizione
    score += evaluate_side(board, Colore::Bianco);
    score -= evaluate_side(board, Colore::Nero);

    // 2. Prospettiva (Negamax)
    // Se tocca al nero muovere, il punteggio deve essere negativo dal suo punto di vista
    if board.turno == Colore::Nero {
        -score
    } else {
        score
    }
}

fn evaluate_side(board: &Scacchiera, c: Colore) -> i32 {
    let mut score = 0;
    let pieces = board.colori[c.indice()];

    // --- VALORI MATERIALE ---
    const VAL_PAWN: i32 = 100;
    const VAL_KNIGHT: i32 = 320;
    const VAL_BISHOP: i32 = 330;
    const VAL_ROOK: i32 = 500;
    const VAL_QUEEN: i32 = 900;
    
    // --- TABELLE PEZZO-QUADRO (PSQT) ---
    // Definiscono dove i pezzi devono andare.
    // Definite per il Bianco (dal basso verso l'alto).
    
    const PAWN_TABLE: [i32; 64] = [
         0,  0,  0,  0,  0,  0,  0,  0,
        50, 50, 50, 50, 50, 50, 50, 50,
        10, 10, 20, 30, 30, 20, 10, 10,
         5,  5, 10, 25, 25, 10,  5,  5,
         0,  0,  0, 20, 20,  0,  0,  0,
         5, -5,-10,  0,  0,-10, -5,  5,
         5, 10, 10,-20,-20, 10, 10,  5,
         0,  0,  0,  0,  0,  0,  0,  0
    ];

    const KNIGHT_TABLE: [i32; 64] = [
        -50,-40,-30,-30,-30,-30,-40,-50,
        -40,-20,  0,  0,  0,  0,-20,-40,
        -30,  0, 10, 15, 15, 10,  0,-30,
        -30,  5, 15, 20, 20, 15,  5,-30,
        -30,  0, 15, 20, 20, 15,  0,-30,
        -30,  5, 10, 15, 15, 10,  5,-30,
        -40,-20,  0,  5,  5,  0,-20,-40,
        -50,-40,-30,-30,-30,-30,-40,-50,
    ];

    const BISHOP_TABLE: [i32; 64] = [
        -20,-10,-10,-10,-10,-10,-10,-20,
        -10,  0,  0,  0,  0,  0,  0,-10,
        -10,  0,  5, 10, 10,  5,  0,-10,
        -10,  5,  5, 10, 10,  5,  5,-10,
        -10,  0, 10, 10, 10, 10,  0,-10,
        -10, 10, 10, 10, 10, 10, 10,-10,
        -10,  5,  0,  0,  0,  0,  5,-10,
        -20,-10,-10,-10,-10,-10,-10,-20,
    ];

    const ROOK_TABLE: [i32; 64] = [
          0,  0,  0,  0,  0,  0,  0,  0,
          5, 10, 10, 10, 10, 10, 10,  5,
         -5,  0,  0,  0,  0,  0,  0, -5,
         -5,  0,  0,  0,  0,  0,  0, -5,
         -5,  0,  0,  0,  0,  0,  0, -5,
         -5,  0,  0,  0,  0,  0,  0, -5,
         -5,  0,  0,  0,  0,  0,  0, -5,
          0,  0,  0,  5,  5,  0,  0,  0
    ];

    const QUEEN_TABLE: [i32; 64] = [
        -20,-10,-10, -5, -5,-10,-10,-20,
        -10,  0,  0,  0,  0,  0,  0,-10,
        -10,  0,  5,  5,  5,  5,  0,-10,
         -5,  0,  5,  5,  5,  5,  0, -5,
          0,  0,  5,  5,  5,  5,  0, -5,
        -10,  5,  5,  5,  5,  5,  0,-10,
        -10,  0,  5,  0,  0,  0,  0,-10,
        -20,-10,-10, -5, -5,-10,-10,-20
    ];

    // Indice 63 è H8, 0 è A1
    for sq in 0..64 {
        let bit = 1u64 << sq;
        if (pieces & bit) == 0 { continue; }

        // Calcolo indice per la tabella
        // Se Bianco: Rank 7 è in alto (indice basso nell'array PSQT visivo), Rank 0 è in basso (indice alto)
        // Le tabelle sopra sono scritte visivamente: 
        // Riga 0 dell'array = Rank 8 scacchiera
        // Riga 7 dell'array = Rank 1 scacchiera
        
        let rank = sq / 8; // 0..7 (0=Rank1, 7=Rank8)
        let file = sq % 8; // 0..7 (0=A, 7=H)
        
        let table_idx = if c == Colore::Bianco {
            // Bianco: Rank 0 mappa all'ultima riga dell'array (indice 56-63)
            (7 - rank) * 8 + file
        } else {
            // Nero: Speculare. Rank 7 (suo Rank 1) mappa all'ultima riga dell'array.
            rank * 8 + file
        };

        if (board.pezzi[0] & bit) != 0 { score += VAL_PAWN + PAWN_TABLE[table_idx]; }
        else if (board.pezzi[1] & bit) != 0 { score += VAL_KNIGHT + KNIGHT_TABLE[table_idx]; }
        else if (board.pezzi[2] & bit) != 0 { score += VAL_BISHOP + BISHOP_TABLE[table_idx]; }
        else if (board.pezzi[3] & bit) != 0 { score += VAL_ROOK + ROOK_TABLE[table_idx]; }
        else if (board.pezzi[4] & bit) != 0 { score += VAL_QUEEN + QUEEN_TABLE[table_idx]; }
        // Re (non contiamo materiale per non sballare tutto, solo posizione)
        // Per il Re bisognerebbe una tabella, ma per ora lo lasciamo neutro o aggiungiamo logica dopo
    }
    
    score
}