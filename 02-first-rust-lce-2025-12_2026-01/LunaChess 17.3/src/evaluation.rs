use crate::board::{Scacchiera, Colore, Bitboard, Pezzo};

pub const MG_VAL: [i32; 6] = [100, 320, 330, 500, 900, 0];
pub const EG_VAL: [i32; 6] = [120, 310, 340, 520, 950, 0];
pub const SEE_VAL: [i32; 6] = [100, 320, 330, 500, 900, 10000];

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

pub fn evaluate(board: &Scacchiera) -> i32 {
    let mut mg = [0i32; 2];
    let mut eg = [0i32; 2];
    let mut game_phase = 0;
    const PHASE_WEIGHTS: [i32; 6] = [0, 1, 1, 2, 4, 0];

    for colore in 0..2 {
        let us = if colore == 0 { Colore::Bianco } else { Colore::Nero };
        for p_idx in 0..6 {
            let mut bb = board.pezzi[p_idx] & board.colori[colore];
            while bb != 0 {
                let sq = bb.trailing_zeros() as usize;
                let rel_sq = if colore == 0 { sq ^ 56 } else { sq };
                
                mg[colore] += MG_VAL[p_idx] + get_pst(p_idx, rel_sq, true);
                eg[colore] += EG_VAL[p_idx] + get_pst(p_idx, rel_sq, false);
                game_phase += PHASE_WEIGHTS[p_idx];

                if p_idx == 5 { mg[colore] += evaluate_king_safety(board, sq, us); }
                bb &= bb - 1;
            }
        }
    }

    let mg_score = mg[0] - mg[1];
    let eg_score = eg[0] - eg[1];
    let phase = (game_phase.min(24) * 256) / 24;
    let mut score = ((mg_score * phase) + (eg_score * (256 - phase))) / 256;
    if board.turno == Colore::Nero { score = -score; }
    score
}

fn get_pst(p_idx: usize, sq: usize, mg: bool) -> i32 {
    match p_idx {
        0 => PAWN_PST[sq],
        1 => KNIGHT_PST[sq],
        5 => if mg { KING_MG_PST[sq] } else { 0 },
        _ => 0,
    }
}

fn evaluate_king_safety(board: &Scacchiera, king_sq: usize, us: Colore) -> i32 {
    let mut safety = 0;
    let col = king_sq % 8;
    let file_mask = 0x0101010101010101u64 << col;
    if (board.pezzi[0] & board.colori[us.indice()] & file_mask) == 0 {
        safety -= 50;
    }
    safety
}

pub fn see(board: &Scacchiera, sq: usize, target_val: i32, attacker_val: i32, mut side: Colore) -> i32 {
    let mut gain = [0i32; 64];
    let mut d = 0;
    let mut occ = board.occupazione();
    let mut attackers = get_all_attackers(board, sq, occ);
    gain[0] = target_val;
    while attackers != 0 && d < 63 {
        d += 1;
        let piece_bb = find_least_valuable_attacker(board, attackers, side);
        if piece_bb == 0 { d -= 1; break; }
        let p_idx = get_piece_type_from_bb(board, piece_bb);
        gain[d] = SEE_VAL[p_idx] - gain[d-1];
        if gain[d].max(-gain[d-1]) < 0 { break; }
        occ ^= piece_bb;
        attackers = get_all_attackers(board, sq, occ);
        side = side.opposto();
    }
    while d > 0 { gain[d-1] = -((-gain[d-1]).max(gain[d])); d -= 1; }
    gain[0]
}

fn get_all_attackers(board: &Scacchiera, sq: usize, occ: Bitboard) -> Bitboard {
    let mut attackers = 0;
    attackers |= crate::attacks::rook_attacks(sq, occ) & (board.pezzi[3] | board.pezzi[4]);
    attackers |= crate::attacks::bishop_attacks(sq, occ) & (board.pezzi[2] | board.pezzi[4]);
    attackers |= crate::attacks::knight_attacks(sq) & board.pezzi[1];
    attackers |= crate::attacks::king_attacks(sq) & board.pezzi[5];
    attackers |= crate::attacks::pawn_attacks(sq, Colore::Bianco) & board.pezzi[0] & board.colori[1];
    attackers |= crate::attacks::pawn_attacks(sq, Colore::Nero) & board.pezzi[0] & board.colori[0];
    attackers & (board.colori[0] | board.colori[1])
}

fn find_least_valuable_attacker(board: &Scacchiera, attackers: Bitboard, side: Colore) -> Bitboard {
    let side_attackers = attackers & board.colori[side.indice()];
    if side_attackers == 0 { return 0; }
    for p_idx in 0..6 {
        let subset = side_attackers & board.pezzi[p_idx];
        if subset != 0 { return 1u64 << subset.trailing_zeros(); }
    }
    0
}

fn get_piece_type_from_bb(board: &Scacchiera, bb: Bitboard) -> usize {
    for p_idx in 0..6 { if (board.pezzi[p_idx] & bb) != 0 { return p_idx; } }
    0
}