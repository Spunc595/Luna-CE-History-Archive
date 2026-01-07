use crate::board::{Scacchiera, Colore, Pezzo, Bitboard};

// --- PESI MATERIALI (Centipawns) ---
// Nota: Alfiere e Cavallo valgono molto più di un pedone.
// Questo impedisce al motore di sacrificarli inutilmente.
const MATERIAL_VAL: [i32; 6] = [
    100,  // Pedone
    320,  // Cavallo
    330,  // Alfiere
    500,  // Torre
    900,  // Regina
    20000 // Re (Valore infinito per evitare calcoli errati)
];

// --- PIECE-SQUARE TABLES (PST) ---
// Bonus per posizionare i pezzi in case attive.
// I valori sono per il BIANCO. Per il Nero, "flippiamo" la scacchiera.

// Pedoni: Vogliono avanzare, ma proteggere il re.
#[rustfmt::skip]
const PST_PAWN: [i32; 64] = [
     0,  0,  0,  0,  0,  0,  0,  0,
    50, 50, 50, 50, 50, 50, 50, 50,
    10, 10, 20, 30, 30, 20, 10, 10,
     5,  5, 10, 25, 25, 10,  5,  5,
     0,  0,  0, 20, 20,  0,  0,  0,
     5, -5,-10,  0,  0,-10, -5,  5,
     5, 10, 10,-20,-20, 10, 10,  5,
     0,  0,  0,  0,  0,  0,  0,  0
];

// Cavalli: Amano il centro, odiano i bordi.
#[rustfmt::skip]
const PST_KNIGHT: [i32; 64] = [
    -50,-40,-30,-30,-30,-30,-40,-50,
    -40,-20,  0,  0,  0,  0,-20,-40,
    -30,  0, 10, 15, 15, 10,  0,-30,
    -30,  5, 15, 20, 20, 15,  5,-30,
    -30,  0, 15, 20, 20, 15,  0,-30,
    -30,  5, 10, 15, 15, 10,  5,-30,
    -40,-20,  0,  5,  5,  0,-20,-40,
    -50,-40,-30,-30,-30,-30,-40,-50
];

// Alfieri: Amano le diagonali lunghe e il centro.
#[rustfmt::skip]
const PST_BISHOP: [i32; 64] = [
    -20,-10,-10,-10,-10,-10,-10,-20,
    -10,  0,  0,  0,  0,  0,  0,-10,
    -10,  0,  5, 10, 10,  5,  0,-10,
    -10,  5,  5, 10, 10,  5,  5,-10,
    -10,  0, 10, 10, 10, 10,  0,-10,
    -10, 10, 10, 10, 10, 10, 10,-10,
    -10,  5,  0,  0,  0,  0,  5,-10,
    -20,-10,-10,-10,-10,-10,-10,-20
];

// Torri: Amano la 7a traversa e le colonne centrali.
#[rustfmt::skip]
const PST_ROOK: [i32; 64] = [
     0,  0,  0,  0,  0,  0,  0,  0,
     5, 10, 10, 10, 10, 10, 10,  5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
     0,  0,  0,  5,  5,  0,  0,  0
];

// Regine: Valori bassi, non vogliamo che escano troppo presto.
#[rustfmt::skip]
const PST_QUEEN: [i32; 64] = [
    -20,-10,-10, -5, -5,-10,-10,-20,
    -10,  0,  0,  0,  0,  0,  0,-10,
    -10,  0,  5,  5,  5,  5,  0,-10,
     -5,  0,  5,  5,  5,  5,  0, -5,
      0,  0,  5,  5,  5,  5,  0, -5,
    -10,  5,  5,  5,  5,  5,  0,-10,
    -10,  0,  5,  0,  0,  0,  0,-10,
    -20,-10,-10, -5, -5,-10,-10,-20
];

// Re (Mediogioco): Deve stare al sicuro (arroccato).
#[rustfmt::skip]
const PST_KING_MG: [i32; 64] = [
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -20,-30,-30,-40,-40,-30,-30,-20,
    -10,-20,-20,-20,-20,-20,-20,-10,
     20, 20,  0,  0,  0,  0, 20, 20,
     20, 30, 10,  0,  0, 10, 30, 20
];

// Funzione principale di Valutazione
pub fn evaluate_classical(board: &Scacchiera) -> i32 {
    let mut score_mg = 0; // Punteggio Mediogioco
    // let mut score_eg = 0; // Punteggio Finale (non usato per semplicità ora)

    // Per ogni tipo di pezzo (eccetto Re, gestito a parte per sicurezza)
    for p in 0..5 {
        let piece = match p {
            0 => Pezzo::Pedone, 1 => Pezzo::Cavallo, 2 => Pezzo::Alfiere,
            3 => Pezzo::Torre, 4 => Pezzo::Regina, _ => unreachable!()
        };
        
        let val = MATERIAL_VAL[p];
        let pst = get_pst(piece);

        // Pezzi Bianchi
        let mut w_bb = board.pezzi[p] & board.colori[Colore::Bianco.indice()];
        while w_bb != 0 {
            let sq = w_bb.trailing_zeros() as usize;
            score_mg += val + pst[sq ^ 56]; // ^56 flippa verticalmente per il bianco se le tabelle sono orientate dall'alto
            // Nota: Le tabelle sopra sono scritte visivamente (0..7 = a8..h8 o a1..h1?)
            // Standard UCI: 0=a1, 63=h8.
            // Le tabelle sopra sono scritte "da codice": riga 0 (index 0-7) è la traversa 1.
            // Quindi per il bianco usiamo l'indice diretto, per il nero flippiamo.
            // FIX: Guardando le tabelle, la prima riga (0-7) ha valori bassi/negativi per i pedoni.
            // Questo suggerisce che indice 0 = A1. 
            // Quindi:
            // Bianco: index diretto.
            // Nero: index ^ 56 (flip verticale).
            
            // CORREZIONE TABELLE:
            // Le tabelle sopra sono scritte visivamente dall'alto (riga 8) al basso (riga 1)
            // se lette sequenzialmente.
            // Se 0=A1, allora la prima riga dell'array (0..7) corrisponde a A1..H1.
            // Nella mia PST_PAWN, la prima riga è [0,0...]. I pedoni sono in 2a traversa.
            // Quindi la prima riga dell'array è la Traversa 1.
            // Quindi per il Bianco usiamo `score_mg += val + pst[sq]`.
            
            score_mg += val + pst[sq];
            w_bb &= w_bb - 1;
        }

        // Pezzi Neri
        let mut b_bb = board.pezzi[p] & board.colori[Colore::Nero.indice()];
        while b_bb != 0 {
            let sq = b_bb.trailing_zeros() as usize;
            // Per il nero flippiamo verticalmente la casa: sq ^ 56
            score_mg -= val + pst[sq ^ 56]; 
            b_bb &= b_bb - 1;
        }
    }

    // Re (gestione separata per usare PST specifiche)
    let w_king = (board.pezzi[Pezzo::Re.indice()] & board.colori[Colore::Bianco.indice()]).trailing_zeros() as usize;
    let b_king = (board.pezzi[Pezzo::Re.indice()] & board.colori[Colore::Nero.indice()]).trailing_zeros() as usize;

    score_mg += PST_KING_MG[w_king];
    score_mg -= PST_KING_MG[b_king ^ 56];

    // Ritorna il punteggio relativo al giocatore che deve muovere
    if board.turno == Colore::Bianco {
        score_mg
    } else {
        -score_mg
    }
}

fn get_pst(p: Pezzo) -> &'static [i32; 64] {
    match p {
        Pezzo::Pedone => &PST_PAWN,
        Pezzo::Cavallo => &PST_KNIGHT,
        Pezzo::Alfiere => &PST_BISHOP,
        Pezzo::Torre => &PST_ROOK,
        Pezzo::Regina => &PST_QUEEN,
        Pezzo::Re => &PST_KING_MG,
    }
}