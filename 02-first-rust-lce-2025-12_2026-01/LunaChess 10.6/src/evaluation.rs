use crate::board::{Scacchiera, Pezzo, Colore, Casella, Bitboard};

const MG_VAL: [i32; 6] = [ 82, 337, 365, 477, 1025, 0 ];
const EG_VAL: [i32; 6] = [ 94, 281, 297, 512,  936, 0 ];

// --- TABELLE PESTO (Invariate per anti-h5 e anti-Queen) ---
#[rustfmt::skip]
const MG_TABLE: [[i32; 64]; 6] = [
    // PEDONE: Penalità per h5/a5 e bordi
    [  0,  0,  0,  0,  0,  0,  0,  0,
      98, 134, 61, 95, 68, 126, 34, -11,
      -6,  7, 26, 31, 65, 56, 25, -20,
     -40, 13,  6, 21, 23, 12, 17, -40, 
     -40, -2, -5, 12, 17,  6, 10, -40,
     -26, -4, -4,-10,  3,  3, 33, -12,
     -35, -1,-20,-23,-15, 24, 38, -22,
       0,  0,  0,  0,  0,  0,  0,  0],
    
    // CAVALLO
    [-167,-89,-34,-49, 61,-97,-15,-107, -73,-41, 72, 36, 23, 62, 7,-17, -47, 60, 37, 65, 84, 129, 73, 44, -9, 17, 19, 53, 37, 69, 18, 22, -13, 4, 16, 13, 28, 19, 21, -8, -23, -9, 12, 10, 19, 17, 25, -16, -29, -53, -12, -3, -1, 18, -14, -19, -105, -21, -58, -33, -17, -28, -19, -23],
    
    // ALFIERE
    [-29, 4, -82, -37, -25, -42, 7, -8, -26, 16, -18, -13, 30, 59, 18, -47, -16, 37, 43, 40, 35, 50, 37, -2, -4, 5, 19, 50, 37, 37, 7, -2, -6, 13, 13, 26, 34, 12, 10, 4, 0, 15, 15, 15, 14, 27, 18, 10, 4, 15, 16, 0, 7, 21, 33, 1, -33, -3, -14, -21, -13, -12, -39, -21],
    
    // TORRE
    [32, 42, 32, 51, 63, 9, 31, 43, 27, 32, 58, 62, 80, 67, 26, 44, -5, 19, 26, 36, 17, 45, 61, 16, -24, -11, 7, 26, 24, 35, -8, -20, -36, -26, -12, -1, 9, -7, 6, -23, -45, -25, -16, -17, 3, 0, -5, -33, -44, -16, -20, -9, -1, 11, -6, -71, -19, -13, 1, 17, 16, 7, -37, -26],
    
    // REGINA: Penalità uscita precoce
    [-50, -40, -30, -20, -20, -30, -40, -50,
     -40, -20,   0,   0,   0,   0, -20, -40,
     -30,   0,  10,  10,  10,  10,   0, -30,
     -20,   0,  10,  15,  15,  10,   0, -20,
     -20,   0,  10,  15,  15,  10,   0, -20,
     -30,   5,  10,  10,  10,  10,   5, -30,
     -40, -20,   0,   5,   5,   0, -20, -40,
     -50, -40, -30, -20, -20, -30, -40, -50],

    // RE
    [-65, 23, 16, -15, -56, -34, 2, 13, 29, -1, -20, -7, -8, -4, -38, -29, -9, 24, 2, 16, -20, 6, 22, -22, -17, -20, -12, -27, -30, -25, -14, -36, -49, -1, -27, -39, -46, -44, -33, -51, -14, -14, -22, -46, -44, -30, -15, -27, 1, 7, -8, -64, -43, -16, 9, 8, -15, 36, 12, -54, 8, -28, 24, 14]
];

#[rustfmt::skip]
const EG_TABLE: [[i32; 64]; 6] = [
    [0,0,0,0,0,0,0,0, 178,173,158,134,147,132,165,187, 94,100,85,67,56,53,82,84, 32,24,13,5,-2,4,17,17, 13,9,-3,-7,-7,-8,3,-1, 4,7,-6,1,0,-5,-1,-8, 13,8,8,10,13,0,2,-7, 0,0,0,0,0,0,0,0],
    [-58,-38,-13,-28,-31,-27,-63,-99, -25,-8,-25,-2,-9,-25,-24,-52, -24,-20,10,9,-1,-9,-19,-41, -17,3,22,22,22,11,8,-18, -18,-6,16,25,16,17,4,-18, -23,-3,-1,15,10,-3,-20,-22, -42,-20,-10,-5,-2,-20,-23,-44, -29,-51,-23,-15,-22,-18,-50,-64],
    [-14,-21,-11,-8,-7,-9,-17,-24, -8,-4,7,-12,-3,-13,-4,-14, 2,-8,0,-1,-2,6,0,4, -3,9,12,9,14,10,3,2, -6,3,13,19,7,10,-3,-9, -12,-3,5,10,5,6,0,-7, -15,-10,-10,-5,-4,0,-8,-23, -23,-9,-23,-5,-9,-16,-5,-17],
    [13,10,18,15,12,12,8,5, 11,13,13,11,-3,3,8,3, 7,7,7,5,4,-3,-5,-3, 4,3,13,1,2,1,-1,2, 3,5,8,4,-5,-6,-8,-11, -4,0,-5,-1,-7,-12,-8,-16, -6,-6,0,2,-9,-9,-11,-3, -9,2,3,-1,-5,-13,4,-20],
    [-9,22,22,27,27,19,10,20, -17,20,32,41,58,25,30,0, -20,6,9,49,47,35,19,9, 3,22,24,45,57,40,57,36, -18,28,19,47,31,34,39,23, -16,-27,15,6,9,17,10,5, -22,-23,-30,-16,-16,23,0,-36, -14,-5,-15,-10,-10,-10,-10,-2],
    [-74,-35,-18,-18,-11,15,4,-17, -12,17,14,17,17,38,23,11, 10,17,23,15,20,45,44,13, -8,22,24,27,26,33,26,3, -18,-4,21,24,27,23,9,-11, -19,-3,11,21,23,16,7,-9, -27,-11,4,13,14,4,-5,-17, -53,-34,-21,-11,-28,-14,-24,-43]
];

pub fn valuta_posizione(s: &Scacchiera) -> i32 {
    let mut mg = [0, 0];
    let mut eg = [0, 0];
    let mut phase = 0;

    for sq in 0..64 {
        if let Some((p, c)) = s.pezzo_su_casella(Casella(sq)) {
            let pi = p.indice();
            let ci = c.indice();
            
            phase += match p {
                Pezzo::Cavallo | Pezzo::Alfiere => 1,
                Pezzo::Torre => 2,
                Pezzo::Regina => 4,
                _ => 0,
            };

            let pst_idx = if c == Colore::Bianco { sq ^ 56 } else { sq };
            
            mg[ci] += MG_VAL[pi] + MG_TABLE[pi][pst_idx];
            eg[ci] += EG_VAL[pi] + EG_TABLE[pi][pst_idx];
        }
    }

    // --- KING SAFETY EVALUATION ---
    // Penalità se lo "scudo" di pedoni davanti al Re è rotto
    let mut king_safety = [0, 0];
    for c in [Colore::Bianco, Colore::Nero] {
        king_safety[c.indice()] = valuta_sicurezza_re(s, c);
    }

    // Aggiungiamo la sicurezza re al punteggio di mediogioco
    mg[0] += king_safety[0];
    mg[1] += king_safety[1];

    let p = phase.min(24);
    let mg_d = mg[0] - mg[1];
    let eg_d = eg[0] - eg[1];
    let score = (mg_d * p + eg_d * (24 - p)) / 24;

    if s.turno() == Colore::Bianco { score } else { -score }
}

fn valuta_sicurezza_re(s: &Scacchiera, c: Colore) -> i32 {
    let king_bb = s.bitboard_pezzo_colore(Pezzo::Re, c);
    if king_bb == 0 { return 0; }
    
    let k_sq = king_bb.trailing_zeros() as usize;
    let mut penalty = 0;
    
    // Controlliamo lo scudo solo se il Re è sulle righe di fondo (1 o 8)
    // e se è sui lati (Arrocco Corto o Lungo)
    let is_kingside = if c == Colore::Bianco { k_sq == 6 || k_sq == 7 } else { k_sq == 62 || k_sq == 63 };
    let is_queenside = if c == Colore::Bianco { k_sq == 1 || k_sq == 2 } else { k_sq == 57 || k_sq == 58 };

    if is_kingside {
        // Scudo ideale Re Bianco: F2, G2, H2. Re Nero: F7, G7, H7
        let (f, g, h) = if c == Colore::Bianco { (13, 14, 15) } else { (53, 54, 55) };
        penalty += valuta_scudo(s, c, f, g, h);
    } else if is_queenside {
        // Scudo ideale Re Bianco: A2, B2, C2. Re Nero: A7, B7, C7
        let (a, b, c_sq) = if c == Colore::Bianco { (8, 9, 10) } else { (48, 49, 50) };
        penalty += valuta_scudo(s, c, a, b, c_sq);
    }

    penalty
}

fn valuta_scudo(s: &Scacchiera, c: Colore, sq1: usize, sq2: usize, sq3: usize) -> i32 {
    let pawns = s.bitboard_pezzo_colore(Pezzo::Pedone, c);
    let mut penalty = 0;
    
    // Pedone "Centrale" dello scudo (es. G2 per arrocco corto bianco) - CRUCIALE
    if (pawns & (1u64 << sq2)) == 0 {
        penalty -= 50; // Penalità grave se manca il pedone davanti al Re
    }
    
    // Pedoni laterali (es. F2/H2)
    if (pawns & (1u64 << sq1)) == 0 { penalty -= 20; }
    if (pawns & (1u64 << sq3)) == 0 { penalty -= 20; }

    penalty
}