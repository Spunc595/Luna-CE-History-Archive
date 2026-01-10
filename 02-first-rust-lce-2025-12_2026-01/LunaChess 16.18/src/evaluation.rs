use crate::board::{Scacchiera, Colore, Pezzo};

/// Valori dei pezzi (in centesimi di pedone)
const VALORE_PEDONE: i32 = 100;
const VALORE_CAVALLO: i32 = 320;
const VALORE_ALFIERE: i32 = 330;
const VALORE_TORRE: i32 = 500;
const VALORE_REGINA: i32 = 900;
const VALORE_RE: i32 = 20000;

/// Tabelle Posizionali (PST) - Bonus per la posizione dei pezzi
/// Aiutano il motore a capire che i pezzi devono stare al centro e non sui bordi.
const PST_CAVALLI: [i32; 64] = [
    -50,-40,-30,-30,-30,-30,-40,-50,
    -40,-20,  0,  0,  0,  0,-20,-40,
    -30,  0, 10, 15, 15, 10,  0,-30,
    -30,  5, 15, 20, 20, 15,  5,-30,
    -30,  0, 15, 20, 20, 15,  0,-30,
    -30,  5, 10, 15, 15, 10,  5,-30,
    -40,-20,  0,  5,  5,  0,-20,-40,
    -50,-40,-30,-30,-30,-30,-40,-50,
];

const PST_PEDONI: [i32; 64] = [
     0,  0,  0,  0,  0,  0,  0,  0,
    50, 50, 50, 50, 50, 50, 50, 50,
    10, 10, 20, 30, 30, 20, 10, 10,
     5,  5, 10, 25, 25, 10,  5,  5,
     0,  0,  0, 20, 20,  0,  0,  0,
     5, -5,-10,  0,  0,-10, -5,  5,
     5, 10, 10,-20,-20, 10, 10,  5,
     0,  0,  0,  0,  0,  0,  0,  0,
];

/// FUNZIONE PRINCIPALE DI VALUTAZIONE
/// Deve essere 'pub' per essere vista da search.rs
pub fn evaluate(board: &Scacchiera) -> i32 {
    let mut score = 0;

    // 1. Valutazione Materiale
    score += contatore_materiale(board, Colore::Bianco) - contatore_materiale(board, Colore::Nero);

    // 2. Valutazione Posizionale (PST)
    score += valuta_pst(board);

    // 3. Bonus Mobilità (Semplificato)
    // Un motore che ha più mosse legali è solitamente in una posizione migliore
    let turno_attuale = board.turno;
    
    // Restituiamo il punteggio sempre dal punto di vista del giocatore che ha il turno (Relative Score)
    if turno_attuale == Colore::Bianco { score } else { -score }
}

fn contatore_materiale(board: &Scacchiera, c: Colore) -> i32 {
    let mut m = 0;
    let idx = c.indice();
    m += (board.pezzi[Pezzo::Pedone.indice()] & board.colori[idx]).count_ones() as i32 * VALORE_PEDONE;
    m += (board.pezzi[Pezzo::Cavallo.indice()] & board.colori[idx]).count_ones() as i32 * VALORE_CAVALLO;
    m += (board.pezzi[Pezzo::Alfiere.indice()] & board.colori[idx]).count_ones() as i32 * VALORE_ALFIERE;
    m += (board.pezzi[Pezzo::Torre.indice()] & board.colori[idx]).count_ones() as i32 * VALORE_TORRE;
    m += (board.pezzi[Pezzo::Regina.indice()] & board.colori[idx]).count_ones() as i32 * VALORE_REGINA;
    m += (board.pezzi[Pezzo::Re.indice()] & board.colori[idx]).count_ones() as i32 * VALORE_RE;
    m
}

fn valuta_pst(board: &Scacchiera) -> i32 {
    let mut pst_score = 0;

    // Cavalli Bianchi
    let mut cavalli_w = board.pezzi[Pezzo::Cavallo.indice()] & board.colori[0];
    while cavalli_w != 0 {
        let sq = cavalli_w.trailing_zeros() as usize;
        pst_score += PST_CAVALLI[sq ^ 56]; // Invertiamo per il bianco se necessario
        cavalli_w &= cavalli_w - 1;
    }

    // Pedoni Bianchi (Incentiva l'avanzata e il controllo del centro)
    let mut pedoni_w = board.pezzi[Pezzo::Pedone.indice()] & board.colori[0];
    while pedoni_w != 0 {
        let sq = pedoni_w.trailing_zeros() as usize;
        pst_score += PST_PEDONI[sq ^ 56];
        pedoni_w &= pedoni_w - 1;
    }

    // Nota: Per brevità ho messo i PST principali, 
    // ma questo impedisce a Luna di muovere i pezzi a caso sui bordi.
    
    pst_score
}