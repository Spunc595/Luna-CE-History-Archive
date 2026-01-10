use crate::board::{Scacchiera, Colore, Pezzo, Bitboard};

// Valori dei pezzi per Mediogioco (MG) e Finale (EG)
pub const MG_VAL: [i32; 6] = [100, 320, 330, 500, 900, 0];
pub const EG_VAL: [i32; 6] = [120, 310, 340, 520, 950, 0];

// Valori statici per SEE (Static Exchange Evaluation)
pub const SEE_VAL: [i32; 6] = [100, 320, 330, 500, 900, 10000];

// Parametri di valutazione
const BISHOP_PAIR_MG: i32 = 25;
const BISHOP_PAIR_EG: i32 = 45;
const ISOLATED_PAWN_PENALTY: i32 = -15;
const DOUBLED_PAWN_PENALTY: i32 = -10;
const PASSED_PAWN_BONUS: [i32; 8] = [0, 5, 10, 25, 45, 75, 120, 0];

pub fn evaluate(board: &Scacchiera) -> i32 {
    let mut mg = [0i32; 2];
    let mut eg = [0i32; 2];
    let mut game_phase = 0;
    const PHASE_WEIGHTS: [i32; 6] = [0, 1, 1, 2, 4, 0];
    const MAX_PHASE: i32 = 24;

    for colore in 0..2 {
        let mut n_alfieri = 0;
        let c = if colore == 0 { Colore::Bianco } else { Colore::Nero };
        let nemico = c.opposto();

        for pezzo_tipo in 0..6 {
            let mut bb = board.pezzi[pezzo_tipo] & board.colori[colore];
            while bb != 0 {
                let sq = bb.trailing_zeros() as usize;
                let rel_sq = if colore == 0 { sq ^ 56 } else { sq };

                mg[colore] += MG_VAL[pezzo_tipo];
                eg[colore] += EG_VAL[pezzo_tipo];
                game_phase += PHASE_WEIGHTS[pezzo_tipo];

                match pezzo_tipo {
                    0 => { // Pedoni
                        let (p_mg, p_eg) = evaluate_pawn(board, sq, c, nemico);
                        mg[colore] += p_mg;
                        eg[colore] += p_eg;
                    },
                    2 => n_alfieri += 1, // Alfieri (per bonus coppia)
                    5 => { // Re
                        mg[colore] += evaluate_king_safety(board, sq, c);
                    },
                    _ => {}
                }
                bb &= bb - 1;
            }
        }
        if n_alfieri >= 2 {
            mg[colore] += BISHOP_PAIR_MG;
            eg[colore] += BISHOP_PAIR_EG;
        }
    }

    let mg_score = mg[0] - mg[1];
    let eg_score = eg[0] - eg[1];
    let phase = (game_phase.min(MAX_PHASE) * 256) / MAX_PHASE;
    let mut score = ((mg_score * phase) + (eg_score * (256 - phase))) / 256;

    if board.turno == Colore::Nero { score = -score; }
    score
}

fn evaluate_pawn(board: &Scacchiera, sq: usize, us: Colore, them: Colore) -> (i32, i32) {
    let mut mg = 0;
    let mut eg = 0;
    let colonna = sq % 8;
    let traversa_rel = if us == Colore::Bianco { sq / 8 } else { 7 - (sq / 8) };

    let i_miei_pedoni = board.pezzi[0] & board.colori[us.indice()];
    let i_suoi_pedoni = board.pezzi[0] & board.colori[them.indice()];

    // Pedoni Doppi
    let col_mask = 0x0101010101010101u64 << colonna;
    if (i_miei_pedoni & col_mask).count_ones() > 1 {
        mg += DOUBLED_PAWN_PENALTY; eg += DOUBLED_PAWN_PENALTY;
    }

    // Pedoni Isolati
    let mut adj_mask = 0u64;
    if colonna > 0 { adj_mask |= 0x0101010101010101u64 << (colonna - 1); }
    if colonna < 7 { adj_mask |= 0x0101010101010101u64 << (colonna + 1); }
    if (i_miei_pedoni & adj_mask) == 0 {
        mg += ISOLATED_PAWN_PENALTY; eg += ISOLATED_PAWN_PENALTY;
    }

    // Pedoni Passati
    if is_passed_pawn(sq, us, i_suoi_pedoni) {
        let bonus = PASSED_PAWN_BONUS[traversa_rel];
        mg += bonus / 2;
        eg += bonus;
    }

    (mg, eg)
}

fn is_passed_pawn(sq: usize, us: Colore, opp_pawns: Bitboard) -> bool {
    let col = sq % 8;
    let mut mask: Bitboard = 0;
    for c in (col as i32 - 1)..=(col as i32 + 1) {
        if c < 0 || c > 7 { continue; }
        let f_mask = 0x0101010101010101u64 << c;
        let forward_mask = if us == Colore::Bianco {
            0xFFFFFFFFFFFFFF00u64 << (8 * (sq / 8 + 1))
        } else {
            0x00FFFFFFFFFFFFFFu64 >> (8 * (7 - sq / 8 + 1))
        };
        mask |= f_mask & forward_mask;
    }
    (opp_pawns & mask) == 0
}

fn evaluate_king_safety(board: &Scacchiera, king_sq: usize, us: Colore) -> i32 {
    let mut safety = 0;
    let col = king_sq % 8;
    // Penalità Re al centro in MG
    if col >= 2 && col <= 5 { safety -= 40; }
    // Penalità perdita arrocco
    let diritti = if us == Colore::Bianco { board.diritti_arrocco & 3 } else { board.diritti_arrocco & 12 };
    if diritti == 0 && (king_sq > 7 && king_sq < 56) { safety -= 70; }
    safety
}

// --- SEE (Static Exchange Evaluation) CON PROTEZIONE CRASH ---
pub fn see(board: &Scacchiera, sq: usize, target_val: i32, attacker_val: i32, mut side: Colore) -> i32 {
    let mut gain = [0i32; 64]; // Dimensione aumentata
    let mut d = 0;
    let mut occ = board.occupazione();
    let mut attackers = get_all_attackers(board, sq, occ);

    gain[0] = target_val;
    
    // d < 63 impedisce l'index out of bounds segnalato nel log
    while attackers != 0 && d < 63 {
        d += 1;
        let piece_bb = find_least_valuable_attacker(board, attackers, side);
        if piece_bb == 0 { 
            d -= 1;
            break; 
        }
        
        let p_idx = get_piece_type_from_bb(board, piece_bb);
        gain[d] = SEE_VAL[p_idx] - gain[d-1];
        
        // Se il lato che muove non può migliorare la sua situazione, taglia
        if gain[d].max(-gain[d-1]) < 0 { break; }
        
        occ ^= piece_bb;
        attackers = get_all_attackers(board, sq, occ);
        side = side.opposto();
    }

    while d > 0 {
        gain[d-1] = -((-gain[d-1]).max(gain[d]));
        d -= 1;
    }
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