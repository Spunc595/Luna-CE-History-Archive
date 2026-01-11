use crate::board::{Bitboard, Colore};
use std::sync::OnceLock;

// --- STRUTTURA TABELLE ---
// Contiene le maschere pre-calcolate per i pezzi che non scivolano (Steppers)
pub struct AttackTables {
    pub pawn_attacks: [[Bitboard; 64]; 2],
    pub knight_attacks: [Bitboard; 64],
    pub king_attacks: [Bitboard; 64],
}

// Singleton globale: viene calcolato solo la prima volta che viene chiamato
static TABLES: OnceLock<AttackTables> = OnceLock::new();

// Funzione interna per ottenere il riferimento alle tabelle
fn get_tables() -> &'static AttackTables {
    TABLES.get_or_init(|| AttackTables {
        pawn_attacks: init_pawn_attacks(),
        knight_attacks: init_knight_attacks(),
        king_attacks: init_king_attacks(),
    })
}

// --- API PUBBLICHE (VELOCI) ---

/// Ritorna le case attaccate da un pedone di un dato colore in una data casa.
/// Nota: Include solo le catture diagonali, non il movimento in avanti.
#[inline(always)]
pub fn pawn_attacks(sq: usize, side: Colore) -> Bitboard {
    get_tables().pawn_attacks[side.indice()][sq]
}

/// Ritorna le case attaccate da un cavallo.
#[inline(always)]
pub fn knight_attacks(sq: usize) -> Bitboard {
    get_tables().knight_attacks[sq]
}

/// Ritorna le case attaccate dal re.
#[inline(always)]
pub fn king_attacks(sq: usize) -> Bitboard {
    get_tables().king_attacks[sq]
}

// --- API SLIDERS (RAY-CASTING) ---
// Questi calcolano le mosse al volo. È un po' più lento dei Magic Bitboards,
// ma molto più semplice e sicuro da implementare.

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
    // La Regina unisce i movimenti di Torre e Alfiere
    bishop_attacks(sq, occ) | rook_attacks(sq, occ)
}

/// Genera attacchi scorrevoli "on-the-fly" bloccandosi quando incontra pezzi (occ).
fn generate_slider_attacks(sq: usize, occ: Bitboard, diagonal: bool) -> Bitboard {
    let mut attacks = 0u64;
    let r = (sq / 8) as i8;
    let f = (sq % 8) as i8;
    
    // Definisce le direzioni: Diagonali o Ortogonali
    let dirs: &[(i8, i8)] = if diagonal {
        &[(1, 1), (1, -1), (-1, 1), (-1, -1)]
    } else {
        &[(1, 0), (-1, 0), (0, 1), (0, -1)]
    };

    for &(dr, df) in dirs {
        let mut nr = r + dr;
        let mut nf = f + df;
        
        while nr >= 0 && nr < 8 && nf >= 0 && nf < 8 {
            let bit = 1u64 << (nr * 8 + nf);
            attacks |= bit;
            
            // Se c'è un pezzo (amico o nemico), il raggio si ferma qui
            // (includiamo la casa del blocco come attaccata, la logica di non catturare
            // i pezzi amici viene gestita in movegen.rs)
            if (occ & bit) != 0 {
                break;
            }
            
            nr += dr;
            nf += df;
        }
    }
    attacks
}

// --- INIZIALIZZATORI (Eseguiti una sola volta all'avvio) ---

fn init_pawn_attacks() -> [[Bitboard; 64]; 2] {
    let mut attacks = [[0; 64]; 2];
    for sq in 0..64 {
        let r = (sq / 8) as i8;
        let f = (sq % 8) as i8;

        // BIANCO (Indice 0): Attacca verso Rank + 1
        // Cattura a sinistra (File - 1) e destra (File + 1)
        if r < 7 {
            if f > 0 { attacks[0][sq] |= 1u64 << (sq + 7); } // +7 = Nord-Ovest
            if f < 7 { attacks[0][sq] |= 1u64 << (sq + 9); } // +9 = Nord-Est
        }

        // NERO (Indice 1): Attacca verso Rank - 1
        if r > 0 {
            if f > 0 { attacks[1][sq] |= 1u64 << (sq - 9); } // -9 = Sud-Ovest
            if f < 7 { attacks[1][sq] |= 1u64 << (sq - 7); } // -7 = Sud-Est
        }
    }
    attacks
}

fn init_knight_attacks() -> [Bitboard; 64] {
    let mut attacks = [0; 64];
    for sq in 0..64 {
        let r = (sq / 8) as i8;
        let f = (sq % 8) as i8;
        // Tutte le 8 possibili mosse del cavallo
        let jumps = [
            (2, 1), (2, -1), (-2, 1), (-2, -1),
            (1, 2), (1, -2), (-1, 2), (-1, -2)
        ];
        
        for (dr, df) in jumps {
            let nr = r + dr;
            let nf = f + df;
            if nr >= 0 && nr < 8 && nf >= 0 && nf < 8 {
                attacks[sq] |= 1u64 << (nr * 8 + nf);
            }
        }
    }
    attacks
}

fn init_king_attacks() -> [Bitboard; 64] {
    let mut attacks = [0; 64];
    for sq in 0..64 {
        let r = (sq / 8) as i8;
        let f = (sq % 8) as i8;
        
        // Il Re si muove di 1 in tutte le direzioni
        for dr in -1..=1 {
            for df in -1..=1 {
                if dr == 0 && df == 0 { continue; }
                
                let nr = r + dr;
                let nf = f + df;
                if nr >= 0 && nr < 8 && nf >= 0 && nf < 8 {
                    attacks[sq] |= 1u64 << (nr * 8 + nf);
                }
            }
        }
    }
    attacks
}