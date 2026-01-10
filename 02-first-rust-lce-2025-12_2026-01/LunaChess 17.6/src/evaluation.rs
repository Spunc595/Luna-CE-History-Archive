use crate::board::{Scacchiera, Colore, Bitboard};

// Valori Base
pub const MG_VAL: [i32; 6] = [100, 320, 330, 500, 900, 0];
pub const EG_VAL: [i32; 6] = [120, 310, 340, 520, 950, 0];
pub const SEE_VAL: [i32; 6] = [100, 320, 330, 500, 900, 20000]; 

// Piece-Square Tables (PST) ottimizzati per solidità
const PAWN_PST: [i32; 64] = [
    0, 0, 0, 0, 0, 0, 0, 0,
    5, 10, 10, -20, -20, 10, 10, 5,
    5, -5, -10, 0, 0, -10, -5, 5, 
    0, 0, 0, 20, 20, 0, 0, 0,      
    5, 5, 10, 25, 25, 10, 5, 5,
    10, 10, 20, 30, 30, 20, 10, 10,
    50, 50, 50, 50, 50, 50, 50, 50,
    0, 0, 0, 0, 0, 0, 0, 0
];

const KNIGHT_PST: [i32; 64] = [
    -50,-40,-30,-30,-30,-30,-40,-50,
    -40,-20, 0, 5, 5, 0,-20,-40,
    -30, 5, 10, 15, 15, 10, 5,-30,
    -30, 0, 15, 20, 20, 15, 0,-30,
    -30, 5, 15, 20, 20, 15, 5,-30,
    -30, 0, 10, 15, 15, 10, 0,-30,
    -40,-20, 0, 0, 0, 0,-20,-40,
    -50,-40,-30,-30,-30,-30,-40,-50
];

const KING_MG_PST: [i32; 64] = [
    20, 30, 10, 0, 0, 10, 30, 20,
    20, 20, 0, 0, 0, 0, 20, 20,
    -10,-20,-20,-20,-20,-20,-20,-10,
    -20,-30,-30,-40,-40,-30,-30,-20,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30
];

// Maschere
const FILE_MASKS: [u64; 8] = [
    0x0101010101010101, 0x0202020202020202, 0x0404040404040404, 0x0808080808080808,
    0x1010101010101010, 0x2020202020202020, 0x4040404040404040, 0x8080808080808080
];

pub fn get_pst(p: usize, sq: usize, _mg: bool) -> i32 {
    match p {
        0 => PAWN_PST[sq],
        1 => KNIGHT_PST[sq],
        5 => KING_MG_PST[sq],
        _ => 0,
    }
}

pub fn evaluate(board: &Scacchiera) -> i32 {
    let mut score = board.pst_val;
    
    // 1. Materiale base
    for c in 0..2 {
        for p in 0..5 {
            let count = (board.pezzi[p] & board.colori[c]).count_ones() as i32;
            if c == 0 { score += count * MG_VAL[p]; } else { score -= count * MG_VAL[p]; }
        }
    }

    // 2. Elementi Strategici
    score += evaluate_queen_development(board);
    score += evaluate_king_safety(board);
    score += evaluate_pawn_structure(board);
    score += evaluate_rooks(board);

    if board.turno == Colore::Nero { score = -score; }
    score
}

// --- FUNZIONI DI VALUTAZIONE ---

fn evaluate_pawn_structure(board: &Scacchiera) -> i32 {
    let mut score = 0;
    let white_pawns = board.pezzi[0] & board.colori[0];
    let black_pawns = board.pezzi[0] & board.colori[1];

    // Penalità
    let doubled_penalty = 15;
    let isolated_penalty = 20;

    for f in 0..8 {
        let file_mask = FILE_MASKS[f];

        // 1. Pedoni Doppiati (più di 1 pedone sulla stessa colonna)
        let w_count = (white_pawns & file_mask).count_ones();
        let b_count = (black_pawns & file_mask).count_ones();

        if w_count > 1 { score -= doubled_penalty * (w_count as i32 - 1); }
        if b_count > 1 { score += doubled_penalty * (b_count as i32 - 1); }

        // 2. Pedoni Isolati (nessun pedone amico sulle colonne adiacenti)
        let left_mask = if f > 0 { FILE_MASKS[f-1] } else { 0 };
        let right_mask = if f < 7 { FILE_MASKS[f+1] } else { 0 };
        let adjacent_mask = left_mask | right_mask;

        if w_count > 0 && (white_pawns & adjacent_mask) == 0 { score -= isolated_penalty; }
        if b_count > 0 && (black_pawns & adjacent_mask) == 0 { score += isolated_penalty; }
    }
    score
}

fn evaluate_rooks(board: &Scacchiera) -> i32 {
    let mut score = 0;
    let open_file_bonus = 25;
    let semi_open_file_bonus = 10;
    let pawns = board.pezzi[0]; // Tutti i pedoni

    // Torri Bianche
    let mut w_rooks = board.pezzi[3] & board.colori[0];
    while w_rooks != 0 {
        let sq = w_rooks.trailing_zeros() as usize;
        let file_mask = FILE_MASKS[sq % 8];
        if (pawns & file_mask) == 0 {
            score += open_file_bonus; // Colonna completamente aperta
        } else if (board.pezzi[0] & board.colori[0] & file_mask) == 0 {
            score += semi_open_file_bonus; // Solo pedoni nemici (buono per attacco)
        }
        w_rooks &= w_rooks - 1;
    }

    // Torri Nere
    let mut b_rooks = board.pezzi[3] & board.colori[1];
    while b_rooks != 0 {
        let sq = b_rooks.trailing_zeros() as usize;
        let file_mask = FILE_MASKS[sq % 8];
        if (pawns & file_mask) == 0 {
            score -= open_file_bonus;
        } else if (board.pezzi[0] & board.colori[1] & file_mask) == 0 {
            score -= semi_open_file_bonus;
        }
        b_rooks &= b_rooks - 1;
    }
    score
}

fn evaluate_queen_development(board: &Scacchiera) -> i32 {
    let mut penalty = 0;
    let minor_pieces = (board.pezzi[1] | board.pezzi[2]).count_ones();
    
    if minor_pieces > 4 {
        if (board.pezzi[4] & board.colori[0]) != 0 {
            if (board.pezzi[4] & board.colori[0] & (1 << 3)) == 0 { penalty -= 30; }
        }
        if (board.pezzi[4] & board.colori[1]) != 0 {
            if (board.pezzi[4] & board.colori[1] & (1 << 59)) == 0 { penalty += 30; }
        }
    }
    penalty
}

fn evaluate_king_safety(board: &Scacchiera) -> i32 {
    let mut score = 0;
    
    for c in 0..2 { 
        let king_bit = board.pezzi[5] & board.colori[c];
        if king_bit == 0 { continue; }
        
        let king_sq = king_bit.trailing_zeros() as usize;
        let king_file = king_sq % 8;
        let king_rank = king_sq / 8;

        let my_pawns = board.pezzi[0] & board.colori[c];
        let enemy_pawns = board.pezzi[0] & board.colori[1-c];

        let mut penalty = 0;

        let start_f = if king_file > 0 { king_file - 1 } else { 0 };
        let end_f = if king_file < 7 { king_file + 1 } else { 7 };

        for f in start_f..=end_f {
            let file_mask = FILE_MASKS[f];
            let my_pawn_on_file = my_pawns & file_mask;
            
            if my_pawn_on_file == 0 {
                penalty += 25; 
            } else {
                let pawn_sq = if c == 0 {
                    my_pawn_on_file.trailing_zeros() as usize
                } else {
                    63 - my_pawn_on_file.leading_zeros() as usize
                };
                let pawn_rank = pawn_sq / 8;
                let dist = if c == 0 {
                    if pawn_rank > king_rank { pawn_rank - king_rank } else { 0 }
                } else {
                    if king_rank > pawn_rank { king_rank - pawn_rank } else { 0 }
                };
                if dist > 1 { penalty += dist as i32 * 10; }
            }
            if (enemy_pawns & file_mask) == 0 { penalty += 10; }
        }

        if (king_file == 3 || king_file == 4) && (board.pezzi[4].count_ones() > 0) {
            for f in 6..=7 {
                let file_mask = FILE_MASKS[f];
                let my_p = my_pawns & file_mask;
                if my_p != 0 {
                    let p_sq = if c == 0 { my_p.trailing_zeros() as usize } else { 63 - my_p.leading_zeros() as usize };
                    let p_rank = p_sq / 8;
                    if c == 0 && p_rank >= 3 { penalty += 35; }
                    if c == 1 && p_rank <= 4 { penalty += 35; }
                } else {
                    penalty += 20; 
                }
            }
        }

        if c == 0 { score -= penalty; } else { score += penalty; }
    }
    score
}

pub fn see(board: &Scacchiera, target: usize, target_val: i32, attacker_val: i32, side: Colore) -> i32 {
    if side == Colore::Bianco { attacker_val - target_val } else { target_val - attacker_val }
}