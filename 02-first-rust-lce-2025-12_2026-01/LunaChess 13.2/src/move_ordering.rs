use crate::board::{Scacchiera, Mossa, Pezzo};

pub fn punteggio_mossa(s: &Scacchiera, m: &Mossa, tt_move: Option<Mossa>) -> i32 {
    // 1. Priorità massima alla mossa della Transposition Table
    if let Some(tm) = tt_move {
        if m.da() == tm.da() && m.a() == tm.a() { return 100000; }
    }

    let mut score = 0;
    let from = m.da();
    let to = m.a();

    // Recupero sicuro del pezzo: se per qualche motivo la casa è vuota, diamo punteggio 0
    let piece = match s.get_piece_at(from) {
        Some(p) => p,
        None => return 0,
    };
    
    let cap = s.get_piece_at(to);

    // 2. MVV-LVA (Cattura del pezzo più prezioso con quello meno prezioso)
    if let Some(captured) = cap {
        score += 1000 * (valore_pezzo(captured) + 10) - valore_pezzo(piece);
    }

    // 3. Bonus per la promozione
    if m.flag() == 6 { score += 500; }

    // 4. Bonus centralità (per evitare aperture laterali)
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