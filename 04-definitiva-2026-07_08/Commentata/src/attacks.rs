use crate::board::{Bitboard, Colore, Scacchiera, Pezzo};
use std::sync::OnceLock;

// --- 1. STRUTTURA TABELLE (Solo per pezzi non-slider) ---
/// Contiene i database pre-calcolati di bitboard per i pezzi a balzo (leapers) e pedoni[cite: 5].
pub struct AttackTables {
    /// Matrice degli attacchi dei pedoni: [colore (0=Bianco, 1=Nero)][casa originaria (0..64)][cite: 5]
    pub pawn_attacks: [[Bitboard; 64]; 2],
    /// Vettore degli attacchi del cavallo per ciascuna delle 64 caselle[cite: 5].
    pub knight_attacks: [Bitboard; 64],
    /// Vettore degli attacchi del re per ciascuna delle 64 caselle[cite: 5].
    pub king_attacks: [Bitboard; 64],
}

/// Sincronizzazione globale unica per garantire la sicurezza thread-safe durante l'inizializzazione[cite: 5].
static TABLES: OnceLock<AttackTables> = OnceLock::new();

/// Restituisce un riferimento statico alle tabelle degli attacchi pre-calcolate,
/// inizializzandole in modo lazy al primo utilizzo[cite: 5].
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

/// Recupera il bitboard degli attacchi di un pedone data la casa e il colore del pezzo[cite: 5].
#[inline(always)]
pub fn pawn_attacks(sq: usize, side: Colore) -> Bitboard {
    get_tables().pawn_attacks[side.indice()][sq]
}

/// Recupera il bitboard degli attacchi di un cavallo data la sua casa[cite: 5].
#[inline(always)]
pub fn knight_attacks(sq: usize) -> Bitboard {
    get_tables().knight_attacks[sq]
}

/// Recupera il bitboard degli attacchi del re data la sua casa[cite: 5].
#[inline(always)]
pub fn king_attacks(sq: usize) -> Bitboard {
    get_tables().king_attacks[sq]
}

// --- 2. CALCOLO AL VOLO PER SLIDER (Ray-Casting Geometrico) ---

/// Calcola al volo gli attacchi dell'alfiere tenendo conto dei blocchi dinamici sulla scacchiera[cite: 5].
#[inline(always)]
pub fn bishop_attacks(sq: usize, occ: Bitboard) -> Bitboard {
    let mut atk = 0;
    let r = (sq / 8) as i32; 
    let f = (sq % 8) as i32;
    
    // Esplorazione delle 4 direzioni diagonali: NE, SE, SW, NW[cite: 5]
    for &(dr, df) in &[(1,1), (1,-1), (-1,1), (-1,-1)] {
        let mut nr = r + dr; 
        let mut nf = f + df;
        while nr >= 0 && nr < 8 && nf >= 0 && nf < 8 {
            let bit = 1u64 << (nr * 8 + nf);
            atk |= bit;
            // Se la casa è occupata da un qualsiasi pezzo, il raggio si interrompe[cite: 5].
            if (occ & bit) != 0 { break; } 
            nr += dr; 
            nf += df;
        }
    }
    atk
}

/// Calcola al volo gli attacchi della torre tenendo conto dei blocchi dinamici sulla scacchiera[cite: 5].
#[inline(always)]
pub fn rook_attacks(sq: usize, occ: Bitboard) -> Bitboard {
    let mut atk = 0;
    let r = (sq / 8) as i32; 
    let f = (sq % 8) as i32;
    
    // Esplorazione delle 4 direzioni ortogonali: Nord, Sud, Est, Ovest[cite: 5]
    for &(dr, df) in &[(1,0), (-1,0), (0,1), (0,-1)] {
        let mut nr = r + dr; 
        let mut nf = f + df;
        while nr >= 0 && nr < 8 && nf >= 0 && nf < 8 {
            let bit = 1u64 << (nr * 8 + nf);
            atk |= bit;
            // Se viene colpito un ostacolo, il raggio si arresta[cite: 5].
            if (occ & bit) != 0 { break; } 
            nr += dr; 
            nf += df;
        }
    }
    atk
}

/// Combina le matrici di attacco ortogonali e diagonali per generare la mappa d'attacco della Regina[cite: 5].
#[inline(always)]
pub fn queen_attacks(sq: usize, occ: Bitboard) -> Bitboard {
    bishop_attacks(sq, occ) | rook_attacks(sq, occ)
}

// --- 3. VERIFICA STATO DI ATTACCO DI UNA CASELLA ---

/// Ritorna true se la casella specificata (`sq`) è sotto l'attacco diretto del colore avversario (`side_attacker`)[cite: 5].
pub fn square_attacked(board: &Scacchiera, sq: usize, side_attacker: Colore) -> bool {
    let occ = board.occupazione();
    let targets = board.colori[side_attacker.indice()];
    
    // 1. Controllo Pedoni (Sfrutta la simmetria: un attacco invertito dal punto di vista della casa d'arrivo)[cite: 5]
    if (pawn_attacks(sq, side_attacker.opposto()) & board.pezzi[Pezzo::Pedone.indice()] & targets) != 0 { return true; }
    
    // 2. Controllo Cavalli[cite: 5]
    if (knight_attacks(sq) & board.pezzi[Pezzo::Cavallo.indice()] & targets) != 0 { return true; }
    
    // 3. Controllo Re[cite: 5]
    if (king_attacks(sq) & board.pezzi[Pezzo::Re.indice()] & targets) != 0 { return true; }

    // 4. Controllo Sliders (Sfrutta la simmetria geometrica dei raggi)[cite: 5]
    
    // Alfiere / Regina[cite: 5]
    if (bishop_attacks(sq, occ) & (board.pezzi[Pezzo::Alfiere.indice()] | board.pezzi[Pezzo::Regina.indice()]) & targets) != 0 { return true; }
    
    // Torre / Regina[cite: 5]
    if (rook_attacks(sq, occ) & (board.pezzi[Pezzo::Torre.indice()] | board.pezzi[Pezzo::Regina.indice()]) & targets) != 0 { return true; }

    false
}

// --- 4. ENGINE DI INIZIALIZZAZIONE MASCHERE ---
/// Genera iterativamente tutti i bitmask costanti all'avvio del programma per i pezzi non-slider[cite: 5].
fn init_tables(t: &mut AttackTables) {
    for sq in 0..64 {
        let b = 1u64 << sq;
        
        // Pedoni Bianchi (Avanzano verso indici crescenti / traversa + 1)[cite: 5]
        if sq < 56 {
            if sq % 8 > 0 { t.pawn_attacks[0][sq] |= b << 7; } // Nord-Ovest (Cattura a sinistra)[cite: 5]
            if sq % 8 < 7 { t.pawn_attacks[0][sq] |= b << 9; } // Nord-Est (Cattura a destra)[cite: 5]
        }
        // Pedoni Neri (Avanzano verso indici decrescenti / traversa - 1)[cite: 5]
        if sq > 7 {
            if sq % 8 > 0 { t.pawn_attacks[1][sq] |= b >> 9; } // Sud-Ovest[cite: 5]
            if sq % 8 < 7 { t.pawn_attacks[1][sq] |= b >> 7; } // Sud-Est[cite: 5]
        }

        // Cavalli: Proiezione a raggio fisso (2,1), (1,2) in tutte le combinazioni di segno[cite: 5]
        let r = (sq / 8) as i32;
        let f = (sq % 8) as i32;
        for &(dr, df) in &[(2,1),(2,-1),(-2,1),(-2,-1),(1,2),(1,-2),(-1,2),(-1,-2)] {
            let nr = r + dr; let nf = f + df;
            if nr >= 0 && nr < 8 && nf >= 0 && nf < 8 { 
                t.knight_attacks[sq] |= 1u64 << (nr * 8 + nf); 
            }
        }

        // Re: Perimetro immediato di 1 casa attorno alla coordinata d'origine[cite: 5]
        for dr in -1..=1 {
            for df in -1..=1 {
                if dr == 0 && df == 0 { continue; } // Salta la casa stessa[cite: 5]
                let nr = r + dr; let nf = f + df;
                if nr >= 0 && nr < 8 && nf >= 0 && nf < 8 { 
                    t.king_attacks[sq] |= 1u64 << (nr * 8 + nf); 
                }
            }
        }
    }
}