use std::time::Instant;
use crate::board::{Scacchiera, Mossa, Colore, Pezzo};
use crate::tt::{TranspositionTable, Bound};
use crate::zobrist::ZobristKeys;
use crate::evaluation::SEE_VAL;

pub const MATE_VALUE: i32 = 49000;
const ASPIRATION_WINDOW: i32 = 40;

pub struct SearchInfo {
    pub start_time: Instant,
    pub time_limit_ms: u128,
    pub nodes: u64,
    pub stop: bool,
}

#[derive(Clone, Copy)]
pub struct PVLine {
    pub moves: [Mossa; 64],
    pub len: usize,
}

pub struct KillerMoves {
    pub primary: [Mossa; 64],
    pub secondary: [Mossa; 64],
}

impl KillerMoves {
    pub fn new() -> Self {
        Self { primary: [Mossa { data: 0 }; 64], secondary: [Mossa { data: 0 }; 64] }
    }
    pub fn update(&mut self, m: Mossa, ply: usize) {
        if ply < 64 && m != self.primary[ply] {
            self.secondary[ply] = self.primary[ply];
            self.primary[ply] = m;
        }
    }
}

pub struct HistoryTable {
    pub scores: [[[i32; 64]; 64]; 2],
}

impl HistoryTable {
    pub fn new() -> Self { Self { scores: [[[0; 64]; 64]; 2] } }
    pub fn update(&mut self, m: Mossa, colore: Colore, depth: i32) {
        let d = depth as i32;
        self.scores[colore.indice()][m.da()][m.a()] += d * d;
    }
}

pub fn iterative_deepening(board: &mut Scacchiera, info: &mut SearchInfo, tt: &mut TranspositionTable, zobrist: &ZobristKeys) -> Mossa {
    let mut best_move = Mossa { data: 0 };
    let mut killers = KillerMoves::new();
    let mut history = HistoryTable::new();
    let mut last_score = 0;

    for depth in 1..65 {
        let elapsed = info.start_time.elapsed().as_millis();
        
        // --- COMMENTO UMANO: GESTIONE TEMPO ---
        if depth > 1 && elapsed > (info.time_limit_ms / 2) {
            println!("info string Luna interrompe la ricerca per gestire il tempo (Profondità raggiunta: {})", depth - 1);
            break; 
        }

        let mut alpha = -100000;
        let mut beta = 100000;
        if depth > 5 { 
            alpha = last_score - ASPIRATION_WINDOW; 
            beta = last_score + ASPIRATION_WINDOW; 
        }

        loop {
            let (score, m, pv) = search_root(board, depth, alpha, beta, info, tt, zobrist, &mut killers, &mut history);
            if info.stop { break; }

            if score <= alpha {
                alpha = -100000;
                println!("info string Ricerca fallita in basso (score {}), riprovo con finestra aperta", score);
            } else if score >= beta {
                beta = 100000;
                println!("info string Ricerca fallita in alto (score {}), riprovo con finestra aperta", score);
            } else {
                last_score = score;
                best_move = m;
                let elapsed_now = info.start_time.elapsed().as_millis().max(1);
                let nps = (info.nodes as u128 * 1000) / elapsed_now;
                
                // Commento sulla valutazione
                if depth > 8 {
                    let commento = match score {
                        s if s > 150 => "Vedo un vantaggio significativo.",
                        s if s < -150 => "La posizione è critica, cerco difesa.",
                        _ => "La partita è in pieno equilibrio."
                    };
                    println!("info string [D{}] {}", depth, commento);
                }

                print!("info depth {} score cp {} nodes {} nps {} pv", depth, score, info.nodes, nps);
                for i in 0..pv.len { if pv.moves[i].data != 0 { print!(" {}", pv.moves[i]); } }
                println!();
                break;
            }
        }
        if info.stop || last_score.abs() > MATE_VALUE - 100 { break; }
    }
    best_move
}

fn punteggio_mossa(m: &Mossa, board: &Scacchiera, ply: usize, killers: &KillerMoves, history: &HistoryTable, tt_move: Option<Mossa>) -> i32 {
    if let Some(tm) = tt_move { if *m == tm { return 30000; } }
    if m.move_flag().is_capture() {
        let victim = board.get_piece_at(m.a()).map_or(0, |p| p.indice());
        let attacker = board.get_piece_at(m.da()).map_or(0, |p| p.indice());
        let see_score = crate::evaluation::see(board, m.a(), SEE_VAL[victim], SEE_VAL[attacker], board.turno);
        if see_score < 0 { return -20000 + see_score; }
        return 20000 + (10 * victim as i32) - attacker as i32;
    }
    if ply < 64 {
        if *m == killers.primary[ply] { return 9000; }
        if *m == killers.secondary[ply] { return 8000; }
    }
    history.scores[board.turno.indice()][m.da()][m.a()]
}

fn search_root(board: &mut Scacchiera, depth: i32, mut alpha: i32, beta: i32, info: &mut SearchInfo, tt: &mut TranspositionTable, zobrist: &ZobristKeys, killers: &mut KillerMoves, history: &mut HistoryTable) -> (i32, Mossa, PVLine) {
    let mut pv = PVLine { moves: [Mossa { data: 0 }; 64], len: 0 };
    let hash = board.get_hash(zobrist);
    let tt_move = tt.probe(hash).map(|e| Mossa { data: e.move_data });
    
    let mut moves: Vec<Mossa> = board.genera_mosse().into_iter().filter(|m| {
        let mut tmp = board.clone();
        tmp.esegui_mossa(m, None)
    }).collect();

    if moves.is_empty() { return (0, Mossa { data: 0 }, pv); }
    moves.sort_by_key(|m| -punteggio_mossa(m, board, 0, killers, history, tt_move));

    let mut best_move = moves[0];
    for m in moves {
        let mut temp_board = board.clone();
        temp_board.esegui_mossa(&m, None);
        let score = -search(&mut temp_board, depth - 1, -beta, -alpha, 1, info, tt, zobrist, killers, history);
        if info.stop { break; }
        if score > alpha { alpha = score; best_move = m; pv.moves[0] = m; pv.len = 1; }
    }
    (alpha, best_move, pv)
}

fn search(board: &mut Scacchiera, depth: i32, mut alpha: i32, mut beta: i32, ply: usize, info: &mut SearchInfo, tt: &mut TranspositionTable, zobrist: &ZobristKeys, killers: &mut KillerMoves, history: &mut HistoryTable) -> i32 {
    info.nodes += 1;
    if info.nodes % 2048 == 0 && info.start_time.elapsed().as_millis() >= info.time_limit_ms { info.stop = true; }
    if info.stop { return 0; }

    let in_scacco = board.re_in_scacco(board.turno);
    let extension = if in_scacco { 1 } else { 0 };

    if depth + extension <= 0 { return quiescence(board, alpha, beta, info); }

    let hash = board.get_hash(zobrist);
    let mut tt_move = None;
    if let Some(entry) = tt.probe(hash) {
        tt_move = Some(Mossa { data: entry.move_data });
        if entry.depth as i32 >= (depth + extension) {
            let tt_score = tt.get_score(&entry, ply);
            match entry.bound {
                1 => return tt_score,
                2 => alpha = alpha.max(tt_score),
                3 => beta = beta.min(tt_score),
                _ => {}
            }
            if alpha >= beta { return tt_score; }
        }
    }

    if depth >= 3 && !in_scacco && ply > 0 {
        let mut temp_board = board.clone();
        temp_board.make_null_move();
        let score = -search(&mut temp_board, depth - 1 - 2, -beta, -beta + 1, ply + 1, info, tt, zobrist, killers, history);
        if score >= beta { return beta; }
    }

    let mut moves = board.genera_mosse();
    moves.sort_by_key(|m| -punteggio_mossa(m, board, ply, killers, history, tt_move));

    let mut legal_count = 0;
    let mut best_score = -100000;
    let mut current_best_move = Mossa { data: 0 };

    for m in moves {
        let mut temp_board = board.clone();
        if !temp_board.esegui_mossa(&m, None) { continue; }
        legal_count += 1;
        
        let da_scacco = temp_board.re_in_scacco(temp_board.turno.opposto());
        
        let score = if depth >= 3 && legal_count > 3 && !m.move_flag().is_capture() && !in_scacco && !da_scacco {
            let s = -search(&mut temp_board, depth - 2 + extension, -alpha - 1, -alpha, ply + 1, info, tt, zobrist, killers, history);
            if s > alpha { 
                -search(&mut temp_board, depth - 1 + extension, -beta, -alpha, ply + 1, info, tt, zobrist, killers, history) 
            } else { s }
        } else {
            -search(&mut temp_board, depth - 1 + extension, -beta, -alpha, ply + 1, info, tt, zobrist, killers, history)
        };

        if score > best_score {
            best_score = score; 
            current_best_move = m;
            if score > alpha {
                alpha = score;
                if alpha >= beta {
                    tt.store(hash, score, m, depth + extension, Bound::Lower, ply);
                    if !m.move_flag().is_capture() { 
                        killers.update(m, ply); 
                        history.update(m, board.turno.opposto(), depth); 
                    }
                    return beta;
                }
            }
        }
    }

    if legal_count == 0 { 
        return if in_scacco { -MATE_VALUE + ply as i32 } else { 0 }; 
    }

    let bound = if best_score <= alpha { Bound::Upper } else { Bound::Exact };
    tt.store(hash, best_score, current_best_move, depth + extension, bound, ply);
    alpha
}

fn quiescence(board: &mut Scacchiera, mut alpha: i32, beta: i32, info: &mut SearchInfo) -> i32 {
    let stand_pat = crate::evaluation::evaluate(board);
    if stand_pat >= beta { return beta; }
    if stand_pat > alpha { alpha = stand_pat; }
    
    let mut moves = board.genera_mosse();
    // Filtro catture con annotazione di tipo esplicita
    moves.retain(|m: &Mossa| m.move_flag().is_capture());
    
    moves.sort_by_key(|m: &Mossa| {
        let target = board.get_piece_at(m.a()).map_or(0, |p| p.indice());
        let attacker = board.get_piece_at(m.da()).map_or(0, |p| p.indice());
        -crate::evaluation::see(board, m.a(), SEE_VAL[target], SEE_VAL[attacker], board.turno)
    });

    for m in moves {
        let mut temp_board = board.clone();
        if !temp_board.esegui_mossa(&m, None) { continue; }
        info.nodes += 1;
        let score = -quiescence(&mut temp_board, -beta, -alpha, info);
        if score >= beta { return beta; }
        if score > alpha { alpha = score; }
    }
    alpha
}