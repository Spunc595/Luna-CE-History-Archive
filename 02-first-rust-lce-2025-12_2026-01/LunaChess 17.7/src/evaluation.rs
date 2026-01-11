use crate::board::{Scacchiera, Colore, Bitboard, Pezzo};

// --- VALORI DEI PEZZI ---
pub const MG_VAL: [i32; 6] = [100, 320, 335, 500, 900, 0];
pub const EG_VAL: [i32; 6] = [120, 310, 345, 520, 950, 0];
pub const SEE_VAL: [i32; 6] = [100, 320, 330, 500, 900, 20000]; 

// Bonus Posizionali
const BISHOP_PAIR_BONUS: i32 = 50;
const ROOK_OPEN_FILE_BONUS: i32 = 30;
const ROOK_SEMI_OPEN_FILE_BONUS: i32 = 15;

// Bonus Mobilità (Pesati per tipo di pezzo)
// [P, N, B, R, Q, K] - Pedone e Re ignorati per mobilità qui
const MOBILITY_BONUS: [i32; 6] = [0, 4, 3, 2, 1, 0]; 

// Piece-Square Tables (PST)
const PAWN_PST: [i32; 64] = [
    0, 0, 0, 0, 0, 0, 0, 0,
    5, 10, 10, -20, -20, 10, 10, 5,
    5, -5, -10, 5, 5, -10, -5, 5, 
    0, 0, 10, 25, 25, 10, 0, 0,      
    5, 5, 10, 25, 25, 10, 5, 5,
    10, 10, 20, 30, 30, 20, 10, 10,
    50, 50, 50, 50, 50, 50, 50, 50,
    0, 0, 0, 0, 0, 0, 0, 0
];

const KNIGHT_PST: [i32; 64] = [
    -50,-40,-30,-30,-30,-30,-40,-50,
    -40,-20, 0, 5, 5, 0,-20,-40,
    -30, 5, 15, 20, 20, 15, 5,-30,
    -30, 0, 20, 25, 25, 20, 0,-30,
    -30, 5, 20, 25, 25, 20, 5,-30,
    -30, 0, 15, 20, 20, 15, 0,-30,
    -40,-20, 0, 0, 0, 0,-20,-40,
    -50,-40,-30,-30,-30,-30,-40,-50
];

const BISHOP_PST: [i32; 64] = [
    -20,-10,-10,-10,-10,-10,-10,-20,
    -10,  0,  0,  0,  0,  0,  0,-10,
    -10,  0,  5, 10, 10,  5,  0,-10,
    -10,  5,  5, 10, 10,  5,  5,-10,
    -10,  0, 10, 10, 10, 10,  0,-10,
    -10, 10, 10, 10, 10, 10, 10,-10,
    -10,  5,  0,  0,  0,  0,  5,-10,
    -20,-10,-10,-10,-10,-10,-10,-20
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

const FILE_MASKS: [u64; 8] = [
    0x0101010101010101, 0x0202020202020202, 0x0404040404040404, 0x0808080808080808,
    0x1010101010101010, 0x2020202020202020, 0x4040404040404040, 0x8080808080808080
];

pub fn get_pst(p: usize, sq: usize, _mg: bool) -> i32 {
    match p {
        0 => PAWN_PST[sq],
        1 => KNIGHT_PST[sq],
        2 => BISHOP_PST[sq],
        5 => KING_MG_PST[sq],
        _ => 0,
    }
}

pub fn evaluate(board: &Scacchiera) -> i32 {
    let mut score = board.pst_val;
    let us = board.turno;
    let occ = board.occupazione(); // Serve per la mobilità
    
    let mut w_bishops = 0;
    let mut b_bishops = 0;

    for c in 0..2 { 
        let multiplier = if c == 0 { 1 } else { -1 };
        
        let bishops = board.pezzi[2] & board.colori[c];
        if c == 0 { w_bishops = bishops.count_ones(); } else { b_bishops = bishops.count_ones(); }
        
        for p in 0..5 {
            let count = (board.pezzi[p] & board.colori[c]).count_ones() as i32;
            score += count * MG_VAL[p] * multiplier;
        }
    }

    if w_bishops >= 2 { score += BISHOP_PAIR_BONUS; }
    if b_bishops >= 2 { score -= BISHOP_PAIR_BONUS; }

    score += evaluate_pawn_structure(board);
    score += evaluate_rooks(board);
    score += evaluate_king_safety(board);
    score += evaluate_queen_activity(board);
    score += evaluate_mobility(board, occ); // NUOVA FUNZIONE

    if us == Colore::Nero { -score } else { score }
}

// --- NUOVA: MOBILITY (Rende Luna aggressiva) ---
fn evaluate_mobility(board: &Scacchiera, occ: Bitboard) -> i32 {
    let mut score = 0;
    
    for c in 0..2 {
        let multiplier = if c == 0 { 1 } else { -1 };
        let my_pieces = board.colori[c];
        
        // 1. Cavalli
        let mut knights = board.pezzi[1] & my_pieces;
        while knights != 0 {
            let from = knights.trailing_zeros() as usize;
            // Usa le lookup table degli attacchi che hai già in board o attacks
            let moves = crate::attacks::knight_attacks(from); 
            // Conta case non occupate da amici
            let safe_moves = moves & !my_pieces;
            score += safe_moves.count_ones() as i32 * MOBILITY_BONUS[1] * multiplier;
            knights &= knights - 1;
        }

        // 2. Alfieri (Scorrevole)
        let mut bishops = board.pezzi[2] & my_pieces;
        while bishops != 0 {
            let from = bishops.trailing_zeros() as usize;
            // Qui assumo tu abbia una funzione magica o ray_attacks. 
            // Se non ce l'hai esposta, usa board.get_attacks_for_piece
            let moves = board.get_attacks_for_piece(Pezzo::Alfiere, from, occ);
            let safe_moves = moves & !my_pieces;
            score += safe_moves.count_ones() as i32 * MOBILITY_BONUS[2] * multiplier;
            bishops &= bishops - 1;
        }

        // 3. Torri (Scorrevole)
        let mut rooks = board.pezzi[3] & my_pieces;
        while rooks != 0 {
            let from = rooks.trailing_zeros() as usize;
            let moves = board.get_attacks_for_piece(Pezzo::Torre, from, occ);
            let safe_moves = moves & !my_pieces;
            score += safe_moves.count_ones() as i32 * MOBILITY_BONUS[3] * multiplier;
            rooks &= rooks - 1;
        }

        // 4. Regina (Scorrevole)
        let mut queens = board.pezzi[4] & my_pieces;
        while queens != 0 {
            let from = queens.trailing_zeros() as usize;
            let moves = board.get_attacks_for_piece(Pezzo::Regina, from, occ);
            let safe_moves = moves & !my_pieces;
            score += safe_moves.count_ones() as i32 * MOBILITY_BONUS[4] * multiplier;
            queens &= queens - 1;
        }
    }
    score
}

fn evaluate_queen_activity(board: &Scacchiera) -> i32 {
    let mut score = 0;
    let center_mask = 0x0000001818000000;
    let w_queen = board.pezzi[4] & board.colori[0];
    if (w_queen & center_mask) != 0 { score += 15; }
    let b_queen = board.pezzi[4] & board.colori[1];
    if (b_queen & center_mask) != 0 { score -= 15; }
    score
}

fn evaluate_pawn_structure(board: &Scacchiera) -> i32 {
    let mut score = 0;
    let white_pawns = board.pezzi[0] & board.colori[0];
    let black_pawns = board.pezzi[0] & board.colori[1];
    let doubled_penalty = 15;
    let isolated_penalty = 20;

    for f in 0..8 {
        let file_mask = FILE_MASKS[f];
        let w_count = (white_pawns & file_mask).count_ones();
        let b_count = (black_pawns & file_mask).count_ones();
        if w_count > 1 { score -= doubled_penalty * (w_count as i32 - 1); }
        if b_count > 1 { score += doubled_penalty * (b_count as i32 - 1); }
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
    let all_pawns = board.pezzi[0];
    for c in 0..2 {
        let my_rooks = board.pezzi[3] & board.colori[c];
        let my_pawns = board.pezzi[0] & board.colori[c];
        let multiplier = if c == 0 { 1 } else { -1 };
        let mut r = my_rooks;
        while r != 0 {
            let sq = r.trailing_zeros() as usize;
            let file_mask = FILE_MASKS[sq % 8];
            if (all_pawns & file_mask) == 0 { score += ROOK_OPEN_FILE_BONUS * multiplier; } 
            else if (my_pawns & file_mask) == 0 { score += ROOK_SEMI_OPEN_FILE_BONUS * multiplier; }
            r &= r - 1;
        }
    }
    score
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
                let mut shield_penalty = 25;
                let enemy_pawn_on_file = enemy_pawns & file_mask;
                if enemy_pawn_on_file == 0 { shield_penalty += 30; }
                penalty += shield_penalty;
            } else {
                let pawn_sq = if c == 0 { my_pawn_on_file.trailing_zeros() as usize } else { 63 - my_pawn_on_file.leading_zeros() as usize };
                let pawn_rank = pawn_sq / 8;
                let dist = if c == 0 { if pawn_rank > king_rank { pawn_rank - king_rank } else { 0 } } else { if king_rank > pawn_rank { king_rank - pawn_rank } else { 0 } };
                if dist > 1 { penalty += dist as i32 * 10; }
            }
        }
        if (king_file < 2 || king_file > 5) && penalty > 40 { penalty += 20; }
        if c == 0 { score -= penalty; } else { score += penalty; }
    }
    score
}

pub fn see(_board: &Scacchiera, _target: usize, target_val: i32, attacker_val: i32, side: Colore) -> i32 {
    if side == Colore::Bianco { attacker_val - target_val } else { target_val - attacker_val }
}