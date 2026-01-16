use crate::board::{Scacchiera, Mossa};
use rand::Rng;

pub fn get_book_move(board: &Scacchiera) -> Option<Mossa> {
    // Generiamo una "firma" visiva della scacchiera (FEN semplificato)
    let fen = get_simple_fen(board);
    
    // Lista delle aperture (Key = FEN, Value = Lista Mosse)
    let candidates: &[&str] = match fen.as_str() {
        // POSIZIONE INIZIALE
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq" => 
            &["e2e4", "d2d4", "c2c4", "g1f3"],

        // RISPOSTE A E4 (Nero)
        "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq" => 
            &["c7c5", "e7e5", "e7e6", "c7c6"], // Siciliana, Aperta, Francese, Caro-Kann

        // RISPOSTE A D4 (Nero)
        "rnbqkbnr/pppppppp/8/8/3P4/8/PPP1PPPP/RNBQKBNR b KQkq" => 
            &["g8f6", "d7d5", "e7e6", "c7c5"],

        // RISPOSTE A E4 E5 (Bianco)
        "rnbqkbnr/pppp1ppp/8/4p3/4P3/8/PPPP1PPP/RNBQKBNR w KQkq" => 
            &["g1f3", "b1c3", "f1c4"], // Cavallo Re, Viennese, Alfiere

        // RISPOSTE A E4 C5 (Siciliana - Bianco)
        "rnbqkbnr/pp1ppppp/8/2p5/4P3/8/PPPP1PPP/RNBQKBNR w KQkq" => 
            &["g1f3", "b1c3", "c2c3"],

        // RISPOSTE A D4 D5 (Bianco)
        "rnbqkbnr/ppp1pppp/8/3p4/3P4/8/PPP1PPPP/RNBQKBNR w KQkq" => 
            &["c2c4", "g1f3", "c1f4"], // Gambetto Donna, Sistema London

        // Ruy Lopez / Italiana (dopo 1.e4 e5 2.Nf3 Nc6)
        "r1bqkbnr/pppp1ppp/2n5/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq" =>
            &["f1b5", "f1c4", "d2d4"],

        // Se non trovo nulla
        _ => return None, 
    };

    // Scelta casuale tra le mosse candidate
    let mut rng = rand::thread_rng();
    let pick = candidates[rng.gen_range(0..candidates.len())];
    
    // Trova la mossa reale corrispondente alla stringa
    let legali = board.genera_mosse();
    legali.into_iter().find(|m| m.to_uci() == pick)
}

// Funzione helper per creare una stringa univoca della posizione
fn get_simple_fen(board: &Scacchiera) -> String {
    let mut fen = String::new();
    for r in (0..8).rev() {
        let mut empty = 0;
        for f in 0..8 {
            let sq = r * 8 + f;
            let mut found = false;
            // Cerca Bianco
            for p in 0..6 {
                if (board.pezzi[p] & board.colori[0] & (1 << sq)) != 0 {
                    if empty > 0 { fen.push_str(&empty.to_string()); empty = 0; }
                    fen.push(["P","N","B","R","Q","K"][p].chars().next().unwrap());
                    found = true; break;
                }
            }
            // Cerca Nero
            if !found {
                for p in 0..6 {
                    if (board.pezzi[p] & board.colori[1] & (1 << sq)) != 0 {
                        if empty > 0 { fen.push_str(&empty.to_string()); empty = 0; }
                        fen.push(["p","n","b","r","q","k"][p].chars().next().unwrap());
                        found = true; break;
                    }
                }
            }
            if !found { empty += 1; }
        }
        if empty > 0 { fen.push_str(&empty.to_string()); }
        if r > 0 { fen.push('/'); }
    }
    
    fen.push(' ');
    fen.push(if board.turno == crate::board::Colore::Bianco { 'w' } else { 'b' });
    fen.push(' ');
    
    // Diritti arrocco semplificati
    let mut castling = String::new();
    // Nota: questo dipende da come board.diritti_arrocco è codificato.
    // Assumo standard bitmask: 1=WK, 2=WQ, 4=BK, 8=BQ. Se diverso, adatta qui.
    if board.diritti_arrocco & 1 != 0 { castling.push('K'); }
    if board.diritti_arrocco & 2 != 0 { castling.push('Q'); }
    if board.diritti_arrocco & 4 != 0 { castling.push('k'); }
    if board.diritti_arrocco & 8 != 0 { castling.push('q'); }
    if castling.is_empty() { castling.push('-'); }
    fen.push_str(&castling);
    
    fen
}