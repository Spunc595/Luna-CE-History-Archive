use crate::board::{Scacchiera, Colore, Bitboard, Pezzo};

// --- COSTANTI DI VALUTAZIONE ---

// Valori Materiali [P, N, B, R, Q, K]
// MG: Valori per Apertura/Mediogioco (Cavalli forti, Pedoni normali)
pub const MG_VAL: [i32; 6] = [82, 337, 365, 477, 1025, 0];
// EG: Valori per Finale (Torri dominanti, Pedoni preziosi)
pub const EG_VAL: [i32; 6] = [94, 281, 297, 512, 936, 0];

// Fase del Gioco (Per interpolazione)
// Totale materiale non-pedone sulla scacchiera: 4*N + 4*B + 4*R + 2*Q
const MAX_PHASE: i32 = 24; 
const PHASE_WEIGHTS: [i32; 6] = [0, 1, 1, 2, 4, 0];

// Bonus Posizionali
const BISHOP_PAIR_BONUS: i32 = 50;
const PASSED_PAWN_BONUS: [i32; 8] = [0, 10, 20, 35, 50, 80, 120, 0];
const ROOK_OPEN_FILE_BONUS: i32 = 40;
const ROOK_SEMI_OPEN_FILE_BONUS: i32 = 20;

// Penalità Strategiche
const BLOCKED_C_PAWN_PENALTY: i32 = 40;
const USELESS_BISHOP_PENALTY: i32 = 30;
const PAWN_STORM_PENALTY: i32 = 60;

// --- PST (Piece-Square Tables) ---

// PEDONI
const PAWN_MG: [i32; 64] = [
    0, 0, 0, 0, 0, 0, 0, 0,
    50, 50, 50, 50, 50, 50, 50, 50,
    10, 10, 20, 30, 30, 20, 10, 10,
    5,  5, 10, 25, 25, 10,  5,  5,
    0,  0,  0, 20, 20,  0,  0,  0,
    5, -5,-10,  0,  0,-10, -5,  5,
    5, 10, 10,-20,-20, 10, 10,  5,
    0, 0, 0, 0, 0, 0, 0, 0
];
const PAWN_EG: [i32; 64] = [
    0, 0, 0, 0, 0, 0, 0, 0,
    80, 80, 80, 80, 80, 80, 80, 80,
    50, 50, 50, 50, 50, 50, 50, 50,
    30, 30, 30, 30, 30, 30, 30, 30,
    20, 20, 20, 20, 20, 20, 20, 20,
    10, 10, 10, 10, 10, 10, 10, 10,
    10, 10, 10, 10, 10, 10, 10, 10,
    0, 0, 0, 0, 0, 0, 0, 0
];

// CAVALLI
const KNIGHT_MG: [i32; 64] = [
    -50,-40,-30,-30,-30,-30,-40,-50,
    -40,-20,  0,  0,  0,  0,-20,-40,
    -30,  0, 10, 15, 15, 10,  0,-30,
    -30,  5, 15, 20, 20, 15,  5,-30,
    -30,  0, 15, 20, 20, 15,  0,-30,
    -30,  5, 10, 15, 15, 10,  5,-30,
    -40,-20,  0,  5,  5,  0,-20,-40,
    -50,-40,-30,-30,-30,-30,-40,-50
];
const KNIGHT_EG: [i32; 64] = KNIGHT_MG; 

// ALFIERI
const BISHOP_MG: [i32; 64] = [
    -20,-10,-10,-10,-10,-10,-10,-20,
    -10,  0,  0,  0,  0,  0,  0,-10,
    -10,  0,  5, 10, 10,  5,  0,-10,
    -10,  5,  5, 10, 10,  5,  5,-10,
    -10,  0, 10, 10, 10, 10,  0,-10,
    -10, 10, 10, 10, 10, 10, 10,-10,
    -10,  5,  0,  0,  0,  0,  5,-10,
    -20,-10,-10,-10,-10,-10,-10,-20
];
const BISHOP_EG: [i32; 64] = BISHOP_MG;

// TORRI
const ROOK_MG: [i32; 64] = [
    0,  0,  0,  0,  0,  0,  0,  0,
    5, 10, 10, 10, 10, 10, 10,  5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    0,  0,  0,  5,  5,  0,  0,  0
];
const ROOK_EG: [i32; 64] = ROOK_MG;

// RE
const KING_MG: [i32; 64] = [
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -20,-30,-30,-40,-40,-30,-30,-20,
    -10,-20,-20,-20,-20,-20,-20,-10,
     20, 20,  0,  0,  0,  0, 20, 20,
     20, 30, 10,  0,  0, 10, 30, 20
];
const KING_EG: [i32; 64] = [
    -50,-40,-30,-20,-20,-30,-40,-50,
    -30,-20,-10,  0,  0,-10,-20,-30,
    -30,-10, 20, 30, 30, 20,-10,-30,
    -30,-10, 30, 40, 40, 30,-10,-30,
    -30,-10, 30, 40, 40, 30,-10,-30,
    -30,-10, 20, 30, 30, 20,-10,-30,
    -30,-30,  0,  0,  0,  0,-30,-30,
    -50,-30,-30,-30,-30,-30,-30,-50
];

const FILE_MASKS: [u64; 8] = [
    0x0101010101010101, 0x0202020202020202, 0x0404040404040404, 0x0808080808080808,
    0x1010101010101010, 0x2020202020202020, 0x4040404040404040, 0x8080808080808080
];

// Funzioni helper PST
pub fn get_pst_mg(p: usize, sq: usize) -> i32 {
    match p { 0=>PAWN_MG[sq], 1=>KNIGHT_MG[sq], 2=>BISHOP_MG[sq], 3=>ROOK_MG[sq], 5=>KING_MG[sq], _=>0 }
}
pub fn get_pst_eg(p: usize, sq: usize) -> i32 {
    match p { 0=>PAWN_EG[sq], 1=>KNIGHT_EG[sq], 2=>BISHOP_EG[sq], 3=>ROOK_EG[sq], 5=>KING_EG[sq], _=>0 }
}

// --- FIX PER L'ERRORE E0425 ---
// Questa funzione serve solo per compatibilità con board.rs che cerca ancora "get_pst".
// Ritorna il valore MG, sufficiente per l'hash/ordinamento interno di board.rs.
pub fn get_pst(p: usize, sq: usize, _mg: bool) -> i32 {
    get_pst_mg(p, sq)
}

fn passed_pawn_mask(sq: usize, side: Colore) -> u64 {
    let file = sq % 8;
    let rank = sq / 8;
    let mut mask = 0;
    let files_to_check = if file == 0 { 3 } else if file == 7 { 0xC0 } else { (0x7 << (file - 1)) as u8 };
    
    if side == Colore::Bianco {
        for r in (rank + 1)..8 { mask |= (files_to_check as u64) << (r * 8); }
    } else {
        for r in 0..rank { mask |= (files_to_check as u64) << (r * 8); }
    }
    mask
}

// --- EVALUATE PRINCIPALE (TAPERED) ---
pub fn evaluate(board: &Scacchiera) -> i32 {
    let mut mg_score = 0;
    let mut eg_score = 0;
    let mut phase = 0;

    let us = board.turno;
    let occ = board.occupazione(); 
    
    let mut w_bishops = 0;
    let mut b_bishops = 0;

    for c in 0..2 { 
        let multiplier = if c == 0 { 1 } else { -1 };
        let enemy = if c == 0 { 1 } else { 0 };
        let enemy_pawns = board.pezzi[0] & board.colori[enemy];

        for p in 0..6 {
            let mut pieces = board.pezzi[p] & board.colori[c];
            let count = pieces.count_ones() as i32;
            phase += count * PHASE_WEIGHTS[p];

            if p == 2 { if c == 0 { w_bishops = count; } else { b_bishops = count; } }

            while pieces != 0 {
                let sq = pieces.trailing_zeros() as usize;
                let rank = sq / 8;
                
                // Materiale
                mg_score += MG_VAL[p] * multiplier;
                eg_score += EG_VAL[p] * multiplier;
                
                // PST (Specchio per Nero)
                let pst_idx = if c == 0 { sq } else { sq ^ 56 };
                mg_score += get_pst_mg(p, pst_idx) * multiplier;
                eg_score += get_pst_eg(p, pst_idx) * multiplier;

                // Pedoni Passati
                if p == 0 {
                    let mask = passed_pawn_mask(sq, if c == 0 { Colore::Bianco } else { Colore::Nero });
                    if (mask & enemy_pawns) == 0 {
                        let rel_rank = if c == 0 { rank } else { 7 - rank };
                        let bonus = PASSED_PAWN_BONUS[rel_rank];
                        mg_score += bonus * multiplier;
                        eg_score += (bonus * 2) * multiplier; 
                    }
                }

                pieces &= pieces - 1;
            }
        }
    }

    phase = phase.min(MAX_PHASE); 
    let mg_weight = phase;
    let eg_weight = MAX_PHASE - phase;

    let mut strategic_mg = 0;
    if w_bishops >= 2 { strategic_mg += BISHOP_PAIR_BONUS; }
    if b_bishops >= 2 { strategic_mg -= BISHOP_PAIR_BONUS; }
    
    strategic_mg += evaluate_strategic_faults(board);
    strategic_mg += evaluate_king_safety(board); 
    strategic_mg += evaluate_rooks(board);
    strategic_mg += evaluate_mobility(board, occ);
    strategic_mg += evaluate_queen_activity(board);
    
    let pawn_struct = evaluate_pawn_structure(board);
    mg_score += strategic_mg + pawn_struct;
    eg_score += pawn_struct; 

    let final_score = (mg_score * mg_weight + eg_score * eg_weight) / MAX_PHASE;

    if us == Colore::Nero { -final_score } else { final_score }
}

fn evaluate_strategic_faults(board: &Scacchiera) -> i32 {
    let mut score = 0;
    let w_pawns = board.pezzi[0] & board.colori[0];
    let w_knights = board.pezzi[1] & board.colori[0];
    if (w_pawns & (1u64 << 10)) != 0 && (w_knights & (1u64 << 18)) != 0 && (w_pawns & (1u64 << 27)) != 0 {
        score -= BLOCKED_C_PAWN_PENALTY;
    }
    let b_pawns = board.pezzi[0] & board.colori[1];
    let b_knights = board.pezzi[1] & board.colori[1];
    if (b_pawns & (1u64 << 50)) != 0 && (b_knights & (1u64 << 42)) != 0 && (b_pawns & (1u64 << 35)) != 0 {
        score += BLOCKED_C_PAWN_PENALTY;
    }

    let w_bishops = board.pezzi[2] & board.colori[0];
    let b_knights_p = board.pezzi[1] & board.colori[1];
    if (w_bishops & (1u64 << 25)) != 0 && (b_knights_p & (1u64 << 42)) == 0 { score -= USELESS_BISHOP_PENALTY; }
    if (w_bishops & (1u64 << 30)) != 0 && (b_knights_p & (1u64 << 45)) == 0 { score -= USELESS_BISHOP_PENALTY; }
    
    let b_bishops = board.pezzi[2] & board.colori[1];
    let w_knights_p = board.pezzi[1] & board.colori[0];
    if (b_bishops & (1u64 << 33)) != 0 && (w_knights_p & (1u64 << 18)) == 0 { score += USELESS_BISHOP_PENALTY; }
    if (b_bishops & (1u64 << 38)) != 0 && (w_knights_p & (1u64 << 21)) == 0 { score += USELESS_BISHOP_PENALTY; }

    score
}

fn evaluate_mobility(board: &Scacchiera, occ: Bitboard) -> i32 {
    let mut score = 0;
    let mob_bonus_n = 4; let mob_bonus_b = 3; let mob_bonus_r = 2; let mob_bonus_q = 1;

    for c in 0..2 {
        let multiplier = if c == 0 { 1 } else { -1 };
        let my_pieces = board.colori[c];
        
        let mut knights = board.pezzi[1] & my_pieces;
        while knights != 0 {
            let from = knights.trailing_zeros() as usize;
            let moves = crate::attacks::knight_attacks(from) & !my_pieces;
            score += moves.count_ones() as i32 * mob_bonus_n * multiplier;
            knights &= knights - 1;
        }
        let mut bishops = board.pezzi[2] & my_pieces;
        while bishops != 0 {
            let moves = board.get_attacks_for_piece(Pezzo::Alfiere, bishops.trailing_zeros() as usize, occ) & !my_pieces;
            score += moves.count_ones() as i32 * mob_bonus_b * multiplier;
            bishops &= bishops - 1;
        }
        let mut rooks = board.pezzi[3] & my_pieces;
        while rooks != 0 {
            let moves = board.get_attacks_for_piece(Pezzo::Torre, rooks.trailing_zeros() as usize, occ) & !my_pieces;
            score += moves.count_ones() as i32 * mob_bonus_r * multiplier;
            rooks &= rooks - 1;
        }
        let mut queens = board.pezzi[4] & my_pieces;
        while queens != 0 {
            let moves = board.get_attacks_for_piece(Pezzo::Regina, queens.trailing_zeros() as usize, occ) & !my_pieces;
            score += moves.count_ones() as i32 * mob_bonus_q * multiplier;
            queens &= queens - 1;
        }
    }
    score
}

fn evaluate_queen_activity(board: &Scacchiera) -> i32 {
    let mut score = 0;
    let center = 0x0000001818000000;
    if (board.pezzi[4] & board.colori[0] & center) != 0 { score += 15; }
    if (board.pezzi[4] & board.colori[1] & center) != 0 { score -= 15; }
    score
}

fn evaluate_pawn_structure(board: &Scacchiera) -> i32 {
    let mut score = 0;
    let wp = board.pezzi[0] & board.colori[0];
    let bp = board.pezzi[0] & board.colori[1];
    
    for f in 0..8 {
        let mask = FILE_MASKS[f];
        let w = (wp & mask).count_ones();
        let b = (bp & mask).count_ones();
        if w > 1 { score -= (w as i32 - 1) * 15; }
        if b > 1 { score += (b as i32 - 1) * 15; }
        
        let adj = (if f>0 {FILE_MASKS[f-1]} else {0}) | (if f<7 {FILE_MASKS[f+1]} else {0});
        if w > 0 && (wp & adj) == 0 { score -= 20; }
        if b > 0 && (bp & adj) == 0 { score += 20; }
    }
    score
}

fn evaluate_rooks(board: &Scacchiera) -> i32 {
    let mut score = 0;
    let pawns = board.pezzi[0];
    let wp = pawns & board.colori[0];
    let bp = pawns & board.colori[1];

    let mut wr = board.pezzi[3] & board.colori[0];
    while wr != 0 {
        let sq = wr.trailing_zeros() as usize;
        let mask = FILE_MASKS[sq % 8];
        if (pawns & mask) == 0 { score += ROOK_OPEN_FILE_BONUS; }
        else if (wp & mask) == 0 { score += ROOK_SEMI_OPEN_FILE_BONUS; }
        wr &= wr - 1;
    }
    let mut br = board.pezzi[3] & board.colori[1];
    while br != 0 {
        let sq = br.trailing_zeros() as usize;
        let mask = FILE_MASKS[sq % 8];
        if (pawns & mask) == 0 { score -= ROOK_OPEN_FILE_BONUS; }
        else if (bp & mask) == 0 { score -= ROOK_SEMI_OPEN_FILE_BONUS; }
        br &= br - 1;
    }
    score
}

fn evaluate_king_safety(board: &Scacchiera) -> i32 {
    let mut score = 0;
    for c in 0..2 {
        let k_bb = board.pezzi[5] & board.colori[c];
        if k_bb == 0 { continue; }
        let k_sq = k_bb.trailing_zeros() as usize;
        let file = k_sq % 8;
        
        let my_pawns = board.pezzi[0] & board.colori[c];
        let opp_pawns = board.pezzi[0] & board.colori[1-c];
        
        let mut penalty = 0;
        let start_f = if file > 0 { file - 1 } else { 0 };
        let end_f = if file < 7 { file + 1 } else { 7 };

        for f in start_f..=end_f {
            let mask = FILE_MASKS[f];
            if (my_pawns & mask) == 0 {
                penalty += 25; 
                if (opp_pawns & mask) == 0 { penalty += 30; } 
            }
            let storm_rank = if c == 0 { 0x0000000000FF0000 } else { 0x0000FF0000000000 };
            if (opp_pawns & mask & storm_rank) != 0 {
                penalty += PAWN_STORM_PENALTY;
            }
        }
        
        if c == 0 { score -= penalty; } else { score += penalty; }
    }
    score
}

pub fn see(_board: &Scacchiera, _target: usize, target_val: i32, attacker_val: i32, side: Colore) -> i32 {
    if side == Colore::Bianco { attacker_val - target_val } else { target_val - attacker_val }
}