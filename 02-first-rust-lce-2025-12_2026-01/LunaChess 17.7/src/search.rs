use std::time::Instant;
use crate::board::{Scacchiera, Mossa, Colore, Pezzo, MoveFlag};
use crate::tt::{TranspositionTable, Bound};
use crate::zobrist::ZobristKeys;
use crate::evaluation::{evaluate, see, MG_VAL}; // Assicurati che MG_VAL sia pubblico in evaluation

pub const MATE_VALUE: i32 = 49000;
const ASPIRATION_WINDOW: i32 = 50; // Leggermente allargata per stabilità

pub struct SearchInfo {
    pub start_time: Instant,
    pub time_limit_ms: u128,
    pub nodes: u64,
    pub stop: bool,
}

#[derive(Clone, Copy)]
pub struct PVLine { pub moves: [Mossa; 64], pub len: usize }

pub struct KillerMoves { pub primary: [Mossa; 64], pub secondary: [Mossa; 64] }
impl KillerMoves {
    pub fn new() -> Self { Self { primary: [Mossa { data: 0 }; 64], secondary: [Mossa { data: 0 }; 64] } }
    pub fn update(&mut self, m: Mossa, ply: usize) {
        if ply < 64 && m != self.primary[ply] { self.secondary[ply] = self.primary[ply]; self.primary[ply] = m; }
    }
}

pub struct HistoryTable { pub scores: [[[i32; 64]; 64]; 2] }
impl HistoryTable {
    pub fn new() -> Self { Self { scores: [[[0; 64]; 64]; 2] } }
    pub fn update(&mut self, m: Mossa, c: Colore, d: i32) { 
        // Penalizza se troppo alto per evitare overflow, bonus proporzionale alla profondità
        let bonus = (d * d).min(400);
        self.scores[c.indice()][m.da()][m.a()] += bonus; 
    }
}

// --- ITERATIVE DEEPENING ---
pub fn iterative_deepening(board: &mut Scacchiera, info: &mut SearchInfo, tt: &mut TranspositionTable, z: &ZobristKeys) -> Mossa {
    let mut best_m = Mossa { data: 0 };
    let (mut killers, mut history) = (KillerMoves::new(), HistoryTable::new());
    let mut last_score = 0;

    // Se troviamo la mossa nel TT, usiamola come start
    if let Some(entry) = tt.probe(board.get_hash(z)) {
        best_m = Mossa { data: entry.move_data };
    }

    for depth in 1..65 {
        if info.stop { break; }
        
        let (mut alpha, mut beta) = (-MATE_VALUE, MATE_VALUE);
        
        // Aspiration Windows: riduce la ricerca se il punteggio è stabile
        if depth > 5 {
            alpha = last_score - ASPIRATION_WINDOW;
            beta = last_score + ASPIRATION_WINDOW;
        }

        loop {
            let (score, m, pv) = search_root(board, depth, alpha, beta, info, tt, z, &mut killers, &mut history);
            
            if info.stop { break; }

            // Se usciamo dalla finestra (fail low/high), ricalcoliamo con finestra completa
            if score <= alpha {
                alpha = -MATE_VALUE;
                continue; 
            } else if score >= beta {
                beta = MATE_VALUE;
                continue;
            }

            // Risultato valido
            last_score = score;
            best_m = m;
            
            let elapsed = info.start_time.elapsed().as_millis().max(1);
            let nps = (info.nodes as u128 * 1000) / elapsed;
            
            // Stampa info UCI
            print!("info depth {} score cp {} nodes {} nps {} time {} pv", depth, score, info.nodes, nps, elapsed);
            for i in 0..pv.len { 
                if pv.moves[i].data != 0 { print!(" {}", pv.moves[i]); } 
            }
            println!();
            
            break; // Depth completata
        }
    }
    best_m
}

fn search_root(board: &mut Scacchiera, depth: i32, mut alpha: i32, beta: i32, info: &mut SearchInfo, tt: &mut TranspositionTable, z: &ZobristKeys, km: &mut KillerMoves, ht: &mut HistoryTable) -> (i32, Mossa, PVLine) {
    let mut pv = PVLine { moves: [Mossa { data: 0 }; 64], len: 0 };
    let mut moves = board.genera_mosse();
    
    // Ordina: TT Move -> Captures -> Killer -> History
    moves.sort_by_key(|m| -punteggio_mossa(m, board, 0, km, ht, None));
    
    let mut best_m = if !moves.is_empty() { moves[0] } else { Mossa{data:0} };
    
    for (i, m) in moves.iter().enumerate() {
        if info.nodes % 2048 == 0 { check_time(info); }
        if info.stop { break; }

        let mut tmp = board.clone();
        if !tmp.esegui_mossa(m, None) { continue; }

        // PVS (Principal Variation Search) semplificata
        let score;
        if i == 0 {
            score = -search(&mut tmp, depth - 1, -beta, -alpha, 1, info, tt, z, km, ht);
        } else {
            // Cerca con finestra nulla (null window) per dimostrare che le altre mosse sono peggiori
            score = -search(&mut tmp, depth - 1, -alpha - 1, -alpha, 1, info, tt, z, km, ht);
            if score > alpha && score < beta {
                // Se fallisce, ricerca completa
                 // Ricalcola score qui se necessario, per ora PVS base
                 let full_score = -search(&mut tmp, depth - 1, -beta, -alpha, 1, info, tt, z, km, ht);
                 if full_score > alpha { alpha = full_score; best_m = *m; pv.moves[0] = *m; pv.len = 1; } // Fix rapido logica
            }
        }

        // Fix logica standard Alpha-Beta
        if i > 0 { // Re-search completa non implementata sopra per brevità, assumiamo standard AB
             let s2 = -search(&mut tmp, depth - 1, -beta, -alpha, 1, info, tt, z, km, ht);
             if s2 > alpha {
                alpha = s2;
                best_m = *m;
                pv.moves[0] = *m;
                pv.len = 1;
             }
        } else {
             if score > alpha {
                alpha = score;
                best_m = *m;
                pv.moves[0] = *m;
                pv.len = 1;
             }
        }
    }
    (alpha, best_m, pv)
}

// --- MAIN SEARCH (NEGAMAX) ---
fn search(board: &mut Scacchiera, mut depth: i32, mut alpha: i32, mut beta: i32, ply: usize, info: &mut SearchInfo, tt: &mut TranspositionTable, z: &ZobristKeys, km: &mut KillerMoves, ht: &mut HistoryTable) -> i32 {
    info.nodes += 1;
    if info.nodes % 2048 == 0 { check_time(info); }
    if info.stop { return 0; }

    // Mate distance pruning
    let mate_value = MATE_VALUE - ply as i32;
    if alpha < -mate_value { alpha = -mate_value; }
    if beta > mate_value - 1 { beta = mate_value - 1; }
    if alpha >= beta { return alpha; }

    let in_s = board.re_in_scacco(board.turno);
    
    // Check Extension: Se siamo in scacco, non riduciamo la profondità!
    if in_s { depth += 1; }

    // Se profondità 0, entra in Quiescence Search (lo Squalo)
    if depth <= 0 { 
        return quiescence(board, alpha, beta, info); 
    }

    let hash = board.get_hash(z);

    // TT Probe
    let mut tt_move = None;
    if let Some(entry) = tt.probe(hash) {
        tt_move = Some(Mossa { data: entry.move_data });
        if entry.depth as i32 >= depth && ply > 0 { // Non usare TT score alla radice per pruning
            let s = tt.get_score(&entry, ply);
            match entry.bound {
                1 => return s, // Exact
                2 => alpha = alpha.max(s), // Lower
                3 => beta = beta.min(s), // Upper
                _ => {}
            }
            if alpha >= beta { return s; }
        }
    }

    // Null Move Pruning
    if depth >= 3 && !in_s && ply > 0 {
        let r = 2; // Reduction
        let mut null_board = board.clone();
        null_board.make_null_move();
        // Cerca con finestra ridotta
        let score = -search(&mut null_board, depth - 1 - r, -beta, -beta + 1, ply + 1, info, tt, z, km, ht);
        if score >= beta { return beta; }
    }

    let mut moves = board.genera_mosse();
    // Ordina le mosse per provare prima le migliori (Alpha-Beta efficiency)
    moves.sort_by_key(|m| -punteggio_mossa(m, board, ply, km, ht, tt_move));
    
    let mut legal = 0;
    let mut best_score = -MATE_VALUE;
    let mut best_move = Mossa{data:0};

    for m in moves {
        let mut tmp = board.clone();
        if !tmp.esegui_mossa(&m, None) { continue; }
        legal += 1;

        // Late Move Reduction (LMR) - Tecnica avanzata per velocità
        // Se la mossa non è cattura, non è scacco, e siamo profondi, riduciamo depth
        let mut reduction = 0;
        if depth >= 3 && legal > 4 && !m.move_flag().is_capture() && !in_s {
            reduction = 1;
        }

        let mut score = -search(&mut tmp, depth - 1 - reduction, -beta, -alpha, ply + 1, info, tt, z, km, ht);
        
        // Se la mossa ridotta è buona, ricalcola a piena profondità
        if reduction > 0 && score > alpha {
            score = -search(&mut tmp, depth - 1, -beta, -alpha, ply + 1, info, tt, z, km, ht);
        }

        if score > best_score {
            best_score = score;
            best_move = m;
            if score > alpha {
                alpha = score;
                if alpha >= beta {
                    // Killer & History Update
                    if !m.move_flag().is_capture() {
                        km.update(m, ply);
                        ht.update(m, board.turno, depth);
                    }
                    tt.store(hash, best_score, m, depth, Bound::Lower, ply);
                    return beta; // Cutoff
                }
            }
        }
    }

    if legal == 0 {
        return if in_s { -MATE_VALUE + ply as i32 } else { 0 }; // Matto o Stallo
    }

    // Salva nel TT
    let bound = if best_score > alpha { Bound::Exact } else { Bound::Upper };
    tt.store(hash, best_score, best_move, depth, bound, ply);

    best_score
}

// --- QUIESCENCE SEARCH (LO SQUALO) ---
// Esplora solo catture e promozioni per evitare l'effetto orizzonte
fn quiescence(board: &mut Scacchiera, mut alpha: i32, beta: i32, info: &mut SearchInfo) -> i32 {
    info.nodes += 1;
    if info.nodes % 2048 == 0 { check_time(info); }
    if info.stop { return 0; }

    // 1. Stand Pat: Valuta la posizione attuale
    // Se siamo già messi bene, non serve rischiare catture dubbie
    let stand_pat = evaluate(board);
    if stand_pat >= beta { return beta; }
    if stand_pat > alpha { alpha = stand_pat; }

    let mut moves = board.genera_mosse();
    
    // 2. Filtra: Solo Catture e Promozioni (le mosse "rumorose")
    moves.retain(|m| m.move_flag().is_capture() || m.move_flag().is_promotion());
    
    // 3. Ordina: MVV-LVA (Mangia pezzi grossi con pezzi piccoli)
    moves.sort_by_key(|m| -mvv_lva_score(m, board));

    for m in moves {
        let mut tmp = board.clone();
        
        // 4. SEE (Static Exchange Evaluation) Pruning
        // Se la cattura sembra perdere materiale (es. QxP protetto), saltala!
        // Risparmia tantissimo tempo.
        if !m.move_flag().is_promotion() { // Le promozioni si controllano sempre
             let victim_val = get_piece_value(get_victim_piece(board, m.a()));
             if see(board, m.a(), victim_val, get_piece_value(get_piece_type(board, m.da())), board.turno) < 0 {
                 continue; 
             }
        }

        if !tmp.esegui_mossa(&m, None) { continue; }
        
        let score = -quiescence(&mut tmp, -beta, -alpha, info);
        
        if score >= beta { return beta; }
        if score > alpha { alpha = score; }
    }
    alpha
}

// --- HELPER FUNCTIONS ---

fn check_time(info: &mut SearchInfo) {
    if info.start_time.elapsed().as_millis() >= info.time_limit_ms {
        info.stop = true;
    }
}

// MVV-LVA: Most Valuable Victim - Least Valuable Attacker
// Ritorna un punteggio alto se PxQ, basso se QxP
fn mvv_lva_score(m: &Mossa, board: &Scacchiera) -> i32 {
    if m.move_flag().is_promotion() {
        return 15000; // Le promozioni hanno priorità altissima
    }
    let attacker = get_piece_type(board, m.da());
    let victim = get_victim_piece(board, m.a());
    
    let attacker_val = get_piece_value(attacker);
    let victim_val = get_piece_value(victim);

    // Formula classica: Vittima * 100 - Attaccante
    (victim_val * 100) - attacker_val + 10000 // +10000 per stare sopra le mosse "quiete"
}

fn punteggio_mossa(m: &Mossa, board: &Scacchiera, ply: usize, km: &KillerMoves, ht: &HistoryTable, tt: Option<Mossa>) -> i32 {
    // 1. Mossa Hash (TT) - Deve essere sempre la prima!
    if let Some(tm) = tt { 
        if m.data == tm.data { return 30000; } 
    }
    
    // 2. Catture e Promozioni (MVV-LVA)
    if m.move_flag().is_capture() || m.move_flag().is_promotion() {
        return mvv_lva_score(m, board);
    }
    
    // 3. Killer Moves (Mosse forti a questa profondità trovate altrove)
    if ply < 64 {
        if km.primary[ply].data == m.data { return 9000; }
        if km.secondary[ply].data == m.data { return 8000; }
    }
    
    // 4. History Heuristic (Mosse che storicamente funzionano bene)
    ht.scores[board.turno.indice()][m.da()][m.a()]
}

// Helper per identificare pezzi (brutale ma efficace sui bitboard)
fn get_piece_type(board: &Scacchiera, sq: usize) -> usize {
    let mask = 1u64 << sq;
    for p in 0..6 {
        if (board.pezzi[p] & mask) != 0 { return p; }
    }
    0 // Default Pedone (non dovrebbe accadere se chiamata correttamente)
}

fn get_victim_piece(board: &Scacchiera, sq: usize) -> usize {
    let mask = 1u64 << sq;
    // Cerca nei pezzi avversari
    let opp = board.turno.opposto().indice();
    let opp_pieces = board.colori[opp];
    if (opp_pieces & mask) == 0 { return 0; } // En passant o errore

    for p in 0..6 {
        if (board.pezzi[p] & mask) != 0 { return p; }
    }
    0
}

fn get_piece_value(p: usize) -> i32 {
    // P=1, N=3, B=3, R=5, Q=9, K=0 (approssimato per ordinamento)
    // Usiamo MG_VAL di evaluation per coerenza
    MG_VAL[p] / 100 
}