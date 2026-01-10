use crate::board::{Colore, Pezzo, Scacchiera, Bitboard};

// Struttura della Rete Neurale (HalfKP: 768 input -> 256 hidden -> 1 output)
// 
#[derive(Clone)]
pub struct Network {
    pub feature_weights: Vec<i16>,
    pub feature_bias: Vec<i16>,
}

impl Network {
    // Carica la rete o ne crea una vuota se il file non esiste/è disabilitato
    pub fn carica(_path: &str) -> Self {
        // In futuro qui caricheremo il file binario .nnue
        // Per ora restituisce una rete inizializzata a zero per evitare crash
        Network {
            feature_weights: vec![0; 768 * 256],
            feature_bias: vec![0; 256],
        }
    }
}

// Calcola l'indice della feature per la rete (HalfKP)
pub fn get_feature_index(pezzo: Pezzo, colore: Colore, sq: usize) -> usize {
    let piece_offset = pezzo.indice();
    let color_offset = if colore == Colore::Bianco { 0 } else { 6 };
    // 64 caselle * 12 tipi di pezzo (6 bianchi + 6 neri)
    (color_offset + piece_offset) * 64 + sq
}

// --- VALUTAZIONE LIGHT DI BACKUP ---
// Usata se la valutazione principale (evaluation.rs) o la rete non sono disponibili.

pub fn evaluate_nnue(board: &Scacchiera, _net: &Network) -> i32 {
    let mut score = 0;
    score += evaluate_side(board, Colore::Bianco);
    score -= evaluate_side(board, Colore::Nero);
    if board.turno == Colore::Nero { -score } else { score }
}

fn evaluate_side(board: &Scacchiera, c: Colore) -> i32 {
    let mut score = 0;
    let my_pieces = board.colori[c.indice()];
    let enemy_pawns = board.pezzi[Pezzo::Pedone.indice()] & board.colori[c.opposto().indice()];

    for sq in 0..64 {
        let bit = 1u64 << sq;
        if (my_pieces & bit) == 0 { continue; }

        // Indice speculare per le tabelle (Bianco: Rank 7 in alto, Nero: Rank 0 in alto)
        let rank = sq / 8;
        let file = sq % 8;
        let table_idx = if c == Colore::Bianco { (7 - rank) * 8 + file } else { rank * 8 + file };

        let p_type = match board.get_piece_at(sq) {
            Some(p) => p,
            None => continue,
        };

        match p_type {
            Pezzo::Pedone => {
                score += 100 + PAWN_TABLE[table_idx];
                if is_passed_pawn(sq, c, enemy_pawns) {
                    let rel_rank = if c == Colore::Bianco { rank } else { 7 - rank };
                    // Bonus quadratico per pedone passato che avanza
                    score += (rel_rank as i32 * rel_rank as i32) * 5;
                }
            },
            Pezzo::Cavallo => score += 320 + KNIGHT_TABLE[table_idx],
            Pezzo::Alfiere => score += 330 + BISHOP_TABLE[table_idx],
            Pezzo::Torre   => score += 500,
            Pezzo::Regina  => score += 900,
            Pezzo::Re      => {
                score += KING_TABLE_MG[table_idx];
                score += check_king_safety(sq, board.pezzi[0] & my_pieces, c);
            },
        }
    }
    score
}

// Controlla se un pedone è passato (nessun pedone nemico davanti o sulle colonne adiacenti)
fn is_passed_pawn(sq: usize, color: Colore, enemy_pawns: Bitboard) -> bool {
    let file = (sq % 8) as isize;
    let rank = sq / 8;
    let mut mask: u64 = 0;

    for f in (file - 1)..=(file + 1) {
        if f < 0 || f > 7 { continue; }
        
        let f_mask = 0x0101010101010101u64 << f;
        
        // Evitiamo overflow dello shift con controlli rigorosi
        if color == Colore::Bianco {
            // Mask per i rank SUPERIORI a quello corrente
            if rank < 7 {
                let shift_amount = (rank + 1) * 8;
                // !0u64 << 64 farebbe panic in debug, quindi controlliamo rank < 7
                mask |= f_mask & (!0u64 << shift_amount);
            }
        } else {
            // Mask per i rank INFERIORI a quello corrente
            if rank > 0 {
                let shift_amount = (8 - rank) * 8;
                // !0u64 >> 64 farebbe panic
                mask |= f_mask & (!0u64 >> shift_amount);
            }
        }
    }
    (enemy_pawns & mask) == 0
}

fn check_king_safety(king_sq: usize, my_pawns: u64, c: Colore) -> i32 {
    let rank = king_sq / 8;
    
    // Sicurezza valida solo se il re è sulla traversa di fondo o quasi
    // Usiamo i controlli per evitare underflow nel calcolo di shield_rank
    if c == Colore::Bianco {
        if rank > 1 { return 0; } // Re bianco troppo avanzato per considerare lo scudo pedonale classico
        let shield_rank = rank + 1;
        
        let shield_mask = 0xFFu64 << (shield_rank * 8);
        return calculate_shield_bonus(king_sq, my_pawns, shield_mask);
    } else {
        if rank < 6 { return 0; } // Re nero troppo avanzato
        let shield_rank = rank - 1;
        
        let shield_mask = 0xFFu64 << (shield_rank * 8);
        return calculate_shield_bonus(king_sq, my_pawns, shield_mask);
    }
}

fn calculate_shield_bonus(king_sq: usize, my_pawns: u64, shield_rank_mask: u64) -> i32 {
    let king_file = (king_sq % 8) as isize;
    let mut file_mask = 0u64;
    
    for f in (king_file - 1)..=(king_file + 1) {
        if f >= 0 && f <= 7 {
            file_mask |= 0x0101010101010101u64 << f;
        }
    }

    let shield = my_pawns & shield_rank_mask & file_mask;
    shield.count_ones() as i32 * 10
}

// --- TABELLE DI BACKUP ---

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
    -40,-20,  0,  5,  5,  0,-20,-40,
    -30,  5, 10, 15, 15, 10,  5,-30,
    -30,  0, 15, 20, 20, 15,  0,-30,
    -30,  5, 15, 20, 20, 15,  5,-30,
    -30,  0, 10, 15, 15, 10,  0,-30,
    -40,-20,  0,  0,  0,  0,-20,-40,
    -50,-40,-30,-30,-30,-30,-40,-50,
];

const BISHOP_TABLE: [i32; 64] = [
    -20,-10,-10,-10,-10,-10,-10,-20,
    -10,  5,  0,  0,  0,  0,  5,-10,
    -10, 10, 10, 10, 10, 10, 10,-10,
    -10,  0, 10, 10, 10, 10,  0,-10,
    -10,  5,  5, 10, 10,  5,  5,-10,
    -10,  0,  5, 10, 10,  5,  0,-10,
    -10,  0,  0,  0,  0,  0,  0,-10,
    -20,-10,-10,-10,-10,-10,-10,-20,
];

const KING_TABLE_MG: [i32; 64] = [
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -20,-30,-30,-40,-40,-30,-30,-20,
    -10,-20,-20,-20,-20,-20,-20,-10,
     20, 20, -5, -5, -5, -5, 20, 20,
     20, 30, 10,  0,  0, 10, 30, 20
];