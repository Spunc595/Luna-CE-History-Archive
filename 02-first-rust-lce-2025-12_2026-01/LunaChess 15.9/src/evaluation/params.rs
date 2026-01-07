// src/evaluation/params.rs

// Valori dei pezzi (in centipawns: 100 = 1 pedone)
// P, N, B, R, Q, K
pub const MATERIAL_VALUES: [i16; 6] = [100, 320, 330, 500, 900, 20000];

// ==========================================
// TABELLE PST (Piece-Square Tables)
// Orientate per il BIANCO (Rank 8 in alto, Rank 1 in basso)
// ==========================================

pub const PAWN_TABLE: [i16; 64] = [
    0,  0,  0,  0,  0,  0,  0,  0,
   90, 90, 90, 90, 90, 90, 90, 90, // Rank 7: Quasi promozione (aumentato da 50)
   10, 10, 20, 30, 30, 20, 10, 10, // Rank 6
    5,  5, 10, 25, 25, 10,  5,  5, // Rank 5
    0,  0,  0, 20, 20,  0,  0,  0, // Rank 4
    5, -5,-10,  0,  0,-10, -5,  5, // Rank 3
    5, 10, 10,-20,-20, 10, 10,  5, // Rank 2
    0,  0,  0,  0,  0,  0,  0,  0, // Rank 1
];

pub const KNIGHT_TABLE: [i16; 64] = [
   -50,-40,-30,-30,-30,-30,-40,-50,
   -40,-20,  0,  0,  0,  0,-20,-40,
   -30,  0, 10, 15, 15, 10,  0,-30,
   -30,  5, 15, 20, 20, 15,  5,-30,
   -30,  0, 15, 20, 20, 15,  0,-30,
   -30,  5, 10, 15, 15, 10,  5,-30,
   -40,-20,  0,  5,  5,  0,-20,-40,
   -50,-40,-30,-30,-30,-30,-40,-50,
];

pub const BISHOP_TABLE: [i16; 64] = [
   -20,-10,-10,-10,-10,-10,-10,-20,
   -10,  0,  0,  0,  0,  0,  0,-10,
   -10,  0,  5, 10, 10,  5,  0,-10,
   -10,  5,  5, 10, 10,  5,  5,-10,
   -10,  0, 10, 10, 10, 10,  0,-10,
   -10, 10, 10, 10, 10, 10, 10,-10,
   -10,  5,  0,  0,  0,  0,  5,-10,
   -20,-10,-10,-10,-10,-10,-10,-20,
];

pub const ROOK_TABLE: [i16; 64] = [
    0,  0,  0,  0,  0,  0,  0,  0,
    5, 10, 10, 10, 10, 10, 10,  5,
   -5,  0,  0,  0,  0,  0,  0, -5,
   -5,  0,  0,  0,  0,  0,  0, -5,
   -5,  0,  0,  0,  0,  0,  0, -5,
   -5,  0,  0,  0,  0,  0,  0, -5,
   -5,  0,  0,  0,  0,  0,  0, -5,
    0,  0,  0,  5,  5,  0,  0,  0,
];

pub const QUEEN_TABLE: [i16; 64] = [
   -20,-10,-10, -5, -5,-10,-10,-20,
   -10,  5,  5,  5,  5,  5,  5,-10, // Più attiva
   -10,  5, 10, 10, 10, 10,  5,-10,
    -5,  5, 10, 15, 15, 10,  5, -5, // Centralizzazione sicura
     0,  5, 10, 15, 15, 10,  5, -5,
   -10,  5, 10, 10, 10, 10,  5,-10,
   -10,  0,  5,  0,  0,  0,  0,-10,
   -20,-10,-10, -5, -5,-10,-10,-20,
];

pub const KING_TABLE: [i16; 64] = [
   -30,-40,-40,-50,-50,-40,-40,-30,
   -30,-40,-40,-50,-50,-40,-40,-30,
   -30,-40,-40,-50,-50,-40,-40,-30,
   -30,-40,-40,-50,-50,-40,-40,-30,
   -20,-30,-30,-40,-40,-30,-30,-20,
   -10,-20,-20,-20,-20,-20,-20,-10,
    20, 20,  0,  0,  0,  0, 20, 20,
    20, 30, 10,  0,  0, 10, 30, 20,
];

// src/evaluation/params.rs

// PST per il Re in FINALE (il Re deve andare verso il centro)
pub const KING_ENDGAME_TABLE: [i16; 64] = [
    -50,-30,-30,-30,-30,-30,-30,-50,
    -30,-10, 10, 10, 10, 10,-10,-30,
    -30, 10, 20, 20, 20, 20, 10,-30,
    -30, 10, 20, 30, 30, 20, 10,-30,
    -30, 10, 20, 30, 30, 20, 10,-30,
    -30, 10, 20, 20, 20, 20, 10,-30,
    -30,-10, 10, 10, 10, 10,-10,-30,
    -50,-30,-30,-30,-30,-30,-30,-50,
];

// ==========================================
// PARAMETRI DINAMICI
// ==========================================

// Bonus per i Pedoni Passati (Passed Pawns) basato sulla traversa (Rank)
// Indici: 0=Rank 1, 1=Rank 2, ... 7=Rank 8
pub const PASSED_PAWN_ENDGAME_BONUS: [i16; 8] = [0, 10, 25, 50, 85, 140, 220, 0];

// Bonus se un pedone passato è supportato da un altro pedone
pub const PROTECTED_PASSED_PAWN_BONUS: i16 = 25;

// Bonus per la mobilità della Donna (punti per ogni casa sicura controllata)
pub const QUEEN_MOBILITY_WEIGHT: i16 = 2;

// Penalità se la Donna è intrappolata o attaccabile da pezzi minori
pub const QUEEN_EXPOSED_PENALTY: i16 = -30;