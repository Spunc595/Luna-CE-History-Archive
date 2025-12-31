use crate::board::{Scacchiera, Pezzo, Colore, Casella};
// Importiamo la rete globale definita in nnue.rs
use crate::nnue::NNUE; 

// =============================================================
// VALORI MATERIALI (PeSTO Tuned - Aggressivi per il Centro)
// =============================================================
const MG_VAL: [i32; 6] = [ 85, 345, 375, 490, 1050, 0 ];
const EG_VAL: [i32; 6] = [ 100, 290, 310, 530, 950, 0 ];

// =============================================================
// TABELLE POSIZIONALI (PeSTO)
// =============================================================
#[rustfmt::skip]
const MG_TABLE: [[i32; 64]; 6] = [
    // PEDONE: Spinge al centro, penalizza stare fermi ai bordi
    [   0,   0,   0,   0,   0,   0,   0,   0,
       55,  60,  60,  60,  60,  60,  60,  55,
       10,  10,  25,  35,  35,  25,  10,  10,
        5,   5,  15,  30,  30,  15,   5,   5,
        0,   0,   0,  25,  25,   0,   0,   0,
        5,  -5, -10,   5,   5, -10,  -5,   5,
        5,  10,  10, -25, -25,  10,  10,   5,
        0,   0,   0,   0,   0,   0,   0,   0],

    // CAVALLO: Ama il centro (d4, e4, d5, e5), odia gli angoli
    [-55,-40,-30,-30,-30,-30,-40,-55,
     -40,-20,  0,  5,  5,  0,-20,-40,
     -30,  5, 15, 20, 20, 15,  5,-30,
     -30,  5, 25, 30, 30, 25,  5,-30,
     -30,  5, 25, 30, 30, 25,  5,-30,
     -30,  5, 15, 20, 20, 15,  5,-30,
     -40,-20,  0,  5,  5,  0,-20,-40,
     -55,-40,-30,-30,-30,-30,-40,-55],

    // ALFIERE: Diagonali lunghe e fianchetto
    [-20,-10,-10,-10,-10,-10,-10,-20,
     -10,  5,  0,  0,  0,  0,  5,-10,
     -10, 10, 10, 15, 15, 10, 10,-10,
     -10,  0, 15, 20, 20, 15,  0,-10,
     -10,  5, 10, 20, 20, 10,  5,-10,
     -10,  0, 10, 15, 15, 10,  0,-10,
     -10, 20,  5,  5,  5,  5, 20,-10,
     -20,-10,-10,-10,-10,-10,-10,-20],

    // TORRE: Colonne aperte e 7a traversa (gestito dopo), qui posizionamento base
    [  0,  0,  0,  0,  0,  0,  0,  0,
       5, 15, 15, 15, 15, 15, 15,  5,
      -5,  0,  0,  0,  0,  0,  0, -5,
      -5,  0,  0,  0,  0,  0,  0, -5,
      -5,  0,  0,  0,  0,  0,  0, -5,
      -5,  0,  0,  0,  0,  0,  0, -5,
      -5,  0,  0,  0,  0,  0,  0, -5,
       0,  0,  0,  5,  5,  0,  0,  0],

    // REGINA: Centralizzazione moderata, evitare esposizione prematura
    [-20,-10,-10, -5, -5,-10,-10,-20,
     -10,  0,  5,  0,  0,  5,  0,-10,
     -10,  5,  5,  5,  5,  5,  5,-10,
      -5,  0,  5,  5,  5,  5,  0, -5,
      -5,  0,  5,  5,  5,  5,  0, -5,
     -10,  0,  5,  5,  5,  5,  0,-10,
     -10,  0,  0,  0,  0,  0,  0,-10,
     -20,-10,-10, -5, -5,-10,-10,-20],

    // RE (MIDDLEGAME): Sicurezza estrema, arrocco
    [ 20, 30, 10,  0,  0, 10, 30, 20,
      20, 20,  0,  0,  0,  0, 20, 20,
     -10,-20,-20,-20,-20,-20,-20,-10,
     -20,-30,-30,-40,-40,-30,-30,-20,
     -30,-40,-40,-50,-50,-40,-40,-30,
     -30,-40,-40,-50,-50,-40,-40,-30,
     -30,-40,-40,-50,-50,-40,-40,-30,
     -30,-40,-40,-50,-50,-40,-40,-30]
];

#[rustfmt::skip]
const EG_TABLE: [[i32; 64]; 6] = [
    // PEDONE EG: Corsa a promozione!
    [  0,  0,  0,  0,  0,  0,  0,  0,
      90, 90, 90, 90, 90, 90, 90, 90,
      50, 50, 50, 50, 50, 50, 50, 50,
      30, 30, 30, 30, 30, 30, 30, 30,
      20, 20, 20, 20, 20, 20, 20, 20,
      10, 10, 10, 10, 10, 10, 10, 10,
      10, 10, 10, 10, 10, 10, 10, 10,
       0,  0,  0,  0,  0,  0,  0,  0],

    // CAVALLO EG
    [-50,-40,-30,-30,-30,-30,-40,-50,
     -40,-20,  0,  0,  0,  0,-20,-40,
     -30,  0, 10, 15, 15, 10,  0,-30,
     -30,  5, 15, 20, 20, 15,  5,-30,
     -30,  0, 15, 20, 20, 15,  0,-30,
     -30,  5, 10, 15, 15, 10,  5,-30,
     -40,-20,  0,  5,  5,  0,-20,-40,
     -50,-40,-30,-30,-30,-30,-40,-50],

    // ALFIERE EG
    [-20,-10,-10,-10,-10,-10,-10,-20,
     -10,  0,  0,  0,  0,  0,  0,-10,
     -10,  0,  5, 10, 10,  5,  0,-10,
     -10,  5,  5, 10, 10,  5,  5,-10,
     -10,  0, 10, 10, 10, 10,  0,-10,
     -10, 10, 10, 10, 10, 10, 10,-10,
     -10,  5,  0,  0,  0,  0,  5,-10,
     -20,-10,-10,-10,-10,-10,-10,-20],

    // TORRE EG
    [-10,-10,-10,-10,-10,-10,-10,-10,
       0,  0,  0,  0,  0,  0,  0,  0,
       0,  0,  0,  0,  0,  0,  0,  0,
       0,  0,  0,  0,  0,  0,  0,  0,
       0,  0,  0,  0,  0,  0,  0,  0,
       0,  0,  0,  0,  0,  0,  0,  0,
       0,  0,  0,  0,  0,  0,  0,  0,
     -10,-10,-10,-10,-10,-10,-10,-10],

    // REGINA EG
    [-20,-10,-10, -5, -5,-10,-10,-20,
     -10,  0,  0,  0,  0,  0,  0,-10,
     -10,  0,  5,  5,  5,  5,  0,-10,
      -5,  0,  5,  5,  5,  5,  0, -5,
       0,  0,  5,  5,  5,  5,  0, -5,
      -10, 5,  5,  5,  5,  5,  0,-10,
      -10, 0,  5,  0,  0,  0,  0,-10,
      -20,-10,-10, -5, -5,-10,-10,-20],

    // RE EG (Attivazione! Deve andare al centro)
    [-50,-40,-30,-20,-20,-30,-40,-50,
     -30,-20,-10,  0,  0,-10,-20,-30,
     -30,-10, 20, 30, 30, 20,-10,-30,
     -30,-10, 30, 40, 40, 30,-10,-30,
     -30,-10, 30, 40, 40, 30,-10,-30,
     -30,-10, 20, 30, 30, 20,-10,-30,
     -30,-30,  0,  0,  0,  0,-30,-30,
     -50,-30,-30,-30,-30,-30,-30,-50]
];

// Bonus esponenziale per pedoni passati nelle ultime traverse
const PASSED_PAWN_BONUS: [i32; 8] = [ 0, 10, 20, 40, 70, 120, 200, 0 ];

// =============================================================
// FUNZIONE PRINCIPALE DI VALUTAZIONE
// =============================================================
pub fn valuta_posizione(s: &Scacchiera) -> i32 {
    
    // ---------------------------------------------------------
    // 1. TENTATIVO NNUE (Priorità Assoluta)
    // ---------------------------------------------------------
    if let Some(net) = &*NNUE {
        // Se la rete è caricata e verificata, usala.
        return net.evaluate(s);
    }

    // ---------------------------------------------------------
    // 2. FALLBACK: VALUTAZIONE ARTIGIANALE (PeSTO + Bonus)
    // ---------------------------------------------------------
    let mut mg = [0, 0];
    let mut eg = [0, 0];
    let mut phase = 0;

    let white_pawns = s.bitboard_pezzo_colore(Pezzo::Pedone, Colore::Bianco);
    let black_pawns = s.bitboard_pezzo_colore(Pezzo::Pedone, Colore::Nero);

    for sq in 0..64 {
        if let Some((p, c)) = s.pezzo_su_casella(Casella(sq)) {
            let pi = p.indice();
            let ci = c.indice();
            
            // Calcolo Fase (24 = apertura, 0 = finale)
            phase += match p {
                Pezzo::Cavallo | Pezzo::Alfiere => 1,
                Pezzo::Torre => 2,
                Pezzo::Regina => 4,
                _ => 0,
            };

            // Indice tabella (Flip per il bianco se le tabelle sono white-relative)
            // Assumiamo tabelle definite "dal punto di vista del bianco" (A1 in basso)
            // Bianco: A1 -> indice normale. Nero: A8 -> A1
            // Nota: Adattato al tuo motore che usa sq^56 per il bianco
            let pst_idx = if c == Colore::Bianco { sq ^ 56 } else { sq };
            
            mg[ci] += MG_VAL[pi] + MG_TABLE[pi][pst_idx];
            eg[ci] += EG_VAL[pi] + EG_TABLE[pi][pst_idx];

            // --- BONUS EXTRA ---
            
            // 1. Pedoni Passati
            if p == Pezzo::Pedone {
                if is_passed_pawn(sq, c, white_pawns, black_pawns) {
                    let rank = if c == Colore::Bianco { sq / 8 } else { 7 - (sq / 8) };
                    mg[ci] += PASSED_PAWN_BONUS[rank]; 
                    eg[ci] += PASSED_PAWN_BONUS[rank]; 
                }
            }

            // 2. Controllo Centrale (Per evitare passività)
            // Se un pezzo (non pedone) è al centro (file 3,4 - rank 3,4)
            if p != Pezzo::Pedone {
                let file = sq % 8;
                let rank = sq / 8;
                if file >= 3 && file <= 4 && rank >= 3 && rank <= 4 {
                    mg[ci] += 15;
                    eg[ci] += 10;
                }
            }
        }
    }

    // --- SICUREZZA RE ---
    mg[0] += valuta_sicurezza_re(s, Colore::Bianco);
    mg[1] += valuta_sicurezza_re(s, Colore::Nero);

    // --- TAPERED EVALUATION (Interpolazione) ---
    let p = phase.min(24);
    let mg_score = mg[0] - mg[1];
    let eg_score = eg[0] - eg[1];
    
    // Formula standard: (MG * phase + EG * (24 - phase)) / 24
    let score = (mg_score * p + eg_score * (24 - p)) / 24;

    // Ritorna score dal punto di vista di chi muove
    if s.turno() == Colore::Bianco { score } else { -score }
}

// =============================================================
// HELPER FUNCTIONS
// =============================================================

fn valuta_sicurezza_re(s: &Scacchiera, c: Colore) -> i32 {
    let king_bb = s.bitboard_pezzo_colore(Pezzo::Re, c);
    if king_bb == 0 { return 0; }
    
    let k_sq = king_bb.trailing_zeros() as usize;
    let mut penalty = 0;
    
    // Penalità grave se il Re è aperto (senza pedoni amici vicini)
    // Controlliamo un raggio semplice
    let nearby_pawns = crate::attacks::king_attacks(k_sq) & s.bitboard_pezzo_colore(Pezzo::Pedone, c);
    if nearby_pawns == 0 {
         penalty -= 40; // Re nudo!
    }

    // Penalità arrocco rovinato (scudo pedonale mancante)
    let is_kingside = if c == Colore::Bianco { k_sq == 6 || k_sq == 7 } else { k_sq == 62 || k_sq == 63 };
    let is_queenside = if c == Colore::Bianco { k_sq == 1 || k_sq == 2 } else { k_sq == 57 || k_sq == 58 };

    if is_kingside {
        // Controlla f, g, h
        let (f, g, h) = if c == Colore::Bianco { (13, 14, 15) } else { (53, 54, 55) };
        if (s.bitboard_pezzo_colore(Pezzo::Pedone, c) & ((1<<f)|(1<<g)|(1<<h))) == 0 { penalty -= 30; }
    } else if is_queenside {
        // Controlla a, b, c
        let (a, b, c_sq) = if c == Colore::Bianco { (8, 9, 10) } else { (48, 49, 50) };
        if (s.bitboard_pezzo_colore(Pezzo::Pedone, c) & ((1<<a)|(1<<b)|(1<<c_sq))) == 0 { penalty -= 30; }
    }

    penalty
}

fn is_passed_pawn(sq: usize, c: Colore, white_pawns: u64, black_pawns: u64) -> bool {
    let file = sq % 8;
    let rank = sq / 8;
    let enemy_pawns = if c == Colore::Bianco { black_pawns } else { white_pawns };
    let mut mask = 0u64;
    
    if c == Colore::Bianco {
        // Controlla righe avanti sulle colonne adiacenti e stessa colonna
        for r in (rank + 1)..8 {
            mask |= 1u64 << (r * 8 + file);
            if file > 0 { mask |= 1u64 << (r * 8 + (file - 1)); }
            if file < 7 { mask |= 1u64 << (r * 8 + (file + 1)); }
        }
    } else {
        // Controlla righe dietro (verso rank 0)
        for r in 0..rank {
            mask |= 1u64 << (r * 8 + file);
            if file > 0 { mask |= 1u64 << (r * 8 + (file - 1)); }
            if file < 7 { mask |= 1u64 << (r * 8 + (file + 1)); }
        }
    }
    (enemy_pawns & mask) == 0
}