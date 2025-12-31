// src/attacks.rs

use crate::board::{Bitboard, Colore};

// Tabelle pre-calcolate
static mut KNIGHT_ATTACKS: [Bitboard; 64] = [0; 64];
static mut KING_ATTACKS: [Bitboard; 64] = [0; 64];
// [Colore][Casella]: 0=Bianco, 1=Nero
static mut PAWN_ATTACKS: [[Bitboard; 64]; 2] = [[0; 64]; 2];

// Costanti per le colonne (per evitare di andare "a capo" dalla colonna H alla A)
const FILE_A_BB: u64 = 0x0101010101010101;
const FILE_H_BB: u64 = 0x8080808080808080;

// Flag per inizializzazione
static mut INITIALIZED: bool = false;

/// Inizializza le tabelle di attacco.
/// DEVE essere chiamata all'avvio del programma.
pub fn init() {
    unsafe {
        if INITIALIZED { return; }
        
        init_leapers();
        // init_sliders(); // Se usi Magic Bitboards, inizializzali qui
        
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

// --- Generatori di maschere ---

fn mask_pawn_attacks(sq: usize, side: Colore) -> Bitboard {
    let mut attacks: Bitboard = 0;
    let bit = 1u64 << sq;

    if side == Colore::Bianco {
        // I bianchi mangiano verso "Nord" (+8) a destra (+1) e sinistra (-1)
        // Nord-Est (+9) (Se non siamo sulla colonna H)
        if (bit & FILE_H_BB) == 0 { attacks |= bit << 9; }
        // Nord-Ovest (+7) (Se non siamo sulla colonna A)
        if (bit & FILE_A_BB) == 0 { attacks |= bit << 7; }
    } else {
        // I neri mangiano verso "Sud" (-8)
        // Sud-Est (-7) (Se non siamo sulla colonna H - vista dal bianco è +1)
        if (bit & FILE_H_BB) == 0 { attacks |= bit >> 7; }
        // Sud-Ovest (-9) (Se non siamo sulla colonna A)
        if (bit & FILE_A_BB) == 0 { attacks |= bit >> 9; }
    }
    attacks
}

fn mask_knight_attacks(sq: usize) -> Bitboard {
    let mut attacks: Bitboard = 0;
    let b = 1u64 << sq;
    
    // Calcoliamo gli attacchi e mascheriamo le colonne per evitare wrap-around
    let file_a = 0x0101010101010101;
    let file_b = 0x0202020202020202;
    let file_g = 0x4040404040404040;
    let file_h = 0x8080808080808080;
    
    // NNE, NNW
    attacks |= (b << 17) & !file_a;
    attacks |= (b << 15) & !file_h;
    // ENE, ESE
    attacks |= (b << 10) & !(file_a | file_b);
    attacks |= (b >>  6) & !(file_a | file_b);
    // SSE, SSW
    attacks |= (b >> 15) & !file_a;
    attacks |= (b >> 17) & !file_h;
    // WNW, WSW
    attacks |= (b <<  6) & !(file_g | file_h);
    attacks |= (b >> 10) & !(file_g | file_h);

    attacks
}

fn mask_king_attacks(sq: usize) -> Bitboard {
    let mut attacks: Bitboard = 0;
    let b = 1u64 << sq;
    let file = sq % 8;
    
    // Ovest
    if file > 0 { 
        attacks |= b >> 1;   
        attacks |= b >> 9; 
        attacks |= b << 7; 
    } 
    // Est
    if file < 7 { 
        attacks |= b << 1;   
        attacks |= b << 9; 
        attacks |= b >> 7; 
    } 
    attacks |= b << 8; // Nord
    attacks |= b >> 8; // Sud
    
    attacks
}

// --- Funzioni Pubbliche di Accesso ---

// NUOVA FUNZIONE AGGIUNTA CHE RISOLVE L'ERRORE
#[inline(always)]
pub fn pawn_attacks(sq: usize, side: Colore) -> Bitboard {
    unsafe { PAWN_ATTACKS[side.indice()][sq] }
}

#[inline(always)]
pub fn knight_attacks(sq: usize) -> Bitboard {
    unsafe { KNIGHT_ATTACKS[sq] }
}

#[inline(always)]
pub fn king_attacks(sq: usize) -> Bitboard {
    unsafe { KING_ATTACKS[sq] }
}

// Slider Attacks (Placeholder o Magic Bitboards)
pub fn bishop_attacks(sq: usize, occ: Bitboard) -> Bitboard {
    generate_slider_attacks(sq, occ, true)
}

pub fn rook_attacks(sq: usize, occ: Bitboard) -> Bitboard {
    generate_slider_attacks(sq, occ, false)
}

pub fn queen_attacks(sq: usize, occ: Bitboard) -> Bitboard {
    bishop_attacks(sq, occ) | rook_attacks(sq, occ)
}

// Funzione fallback lenta (Se non usi Magic Bitboards, il motore sarà lento qui)
// Per un motore PRO, questo andrebbe sostituito con Magic Bitboards.
fn generate_slider_attacks(sq: usize, occ: Bitboard, diagonal: bool) -> Bitboard {
    let mut attacks = 0;
    let r = (sq / 8) as i32;
    let f = (sq % 8) as i32;
    
    let deltas = if diagonal {
        [ (1,1), (1,-1), (-1,1), (-1,-1) ] // Alfiere
    } else {
        [ (1,0), (-1,0), (0,1), (0,-1) ]   // Torre
    };

    for (dr, df) in deltas {
        for i in 1..8 {
            let nr = r + dr * i;
            let nf = f + df * i;
            if nr < 0 || nr > 7 || nf < 0 || nf > 7 { break; }
            let bit = 1u64 << (nr * 8 + nf);
            attacks |= bit;
            if (bit & occ) != 0 { break; }
        }
    }
    attacks
}