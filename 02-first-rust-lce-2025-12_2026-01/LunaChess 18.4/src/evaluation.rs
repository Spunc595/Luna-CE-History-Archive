use crate::board::{Scacchiera, Colore, Bitboard};

// Valori dei pezzi (Middlegame e Endgame)
pub const MG_VAL: [i32; 6] = [82, 337, 365, 477, 1025, 0];
pub const EG_VAL: [i32; 6] = [94, 281, 297, 512, 936, 0];

// Punteggio Tablebase: deve essere alto ma inferiore al matto
pub const TB_WIN_SCORE: i32 = 30000;

const MAX_PHASE: i32 = 24;
const PHASE_WEIGHTS: [i32; 6] = [0, 1, 1, 2, 4, 0];

// --- TABELLE POSIZIONALI (PST) ---
const PAWN_PST: [i32; 64] = [
    0,  0,  0,  0,  0,  0,  0,  0,
    50, 50, 50, 50, 50, 50, 50, 50,
    10, 10, 20, 30, 30, 20, 10, 10,
     5,  5, 10, 25, 25, 10,  5,  5,
     0,  0,  0, 20, 20,  0,  0,  0,
     5, -5,-10,  0,  0,-10, -5,  5,
     5, 10, 10,-20,-20, 10, 10,  5,
     0,  0,  0,  0,  0,  0,  0,  0
];

const KNIGHT_PST: [i32; 64] = [
    -50,-40,-30,-30,-30,-30,-40,-50,
    -40,-20,  0,  0,  0,  0,-20,-40,
    -30,  0, 10, 15, 15, 10,  0,-30,
    -30,  5, 15, 20, 20, 15,  5,-30,
    -30,  0, 15, 20, 20, 15,  0,-30,
    -30,  5, 10, 15, 15, 10,  5,-30,
    -40,-20,  0,  5,  5,  0,-20,-40,
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

const ROOK_PST: [i32; 64] = [
      0,  0,  0,  0,  0,  0,  0,  0,
      5, 10, 10, 10, 10, 10, 10,  5,
     -5,  0,  0,  0,  0,  0,  0, -5,
     -5,  0,  0,  0,  0,  0,  0, -5,
     -5,  0,  0,  0,  0,  0,  0, -5,
     -5,  0,  0,  0,  0,  0,  0, -5,
     -5,  0,  0,  0,  0,  0,  0, -5,
      0,  0,  0,  5,  5,  0,  0,  0
];

const KING_MG_PST: [i32; 64] = [
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -20,-30,-30,-40,-40,-30,-30,-20,
    -10,-20,-20,-20,-20,-20,-20,-10,
     20, 20,  0,  0,  0,  0, 20, 20,
     20, 30, 10,  0,  0, 10, 30, 20
];

const KING_EG_PST: [i32; 64] = [
    -50,-40,-30,-20,-20,-30,-40,-50,
    -30,-20,-10,  0,  0,-10,-20,-30,
    -30,-10, 20, 30, 30, 20,-10,-30,
    -30,-10, 30, 40, 40, 30,-10,-30,
    -30,-10, 30, 40, 40, 30,-10,-30,
    -30,-10, 20, 30, 30, 20,-10,-30,
    -30,-30,  0,  0,  0,  0,-30,-30,
    -50,-30,-30,-30,-30,-30,-30,-50
];

pub fn get_pst(p: usize, sq: usize, mg: bool) -> i32 {
    match p {
        0 => PAWN_PST[sq],
        1 => KNIGHT_PST[sq],
        2 => BISHOP_PST[sq],
        3 => ROOK_PST[sq],
        4 => BISHOP_PST[sq] + ROOK_PST[sq],
        5 => if mg { KING_MG_PST[sq] } else { KING_EG_PST[sq] },
        _ => 0,
    }
}

pub fn evaluate(board: &Scacchiera) -> i32 {
    let mut mg = 0;
    let mut eg = 0;
    let mut phase = 0;

    let occ = board.occupazione();
    let all_pawns = board.pezzi[0];

    for c in 0..2 {
        let is_white = c == 0;
        let mult = if is_white { 1 } else { -1 };
        let us_color = board.colori[c];
        let enemy_pawns = board.pezzi[0] & board.colori[1 - c];
        
        for p in 0..6 {
            let mut pieces = board.pezzi[p] & us_color;
            while pieces != 0 {
                let sq = pieces.trailing_zeros() as usize;
                phase += PHASE_WEIGHTS[p];
                let pst_idx = if is_white { sq } else { sq ^ 56 };
                
                mg += (MG_VAL[p] + get_pst(p, pst_idx, true)) * mult;
                eg += (EG_VAL[p] + get_pst(p, pst_idx, false)) * mult;
                
                // MOBILITÀ
                if p > 0 && p < 5 {
                    let att = board.get_attacks_for_piece(
                        match p { 
                            1=>crate::board::Pezzo::Cavallo, 
                            2=>crate::board::Pezzo::Alfiere, 
                            3=>crate::board::Pezzo::Torre, 
                            4=>crate::board::Pezzo::Regina, 
                            _=>crate::board::Pezzo::Re 
                        },
                        sq, occ
                    );
                    mg += (att.count_ones() as i32 * 3) * mult;
                }

                // VALUTAZIONE TORRI SU COLONNE APERTE
                if p == 3 {
                    let file = sq % 8;
                    let file_mask = 0x0101010101010101u64 << file;
                    if (all_pawns & file_mask) == 0 {
                        mg += 25 * mult; // Colonna aperta (bonus aumentato)
                    } else if (board.pezzi[0] & us_color & file_mask) == 0 {
                        mg += 12 * mult; // Colonna semi-aperta
                    }
                }

                // PEDONI PASSATI (Cruciali per vincere i finali come contro Rybka)
                if p == 0 {
                    if is_passed_pawn(sq, is_white, enemy_pawns) {
                        let rank = if is_white { sq / 8 } else { 7 - (sq / 8) };
                        // Bonus quadratico: un pedone in 7a traversa vale molto più di uno in 4a
                        let bonus = (rank as i32 * rank as i32) * 18;
                        eg += bonus * mult;
                    }
                }
                
                pieces &= pieces - 1;
            }
        }
    }

    let phase_clamped = phase.min(MAX_PHASE);
    mg += evaluate_king_safety(board);
    
    // Tapered Eval: interpola tra Middlegame ed Endgame
    let score = (mg * phase_clamped + eg * (MAX_PHASE - phase_clamped)) / MAX_PHASE;
    
    if board.turno == Colore::Nero { -score } else { score }
}

fn is_passed_pawn(sq: usize, is_white: bool, enemy_pawns: Bitboard) -> bool {
    let file = sq % 8;
    let mut mask: u64 = 0;
    
    // Controlla la colonna del pedone e le due adiacenti
    for f in (file.saturating_sub(1))..=(file + 1).min(7) {
        let file_mask = 0x0101010101010101u64 << f;
        let rank_mask = if is_white {
            !((1u64 << ((sq / 8 + 1) * 8)) - 1) // Tutte le traverse sopra il pedone
        } else {
            (1u64 << (sq / 8 * 8)) - 1 // Tutte le traverse sotto il pedone
        };
        mask |= file_mask & rank_mask;
    }
    (mask & enemy_pawns) == 0
}

fn evaluate_king_safety(board: &Scacchiera) -> i32 {
    let mut safety = 0;
    for c in 0..2 {
        let is_white = c == 0;
        let mult = if is_white { 1 } else { -1 };
        let k_bb = board.pezzi[5] & board.colori[c];
        if k_bb == 0 { continue; }
        let k_sq = k_bb.trailing_zeros() as usize;
        
        // Calcolo dinamico dello scudo pedonale
        // Definiamo un'area di 3 case davanti al re
        let shield_rank = if is_white { (k_sq / 8) + 1 } else { (k_sq / 8).saturating_sub(1) };
        if shield_rank < 8 {
            let mut shield_mask = 0u64;
            let file = k_sq % 8;
            for f in (file.saturating_sub(1))..=(file + 1).min(7) {
                shield_mask |= 1u64 << (shield_rank * 8 + f);
            }
            
            let count = (shield_mask & board.pezzi[0] & board.colori[c]).count_ones();
            safety += (count as i32 * 35) * mult;
        }
    }
    safety
}