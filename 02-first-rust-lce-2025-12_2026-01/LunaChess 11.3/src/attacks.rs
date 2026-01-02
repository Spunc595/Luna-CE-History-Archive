use crate::board::{Bitboard, Colore};

// Tabelle pre-calcolate (Static Mutable)
static mut KNIGHT_ATTACKS: [Bitboard; 64] = [0; 64];
static mut KING_ATTACKS: [Bitboard; 64] = [0; 64];
// [Colore][Casella]: 0=Bianco, 1=Nero
static mut PAWN_ATTACKS: [[Bitboard; 64]; 2] = [[0; 64]; 2];

// Flag per assicurarsi che init() venga chiamato una sola volta
static mut INITIALIZED: bool = false;

/// Inizializza le tabelle di attacco.
/// IMPORTANTE: Deve essere chiamata in `main.rs` prima di qualsiasi calcolo!
pub fn init() {
    unsafe {
        if INITIALIZED { return; }
        
        init_leapers();
        // Gli sliders (Torri/Alfieri) li calcoliamo "on-the-fly" nel generatore qui sotto,
        // ma se volessi usare Magic Bitboards, qui inizializzeresti le loro tabelle.
        
        INITIALIZED = true;
    }
}

fn init_leapers() {
    for sq in 0..64 {
        unsafe {
            KNIGHT_ATTACKS[sq] = mask_knight_attacks(sq);
            KING_ATTACKS[sq] = mask_king_attacks(sq);
            
            // Inizializza attacchi pedoni (Bianco=0, Nero=1)
            PAWN_ATTACKS[0][sq] = mask_pawn_attacks(sq, Colore::Bianco);
            PAWN_ATTACKS[1][sq] = mask_pawn_attacks(sq, Colore::Nero);
        }
    }
}

// --- Generatori di maschere (Basati su Coordinate per Sicurezza) ---

fn mask_pawn_attacks(sq: usize, side: Colore) -> Bitboard {
    let mut attacks: Bitboard = 0;
    let r = (sq / 8) as i8;
    let f = (sq % 8) as i8;

    // Definiamo la direzione di cattura (Bianco: rank +1, Nero: rank -1)
    let dr = if side == Colore::Bianco { 1 } else { -1 };
    
    // I pedoni mangiano in diagonale: (rank + dr, file - 1) e (rank + dr, file + 1)
    for df in [-1, 1] {
        let nr = r + dr;
        let nf = f + df;
        
        if nr >= 0 && nr < 8 && nf >= 0 && nf < 8 {
            attacks |= 1u64 << (nr * 8 + nf);
        }
    }
    attacks
}

fn mask_knight_attacks(sq: usize) -> Bitboard {
    let mut attacks: Bitboard = 0;
    let r = (sq / 8) as i8;
    let f = (sq % 8) as i8;
    
    // Tutte le 8 possibili mosse del cavallo (Delta Rank, Delta File)
    let jumps = [
        (2, 1), (2, -1),   // NNE, NNW
        (-2, 1), (-2, -1), // SSE, SSW
        (1, 2), (1, -2),   // ENE, ESE
        (-1, 2), (-1, -2)  // WNW, WSW
    ];

    for (dr, df) in jumps {
        let nr = r + dr;
        let nf = f + df;
        // Controlliamo i bordi per evitare il wrap-around (es. H1 -> A2)
        if nr >= 0 && nr < 8 && nf >= 0 && nf < 8 {
            attacks |= 1u64 << (nr * 8 + nf);
        }
    }
    attacks
}

fn mask_king_attacks(sq: usize) -> Bitboard {
    let mut attacks: Bitboard = 0;
    let r = (sq / 8) as i8;
    let f = (sq % 8) as i8;

    for dr in -1..=1 {
        for df in -1..=1 {
            if dr == 0 && df == 0 { continue; } // Il Re non può rimanere fermo
            
            let nr = r + dr;
            let nf = f + df;
            
            if nr >= 0 && nr < 8 && nf >= 0 && nf < 8 {
                attacks |= 1u64 << (nr * 8 + nf);
            }
        }
    }
    attacks
}

// --- Funzioni Pubbliche di Accesso ---

#[inline(always)]
pub fn pawn_attacks(sq: usize, side: Colore) -> Bitboard {
    unsafe { 
        // Se non è inizializzato, potremmo restituire 0 o fare panic, 
        // ma per velocità assumiamo che init() sia stato chiamato.
        PAWN_ATTACKS[side.indice()][sq] 
    }
}

#[inline(always)]
pub fn knight_attacks(sq: usize) -> Bitboard {
    unsafe { KNIGHT_ATTACKS[sq] }
}

#[inline(always)]
pub fn king_attacks(sq: usize) -> Bitboard {
    unsafe { KING_ATTACKS[sq] }
}

// Slider Attacks: Calcolati al volo (Ray-Casting)
// Questo è il metodo più sicuro. Se vorrai ottimizzare in futuro, 
// sostituirai queste con Magic Bitboards.

#[inline(always)]
pub fn bishop_attacks(sq: usize, occ: Bitboard) -> Bitboard {
    generate_slider_attacks(sq, occ, true)
}

#[inline(always)]
pub fn rook_attacks(sq: usize, occ: Bitboard) -> Bitboard {
    generate_slider_attacks(sq, occ, false)
}

#[inline(always)]
pub fn queen_attacks(sq: usize, occ: Bitboard) -> Bitboard {
    bishop_attacks(sq, occ) | rook_attacks(sq, occ)
}

fn generate_slider_attacks(sq: usize, occ: Bitboard, diagonal: bool) -> Bitboard {
    let mut attacks = 0;
    let r = (sq / 8) as i8;
    let f = (sq % 8) as i8;
    
    // Direzioni: (Delta Rank, Delta File)
    let dirs = if diagonal {
        [(1, 1), (1, -1), (-1, 1), (-1, -1)] // Alfiere: NE, NW, SE, SW
    } else {
        [(1, 0), (-1, 0), (0, 1), (0, -1)]   // Torre: N, S, E, W
    };

    for (dr, df) in dirs {
        for i in 1..8 { // Massimo 7 passi in ogni direzione
            let nr = r + dr * i;
            let nf = f + df * i;
            
            // Se usciamo dalla scacchiera, stop in questa direzione
            if nr < 0 || nr > 7 || nf < 0 || nf > 7 { break; }
            
            let bit = 1u64 << (nr * 8 + nf);
            attacks |= bit;
            
            // Se colpiamo un pezzo (qualsiasi colore), ci fermiamo (inclusa la casa del pezzo colpito)
            if (occ & bit) != 0 { break; }
        }
    }
    attacks
}