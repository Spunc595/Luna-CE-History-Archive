use crate::board::{Scacchiera, Mossa, Pezzo, MoveFlag};

// Valori approssimativi per MVV-LVA (Pedone, Cavallo, Alfiere, Torre, Regina, Re)
// Indici basati sul tuo enum Pezzo: 0=P, 1=N, 2=B, 3=R, 4=Q, 5=K
const PIECE_VALUES: [i32; 6] = [100, 300, 320, 500, 900, 20000];

pub fn score_move(board: &Scacchiera, mv: &Mossa) -> i32 {
    let mut score = 0;
    
    let from = mv.da();
    let to = mv.a();
    let flag = mv.flag();

    // 1. CATTURE (MVV-LVA)
    // Usiamo get_piece_at sulla casella di destinazione per vedere se mangiamo
    if let Some(victim) = board.get_piece_at(to) {
        let victim_value = PIECE_VALUES[victim.indice()];
        
        // Chi sta attaccando?
        if let Some(attacker) = board.get_piece_at(from) {
            let attacker_value = PIECE_VALUES[attacker.indice()];
            // Vittima preziosa mangiata da pezzo scarso = Punteggio alto
            score += (victim_value * 10) - attacker_value + 10000; 
        }
    }

    // 2. PROMOZIONI
    // Nel tuo Mossa::new, flag 6 è Promotion.
    // Attenzione: il tuo MoveFlag non specifica A COSA promuovi (di solito Regina).
    // Assumiamo Regina per dare priorità alta.
    if flag == 6 { // MoveFlag::Promotion
        score += 900 + 15000; 
    }

    // 3. EN PASSANT (Flag 5)
    if flag == 5 {
        score += 10500; // Simile a mangiare un pedone
    }

    score
}

pub fn sort_moves(board: &Scacchiera, moves: &mut Vec<Mossa>) {
    let mut scores: Vec<(i32, Mossa)> = moves.iter()
        .map(|mv| (score_move(board, mv), *mv))
        .collect();

    // Ordine decrescente
    scores.sort_by(|a, b| b.0.cmp(&a.0));

    *moves = scores.into_iter().map(|(_, mv)| mv).collect();
}