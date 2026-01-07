use crate::board::Bitboard;
use lazy_static::lazy_static;

// ============================================
// FUNZIONI DI CALCOLO PER LE TABELLE
// ============================================

fn compute_attacchi_cavallo() -> [Bitboard; 64] {
    let mut table = [0; 64];
    let offsets = [(-2, -1), (-2, 1), (-1, -2), (-1, 2),
                   (1, -2), (1, 2), (2, -1), (2, 1)];
    
    for square in 0..64 {
        let rank = (square / 8) as i32;
        let file = (square % 8) as i32;
        let mut attacks = 0;
        
        for &(dr, df) in &offsets {
            let new_rank = rank + dr;
            let new_file = file + df;
            
            if new_rank >= 0 && new_rank < 8 && new_file >= 0 && new_file < 8 {
                attacks |= 1 << (new_rank * 8 + new_file);
            }
        }
        
        table[square] = attacks;
    }
    
    table
}

fn compute_attacchi_re() -> [Bitboard; 64] {
    let mut table = [0; 64];
    
    for square in 0..64 {
        let rank = (square / 8) as i32;
        let file = (square % 8) as i32;
        let mut attacks = 0;
        
        for dr in -1..=1 {
            for df in -1..=1 {
                if dr == 0 && df == 0 {
                    continue;
                }
                
                let new_rank = rank + dr;
                let new_file = file + df;
                
                if new_rank >= 0 && new_rank < 8 && new_file >= 0 && new_file < 8 {
                    attacks |= 1 << (new_rank * 8 + new_file);
                }
            }
        }
        
        table[square] = attacks;
    }
    
    table
}

// ============================================
// TABELLE INIZIALIZZATE CON lazy_static
// ============================================

lazy_static! {
    pub static ref ATTACCHI_CAVALLO: [Bitboard; 64] = compute_attacchi_cavallo();
    pub static ref ATTACCHI_RE: [Bitboard; 64] = compute_attacchi_re();
}

// ============================================
// FUNZIONI DI ACCESSO PUBBLICHE
// ============================================

pub fn attacchi_cavallo(square: usize) -> Bitboard {
    ATTACCHI_CAVALLO[square]
}

pub fn attacchi_re(square: usize) -> Bitboard {
    ATTACCHI_RE[square]
}