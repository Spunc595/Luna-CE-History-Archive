use crate::board::{Scacchiera, Colore, Pezzo};

// --- PESI MATERIALI ---
const MATERIAL_VAL: [i32; 6] = [100, 320, 330, 500, 900, 20000];

// --- PST (Piece-Square Tables) ---
// Semplificate per leggibilità, ma efficaci.
// NOTA: Per il Re, penalizziamo il centro e premiamo i lati G1/C1/G8/C8.

#[rustfmt::skip]
const PST_PAWN: [i32; 64] = [
    0,  0,  0,  0,  0,  0,  0,  0,
    50, 50, 50, 50, 50, 50, 50, 50,
    10, 10, 20, 30, 30, 20, 10, 10,
     5,  5, 10, 25, 25, 10,  5,  5,
     0,  0,  0, 20, 20,  0,  0,  0,
     5, -5,-10,  0,  0,-10, -5,  5,
     5, 10, 10,-20,-20, 10, 10,  5,
     0,  0,  0,  0,  0,  0,  0,  0
];

#[rustfmt::skip]
const PST_KNIGHT: [i32; 64] = [
    -50,-40,-30,-30,-30,-30,-40,-50,
    -40,-20,  0,  0,  0,  0,-20,-40,
    -30,  0, 10, 15, 15, 10,  0,-30,
    -30,  5, 15, 20, 20, 15,  5,-30,
    -30,  0, 15, 20, 20, 15,  0,-30,
    -30,  5, 10, 15, 15, 10,  5,-30,
    -40,-20,  0,  5,  5,  0,-20,-40,
    -50,-40,-30,-30,-30,-30,-40,-50
];

#[rustfmt::skip]
const PST_BISHOP: [i32; 64] = [
    -20,-10,-10,-10,-10,-10,-10,-20,
    -10,  0,  0,  0,  0,  0,  0,-10,
    -10,  0,  5, 10, 10,  5,  0,-10,
    -10,  5,  5, 10, 10,  5,  5,-10,
    -10,  0, 10, 10, 10, 10,  0,-10,
    -10, 10, 10, 10, 10, 10, 10,-10,
    -10,  5,  0,  0,  0,  0,  5,-10,
    -20,-10,-10,-10,-10,-10,-10,-20
];

#[rustfmt::skip]
const PST_ROOK: [i32; 64] = [
    0,  0,  0,  0,  0,  0,  0,  0,
    5, 10, 10, 10, 10, 10, 10,  5,
   -5,  0,  0,  0,  0,  0,  0, -5,
   -5,  0,  0,  0,  0,  0,  0, -5,
   -5,  0,  0,  0,  0,  0,  0, -5,
   -5,  0,  0,  0,  0,  0,  0, -5,
    5, 10, 10, 10, 10, 10, 10,  5,
    0,  0,  0,  0,  0,  0,  0,  0
];

#[rustfmt::skip]
const PST_QUEEN: [i32; 64] = [
   -20,-10,-10, -5, -5,-10,-10,-20,
   -10,  0,  0,  0,  0,  0,  0,-10,
   -10,  0,  5,  5,  5,  5,  0,-10,
    -5,  0,  5,  5,  5,  5,  0, -5,
     0,  0,  5,  5,  5,  5,  0, -5,
   -10,  5,  5,  5,  5,  5,  0,-10,
   -10,  0,  5,  0,  0,  0,  0,-10,
   -20,-10,-10, -5, -5,-10,-10,-20
];

#[rustfmt::skip]
const PST_KING_MG: [i32; 64] = [
   -30,-40,-40,-50,-50,-40,-40,-30,
   -30,-40,-40,-50,-50,-40,-40,-30,
   -30,-40,-40,-50,-50,-40,-40,-30,
   -30,-40,-40,-50,-50,-40,-40,-30,
   -20,-30,-30,-40,-40,-30,-30,-20,
   -10,-20,-20,-20,-20,-20,-20,-10,
    20, 20,  0,  0,  0,  0, 20, 20,
    20, 30, 10,  0,  0, 10, 30, 20
];

pub fn evaluate_classical(board: &Scacchiera) -> i32 {
    let mut score_mg = 0;

    // 1. Materiale e Posizione
    for p in 0..6 {
        let piece = match p {
            0 => Pezzo::Pedone, 1 => Pezzo::Cavallo, 2 => Pezzo::Alfiere,
            3 => Pezzo::Torre, 4 => Pezzo::Regina, _ => Pezzo::Re
        };
        let val = MATERIAL_VAL[p];
        let pst = get_pst(piece);

        let mut w_bb = board.pezzi[p] & board.colori[Colore::Bianco.indice()];
        while w_bb != 0 {
            let sq = w_bb.trailing_zeros() as usize;
            score_mg += val + pst[sq];
            w_bb &= w_bb - 1;
        }

        let mut b_bb = board.pezzi[p] & board.colori[Colore::Nero.indice()];
        while b_bb != 0 {
            let sq = b_bb.trailing_zeros() as usize;
            score_mg -= val + pst[sq ^ 56]; 
            b_bb &= b_bb - 1;
        }
    }

    // 2. Bonus Arrocco e Sicurezza Re (Anti-Kf1)
    score_mg += evaluate_king_safety(board, Colore::Bianco);
    score_mg -= evaluate_king_safety(board, Colore::Nero);

    if board.turno == Colore::Bianco { score_mg } else { -score_mg }
}

fn evaluate_king_safety(board: &Scacchiera, color: Colore) -> i32 {
    let king_bb = board.pezzi[Pezzo::Re.indice()] & board.colori[color.indice()];
    if king_bb == 0 { return 0; }
    let k_sq = king_bb.trailing_zeros() as usize;
    let mut score = 0;

    // --- CASTLING BONUS ---
    // Se il Re è in G1/C1 (o G8/C8 per il nero), presumiamo abbia arroccato o sia al sicuro.
    // Questo è un controllo statico posizionale.
    let is_castled_kingside = if color == Colore::Bianco { k_sq == 6 } else { k_sq == 62 };
    let is_castled_queenside = if color == Colore::Bianco { k_sq == 2 } else { k_sq == 58 };

    if is_castled_kingside || is_castled_queenside {
        score += 60; // Bonus significativo per aver arroccato
    } else {
        // Se non è arroccato, penalizziamo se ha perso i diritti o è esposto
        // Diritti arrocco: bit 0=WK, 1=WQ, 2=BK, 3=BQ
        let rights = board.diritti_arrocco;
        let can_castle_k = if color == Colore::Bianco { (rights & 1) != 0 } else { (rights & 4) != 0 };
        let can_castle_q = if color == Colore::Bianco { (rights & 2) != 0 } else { (rights & 8) != 0 };

        if !can_castle_k && !can_castle_q {
            // Penalità severa se è al centro e non può più arroccare (es. dopo Kf1)
            // Colonne centrali: D, E (file 3, 4)
            let file = k_sq % 8;
            if file >= 3 && file <= 4 {
                score -= 50; 
            }
        }
    }

    // --- PAWN SHIELD (Semplificato) ---
    // Se siamo in 1a o 2a traversa (o 7a/8a), controlliamo i pedoni davanti
    let rank = k_sq / 8;
    let safe_rank = if color == Colore::Bianco { rank < 2 } else { rank > 5 };
    
    if safe_rank {
        let pawns = board.pezzi[Pezzo::Pedone.indice()] & board.colori[color.indice()];
        let forward = if color == Colore::Bianco { 8 } else { -8 };
        
        // Controlla pedone direttamente davanti e sulle diagonali frontali
        let shield_sqs = [k_sq as i32 + forward, k_sq as i32 + forward - 1, k_sq as i32 + forward + 1];
        
        for &sq in &shield_sqs {
            if sq >= 0 && sq < 64 {
                if (pawns & (1u64 << sq)) != 0 {
                    score += 15; // Bonus per ogni pedone scudo
                } else if (board.occupazione() & (1u64 << sq)) == 0 {
                    // Penalità per colonna aperta davanti al re
                    score -= 10;
                }
            }
        }
    }

    score
}

fn get_pst(p: Pezzo) -> &'static [i32; 64] {
    match p {
        Pezzo::Pedone => &PST_PAWN, Pezzo::Cavallo => &PST_KNIGHT, Pezzo::Alfiere => &PST_BISHOP,
        Pezzo::Torre => &PST_ROOK, Pezzo::Regina => &PST_QUEEN, Pezzo::Re => &PST_KING_MG,
    }
}