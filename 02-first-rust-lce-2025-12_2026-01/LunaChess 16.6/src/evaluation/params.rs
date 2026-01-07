// src/evaluation/params.rs

pub const MATERIAL_VALUES: [i16; 6] = [100, 320, 330, 500, 900, 20000];

// PST Pedone: Penalizziamo drasticamente a3/b3/g3/h3 per evitare la "apatia" dello sviluppo.
// I valori centrali (d4, e4) sono i più alti per incoraggiare il controllo del centro.
pub const PAWN_TABLE: [i16; 64] = [
    0,  0,  0,  0,  0,  0,  0,  0,
   70, 70, 70, 70, 70, 70, 70, 70,
    5, 10, 20, 30, 30, 20, 10,  5,
    0,  5, 15, 25, 25, 15,  5,  0,
  -25, -15, 15, 25, 25, 15, -15, -25, // Rank 4: Pesante penalità bordi
  -35, -25,  5, 15, 15,  5, -25, -35, // Rank 3: Muovere a3/h3 ora costa -35 punti
    5, 10, 10,-20,-20, 10, 10,  5,
    0,  0,  0,  0,  0,  0,  0,  0,
];

// PST Cavallo: Aumentato il differenziale tra centro e bordi.
pub const KNIGHT_TABLE: [i16; 64] = [
   -60,-50,-40,-40,-40,-40,-50,-60,
   -50,-20,  5, 10, 10,  5,-20,-50,
   -40,  5, 20, 25, 25, 20,  5,-40,
   -40, 10, 25, 35, 35, 25, 10,-40,
   -40, 10, 25, 35, 35, 25, 10,-40,
   -40,  5, 20, 25, 25, 20,  5,-40,
   -50,-20,  5, 15, 15,  5,-20,-50,
   -60,-50,-40,-40,-40,-40,-50,-60,
];

// PST Alfiere: Premiato lo sviluppo e le diagonali lunghe.
pub const BISHOP_TABLE: [i16; 64] = [
   -20,-10,-10,-10,-10,-10,-10,-20,
   -10,  5,  0,  0,  0,  0,  5,-10,
   -10, 10, 10, 15, 15, 10, 10,-10,
   -10, 10, 15, 20, 20, 15, 10,-10,
   -10,  5, 15, 20, 20, 15,  5,-10,
   -10,  5, 10, 15, 15, 10,  5,-10,
   -10, 15,  5,  5,  5,  5, 15,-10, // Premiato il fianchetto (b2/g2)
   -20,-10,-10,-10,-10,-10,-10,-20,
];

// PST Torre: Incoraggiata la settima traversa e le colonne centrali.
pub const ROOK_TABLE: [i16; 64] = [
    0,  0,  0,  5,  5,  0,  0,  0,
    5, 10, 10, 10, 10, 10, 10,  5,
   -5,  0,  0,  0,  0,  0,  0, -5,
   -5,  0,  0,  0,  0,  0,  0, -5,
   -5,  0,  0,  0,  0,  0,  0, -5,
   -5,  0,  0,  0,  0,  0,  0, -5,
   -5,  0,  0,  0,  0,  0,  0, -5,
    0,  0,  0,  2,  2,  0,  0,  0,
];

// PST Regina: Posizionamento prudente nel middlegame.
pub const QUEEN_TABLE: [i16; 64] = [
   -20,-10,-10, -5, -5,-10,-10,-20,
   -10,  0,  5,  5,  5,  5,  0,-10,
   -10,  5,  5,  5,  5,  5,  0,-10,
    -5,  0,  5,  5,  5,  5,  0, -5,
     0,  0,  5,  5,  5,  5,  0, -5,
   -10,  5,  5,  5,  5,  5,  0,-10,
   -10,  0,  5,  0,  0,  0,  0,-10,
   -20,-10,-10, -5, -5,-10,-10,-20,
];

// PST Re (Middlegame): Sicurezza dell'arrocco.
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

// PST Re (Endgame): Centralizzazione attiva.
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

// Bonus Dinamici e Penalità Forzate
pub const PASSED_PAWN_BONUS: [i16; 8] = [0, 2, 5, 10, 25, 50, 90, 0];
pub const PASSED_PAWN_ENDGAME_BONUS: [i16; 8] = [0, 10, 25, 50, 85, 140, 220, 0];
pub const DEVELOPMENT_PENALTY: i16 = -60; // Penalità severa per forzare l'uscita dei pezzi minori
pub const KNIGHT_OUTPOST_BONUS: i16 = 30;
pub const KING_PROXIMITY_TO_OWN_PASSED_PAWN: i16 = 12;
pub const QUEEN_MOBILITY_WEIGHT: i16 = 2;
pub const QUEEN_EXPOSED_PENALTY: i16 = -35;
pub const PROTECTED_PASSED_PAWN_BONUS: i16 = 20;