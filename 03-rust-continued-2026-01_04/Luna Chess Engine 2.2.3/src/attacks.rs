use crate::board::{Bitboard, Colore, Scacchiera, Pezzo};
use std::sync::OnceLock;

// --- STRUTTURA TABELLE (Solo per pezzi non-slider) ---
pub struct AttackTables {
    pub pawn_attacks: [[Bitboard; 64]; 2],
    pub knight_attacks: [Bitboard; 64],
    pub king_attacks: [Bitboard; 64],
}

static TABLES: OnceLock<AttackTables> = OnceLock::new();

#[inline(always)]
pub fn get_tables() -> &'static AttackTables {
    TABLES.get_or_init(|| {
        let mut tables = AttackTables {
            pawn_attacks: [[0; 64]; 2],
            knight_attacks: [0; 64],
            king_attacks: [0; 64],
        };
        init_tables(&mut tables);
        tables
    })
}

#[inline(always)]
pub fn pawn_attacks(sq: usize, side: Colore) -> Bitboard {
    get_tables().pawn_attacks[side.indice()][sq]
}

#[inline(always)]
pub fn knight_attacks(sq: usize) -> Bitboard {
    get_tables().knight_attacks[sq]
}

#[inline(always)]
pub fn king_attacks(sq: usize) -> Bitboard {
    get_tables().king_attacks[sq]
}

// --- CALCOLO AL VOLO PER SLIDER (Sicuro al 100%) ---

#[inline(always)]
pub fn bishop_attacks(sq: usize, occ: Bitboard) -> Bitboard {
    let mut atk = 0;
    let r = (sq / 8) as i32; 
    let f = (sq % 8) as i32;
    
    // 4 Direzioni: NE, SE, SW, NW
    for &(dr, df) in &[(1,1), (1,-1), (-1,1), (-1,-1)] {
        let mut nr = r + dr; 
        let mut nf = f + df;
        while nr >= 0 && nr < 8 && nf >= 0 && nf < 8 {
            let bit = 1u64 << (nr * 8 + nf);
            atk |= bit;
            if (occ & bit) != 0 { break; } // Colpito ostacolo (amico o nemico), stop.
            nr += dr; 
            nf += df;
        }
    }
    atk
}

#[inline(always)]
pub fn rook_attacks(sq: usize, occ: Bitboard) -> Bitboard {
    let mut atk = 0;
    let r = (sq / 8) as i32; 
    let f = (sq % 8) as i32;
    
    // 4 Direzioni: Nord, Sud, Est, Ovest
    for &(dr, df) in &[(1,0), (-1,0), (0,1), (0,-1)] {
        let mut nr = r + dr; 
        let mut nf = f + df;
        while nr >= 0 && nr < 8 && nf >= 0 && nf < 8 {
            let bit = 1u64 << (nr * 8 + nf);
            atk |= bit;
            if (occ & bit) != 0 { break; } // Colpito ostacolo, stop.
            nr += dr; 
            nf += df;
        }
    }
    atk
}

#[inline(always)]
pub fn queen_attacks(sq: usize, occ: Bitboard) -> Bitboard {
    bishop_attacks(sq, occ) | rook_attacks(sq, occ)
}

// Wrapper per verificare se una casella è attaccata
pub fn square_attacked(board: &Scacchiera, sq: usize, side_attacker: Colore) -> bool {
    let occ = board.occupazione();
    let targets = board.colori[side_attacker.indice()];
    
    // Check pedoni (attacchi inversi: da chi attacca verso sq)
    if (pawn_attacks(sq, side_attacker.opposto()) & board.pezzi[Pezzo::Pedone.indice()] & targets) != 0 { return true; }
    
    // Check cavalli
    if (knight_attacks(sq) & board.pezzi[Pezzo::Cavallo.indice()] & targets) != 0 { return true; }
    
    // Check Re
    if (king_attacks(sq) & board.pezzi[Pezzo::Re.indice()] & targets) != 0 { return true; }

    // Check Sliders (calcolati al volo, molto sicuri)
    // Se un alfiere nemico può attaccare sq, significa che un alfiere in sq può attaccare quel nemico.
    // Usiamo questa simmetria.
    
    // Alfiere / Regina
    if (bishop_attacks(sq, occ) & (board.pezzi[Pezzo::Alfiere.indice()] | board.pezzi[Pezzo::Regina.indice()]) & targets) != 0 { return true; }
    
    // Torre / Regina
    if (rook_attacks(sq, occ) & (board.pezzi[Pezzo::Torre.indice()] | board.pezzi[Pezzo::Regina.indice()]) & targets) != 0 { return true; }

    false
}

// --- INIZIALIZZAZIONE ---
fn init_tables(t: &mut AttackTables) {
    for sq in 0..64 {
        let b = 1u64 << sq;
        
        // Pedoni Bianchi (attaccano verso rank+1)
        if sq < 56 {
            if sq % 8 > 0 { t.pawn_attacks[0][sq] |= b << 7; } // NW
            if sq % 8 < 7 { t.pawn_attacks[0][sq] |= b << 9; } // NE
        }
        // Pedoni Neri (attaccano verso rank-1)
        if sq > 7 {
            if sq % 8 > 0 { t.pawn_attacks[1][sq] |= b >> 9; } // SW
            if sq % 8 < 7 { t.pawn_attacks[1][sq] |= b >> 7; } // SE
        }

        // Cavalli
        let r = (sq / 8) as i32;
        let f = (sq % 8) as i32;
        for &(dr, df) in &[(2,1),(2,-1),(-2,1),(-2,-1),(1,2),(1,-2),(-1,2),(-1,-2)] {
            let nr = r + dr; let nf = f + df;
            if nr >= 0 && nr < 8 && nf >= 0 && nf < 8 { 
                t.knight_attacks[sq] |= 1u64 << (nr * 8 + nf); 
            }
        }

        // Re
        for dr in -1..=1 {
            for df in -1..=1 {
                if dr == 0 && df == 0 { continue; }
                let nr = r + dr; let nf = f + df;
                if nr >= 0 && nr < 8 && nf >= 0 && nf < 8 { 
                    t.king_attacks[sq] |= 1u64 << (nr * 8 + nf); 
                }
            }
        }
    }
}