use crate::board::{Scacchiera, Mossa, Pezzo};

pub fn punteggio_mossa(s: &Scacchiera, m: &Mossa, tt_move: Option<Mossa>) -> i32 {
    // 1. Mossa della Transposition Table (massima priorità)
    if let Some(tm) = tt_move {
        if m.da() == tm.da() && m.a() == tm.a() { return 100000; }
    }

    let mut score = 0;
    let from = m.da();
    let to = m.a();
    let piece = s.get_piece_at(from).unwrap();
    let cap = s.get_piece_at(to);

    // 2. MVV-LVA (Most Valuable Victim - Least Valuable Attacker)
    // Premia la cattura di un pezzo grande con un pezzo piccolo
    if let Some(captured) = cap {
        score += 1000 * (valore_pezzo(captured) + 10) - valore_pezzo(piece);
    }

    // 3. Premia mosse centrali nei primi stadi (per evitare aperture assurde)
    let centralita = [
        0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 10, 20, 20, 10, 0, 0,
        0, 0, 20, 30, 30, 20, 0, 0,
        0, 0, 20, 30, 30, 20, 0, 0,
        0, 0, 10, 20, 20, 10, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0,
    ];
    score += centralita[to];

    score
}

fn valore_pezzo(p: Pezzo) -> i32 {
    match p {
        Pezzo::Pedone => 1,
        Pezzo::Cavallo => 3,
        Pezzo::Alfiere => 3,
        Pezzo::Torre => 5,
        Pezzo::Regina => 9,
        Pezzo::Re => 100,
    }
}