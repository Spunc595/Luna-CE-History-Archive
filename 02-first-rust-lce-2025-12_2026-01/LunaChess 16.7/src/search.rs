use crate::board::{Scacchiera, Mossa, Pezzo, MoveFlag};
use crate::evaluation;
use std::time::Instant;

// Valori per l'infinito e il matto. 
// MATE_VALUE è leggermente inferiore a INFINITY per gestire la distanza dal matto (mate in 1 > mate in 5).
pub const INFINITY: i32 = 50000;
pub const MATE_VALUE: i32 = 49000;

pub struct SearchInfo {
    pub start_time: Instant,
    pub time_limit_ms: u128,
    pub nodes: u64,
    pub stop: bool,
}

// --- RICERCA PRINCIPALE (Alpha-Beta Negamax) ---
pub fn search(board: &mut Scacchiera, mut depth: i32, mut alpha: i32, beta: i32, info: &mut SearchInfo) -> i32 {
    // 1. Controllo Tempo ogni 2048 nodi per non rallentare troppo
    if info.nodes & 2047 == 0 {
        if info.start_time.elapsed().as_millis() > info.time_limit_ms {
            info.stop = true;
        }
    }
    if info.stop { return 0; }

    info.nodes += 1;

    // 2. Check Extension: Se il re è sotto scacco, cerchiamo più a fondo
    // Questo è fondamentale per non perdere sequenze di matto forzato.
    let in_check = board.re_in_scacco(board.turno);
    if in_check {
        depth += 1;
    }

    // 3. Se siamo arrivati alla fine della profondità, passiamo alla Quiescence Search
    // invece di valutare staticamente (evita l'Horizon Effect).
    if depth <= 0 {
        return quiescence(board, alpha, beta, info);
    }

    let mut moves = board.genera_mosse();
    
    // 4. Move Ordering: Ordiniamo le mosse per analizzare prima le più forti.
    moves.sort_by_cached_key(|m| -score_move(board, m));

    let mut legal_moves_count = 0;
    let mut best_score = -INFINITY;

    for mv in moves {
        // Proviamo la mossa
        if board.esegui_mossa(&mv, None) {
            legal_moves_count += 1;
            
            // Ricorsione: notare il segno meno e lo scambio di alpha/beta
            let score = -search(board, depth - 1, -beta, -alpha, info);
            
            // Torniamo indietro
            board.annulla_mossa(&mv, None, None);

            if info.stop { return 0; }

            if score > best_score {
                best_score = score;
            }

            // Alpha-Beta Pruning
            if score > alpha {
                alpha = score;
                if score >= beta {
                    return beta; // Cut-off
                }
            }
        }
    }

    // 5. Gestione fine partita: Scacco Matto o Stallo
    if legal_moves_count == 0 {
        if in_check {
            // È matto. Aggiungiamo la profondità per preferire matti più veloci.
            return -MATE_VALUE + (info.nodes as i32 % 100); 
        } else {
            // È stallo
            return 0; 
        }
    }

    alpha
}

// --- QUIESCENCE SEARCH (Ricerca di Quiete) ---
// Continua a cercare finché ci sono catture, per evitare di valutare posizioni instabili.
pub fn quiescence(board: &mut Scacchiera, mut alpha: i32, beta: i32, info: &mut SearchInfo) -> i32 {
    info.nodes += 1;
    
    if info.nodes & 2047 == 0 {
        if info.start_time.elapsed().as_millis() > info.time_limit_ms {
            info.stop = true;
        }
    }
    if info.stop { return 0; }

    // 1. Stand-Pat: Valutazione statica corrente.
    // Se siamo già sopra Beta senza fare nulla, tagliamo (assumiamo che non siamo costretti a peggiorare).
    let stand_pat = evaluation::evaluate_classical(board);

    if stand_pat >= beta {
        return beta;
    }
    if stand_pat > alpha {
        alpha = stand_pat;
    }

    // 2. Generiamo e filtriamo le mosse: Solo CATTURE e PROMOZIONI
    let mut moves = board.genera_mosse();
    let mut captures = Vec::new();
    
    for m in moves {
        let flag = m.move_flag();
        // Qui abbiamo corretto l'errore: controlliamo Capture, EnPassant o se è una promozione generica
        if flag == MoveFlag::Capture || 
           flag == MoveFlag::EnPassant || 
           m.is_promotion() { 
               captures.push(m);
           }
    }

    // Fondamentale: ordiniamo le catture (es. PxQ prima di QxP)
    captures.sort_by_cached_key(|m| -score_move(board, m));

    for mv in captures {
        if board.esegui_mossa(&mv, None) {
            let score = -quiescence(board, -beta, -alpha, info);
            board.annulla_mossa(&mv, None, None);

            if info.stop { return 0; }

            if score >= beta {
                return beta;
            }
            if score > alpha {
                alpha = score;
            }
        }
    }

    alpha
}

// --- MOVE ORDERING (MVV-LVA) ---
// Assegna un punteggio alla mossa per l'ordinamento. Più alto è meglio.
pub fn score_move(board: &Scacchiera, mv: &Mossa) -> i32 {
    let to = mv.a();
    let from = mv.da();
    let move_flag = mv.move_flag();
    
    // Identifichiamo chi attacca
    let attacker = board.get_piece_at(from).unwrap_or(Pezzo::Pedone);
    
    // Identifichiamo la vittima (se c'è)
    let victim = if move_flag == MoveFlag::EnPassant {
        Some(Pezzo::Pedone)
    } else {
        board.get_piece_at(to)
    };

    let mut score = 0;

    // MVV-LVA: Most Valuable Victim - Least Valuable Attacker
    if let Some(v) = victim {
        let victim_val = piece_value(v);
        let attacker_val = piece_value(attacker);
        
        // Esempio: Pedone(1) mangia Donna(9) -> 10000 + 90 - 1 = 10089 (Altissimo)
        // Esempio: Donna(9) mangia Pedone(1) -> 10000 + 10 - 9 = 10001 (Basso ma prioritario rispetto a mosse quiete)
        score = 10000 + (victim_val * 10) - attacker_val;
    }

    // Bonus Promozioni
    if mv.is_promotion() {
        score += 8000; 
    }

    // Bonus Arrocco (sviluppo sicuro)
    if move_flag == MoveFlag::Castle {
        score += 1000;
    }
    
    // Penalità per muovere il Re troppo presto (se non è arrocco)
    if attacker == Pezzo::Re && move_flag != MoveFlag::Castle {
        score -= 50;
    }

    score
}

// Helper per ottenere valori numerici grezzi per l'ordinamento (non per la valutazione posizionale)
fn piece_value(p: Pezzo) -> i32 {
    match p {
        Pezzo::Pedone => 1,
        Pezzo::Cavallo => 3,
        Pezzo::Alfiere => 3,
        Pezzo::Torre => 5,
        Pezzo::Regina => 9,
        Pezzo::Re => 100, // Il Re non viene mai mangiato, ma serve un valore
    }
}

// Wrapper per compatibilità se uci.rs lo chiama
pub fn punteggio_mossa(board: &Scacchiera, mv: &Mossa) -> i32 {
    score_move(board, mv)
}