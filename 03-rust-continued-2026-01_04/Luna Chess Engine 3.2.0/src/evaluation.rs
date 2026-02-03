use crate::board::{Scacchiera, Colore, Pezzo};
use crate::nnue::LunaNNUE;

// Valori materiale classici per fallback
const MG_PAWN: i32 = 100;
const MG_KNIGHT: i32 = 320;
const MG_BISHOP: i32 = 330;
const MG_ROOK: i32 = 500;
const MG_QUEEN: i32 = 900;

pub fn evaluate(board: &Scacchiera, nnue: Option<&LunaNNUE>) -> i32 {
    // 1. Calcolo punteggio base (NNUE o Fallback)
    let mut score = if let Some(net) = nnue {
        net.evaluate(board)
    } else {
        evaluate_hce(board)
    };

    // 2. Logica Mop-Up (Per i finali vinti)
    // Si attiva solo se il punteggio indica un vantaggio netto (> 2 pedoni)
    // Assumiamo che 'score' sia relativo al giocatore di turno (Negamax).
    if score > 200 {
        let us = board.turno;
        let them = us.opposto();
        
        // Controlliamo se l'avversario ha ancora pezzi pesanti
        let enemy_material = conta_materiale_pesante(board, them);
        
        // Se l'avversario è "nudo" (solo Re e pedoni), attiviamo il mop-up
        // per spingere il Re all'angolo e chiudere la partita.
        if enemy_material == 0 {
            let bonus = mop_up_eval(board, us, them);
            score += bonus;
        }
    }

    score
}

fn evaluate_hce(board: &Scacchiera) -> i32 {
    let mut score = 0;
    let us = board.turno;

    for sq in 0..64 {
        if let Some((colore, pezzo)) = board.pezzo_e_colore_in(sq) {
            let val = match pezzo {
                Pezzo::Pedone => MG_PAWN,
                Pezzo::Cavallo => MG_KNIGHT,
                Pezzo::Alfiere => MG_BISHOP,
                Pezzo::Torre => MG_ROOK,
                Pezzo::Regina => MG_QUEEN,
                Pezzo::Re => 0,
            };
            if colore == Colore::Bianco { score += val; } else { score -= val; }
        }
    }

    // Restituisce il punteggio relativo al giocatore di turno (Negamax)
    if us == Colore::Bianco { score } else { -score }
}

// --- Funzioni Ausiliarie per i Finali (Mop-Up) ---

fn mop_up_eval(board: &Scacchiera, us: Colore, them: Colore) -> i32 {
    let my_king_sq = trova_re(board, us);
    let enemy_king_sq = trova_re(board, them);

    // Se per assurdo non troviamo un re, ritorniamo 0
    if my_king_sq == 64 || enemy_king_sq == 64 { return 0; }

    let mut score = 0;

    // 1. Spingere il Re nemico ai bordi/angoli (Manhattan distance dal centro)
    let enemy_rank = (enemy_king_sq / 8) as i32;
    let enemy_file = (enemy_king_sq % 8) as i32;
    
    // Il centro è 3.5. Moltiplichiamo per 2 per lavorare con interi:
    // Distanza = abs(2*coord - 7). Risultato 0 (centro) -> 7 (bordo).
    let center_dist = (2 * enemy_rank - 7).abs() + (2 * enemy_file - 7).abs();
    
    // Più il re nemico è lontano dal centro, più alto è il bonus.
    score += center_dist * 10; 

    // 2. Avvicinare il nostro Re al Re nemico (ridurre la distanza)
    let my_rank = (my_king_sq / 8) as i32;
    let my_file = (my_king_sq % 8) as i32;

    let dist_kings = (my_rank - enemy_rank).abs() + (my_file - enemy_file).abs();
    
    // Bonus inversamente proporzionale alla distanza (più siamo vicini, meglio è)
    // 14 è la distanza massima teorica sulla scacchiera.
    score += (14 - dist_kings) * 4; 

    score
}

fn conta_materiale_pesante(board: &Scacchiera, colore: Colore) -> i32 {
    let mut mat = 0;
    // Iteriamo sulla scacchiera per compatibilità con la tua struttura
    for sq in 0..64 {
        if let Some((c, pezzo)) = board.pezzo_e_colore_in(sq) {
            if c == colore {
                match pezzo {
                    Pezzo::Cavallo => mat += MG_KNIGHT,
                    Pezzo::Alfiere => mat += MG_BISHOP,
                    Pezzo::Torre => mat += MG_ROOK,
                    Pezzo::Regina => mat += MG_QUEEN,
                    _ => {} // Ignora Pedone e Re
                }
            }
        }
    }
    mat
}

fn trova_re(board: &Scacchiera, colore: Colore) -> usize {
    for sq in 0..64 {
        if let Some((c, pezzo)) = board.pezzo_e_colore_in(sq) {
            // Usiamo un match per essere sicuri del tipo Pezzo::Re
            if c == colore {
                if let Pezzo::Re = pezzo {
                    return sq;
                }
            }
        }
    }
    64 // Fallback (non dovrebbe accadere)
}