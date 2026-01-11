use std::time::Instant;
use crate::board::{Scacchiera, Mossa};
use crate::evaluation::evaluate;
use crate::tt::{TranspositionTable, Bound};
use crate::zobrist::ZobristKeys;

pub const MATE_VALUE: i32 = 49000;

pub struct SearchInfo {
    pub start_time: Instant,
    pub time_limit_ms: u128,
    pub nodes: u64,
    pub stop: bool,
    pub history: Vec<u64>,
}

pub fn iterative_deepening(board: &mut Scacchiera, info: &mut SearchInfo, tt: &mut TranspositionTable, z: &ZobristKeys) -> Mossa {
    // Generiamo le mosse legali per la radice
    let all_legal_moves = board.genera_mosse();
    let mut best_m = all_legal_moves[0];
    
    // Iterative Deepening
    for depth in 1..64 {
        if info.stop { break; }
        
        let score = negamax(board, -MATE_VALUE, MATE_VALUE, depth, 0, true, info, tt, z);
        
        if info.stop { break; }

        // Recuperiamo la mossa migliore dalla TT
        if let Some(entry) = tt.probe(board.hash) {
            let tt_move = Mossa { data: entry.move_data };
            
            // FILTRO DI LEGALITÀ RIGOROSO
            // Se la TT suggerisce una mossa non legale (per collisione), non caricarla
            if all_legal_moves.iter().any(|m| m.data == tt_move.data) {
                best_m = tt_move;
            }

            let elapsed = info.start_time.elapsed().as_millis().max(1);
            let nps = (info.nodes as u128 * 1000) / elapsed;
            
            // Formato standard UCI per la comunicazione con Arena/Rybka
            println!("info depth {} score cp {} nodes {} nps {} pv {}", 
                depth, score, info.nodes, nps, best_m);
        }

        // Se troviamo un matto, fermiamo la ricerca
        if score.abs() > MATE_VALUE - 100 { break; }
    }
    
    best_m
}

fn negamax(board: &mut Scacchiera, mut alpha: i32, mut beta: i32, depth: i32, ply: usize, allow_null: bool, info: &mut SearchInfo, tt: &mut TranspositionTable, z: &ZobristKeys) -> i32 {
    info.nodes += 1;
    
    // Controllo del tempo ogni 2048 nodi
    if info.nodes & 2047 == 0 && info.start_time.elapsed().as_millis() >= info.time_limit_ms {
        info.stop = true;
    }
    if info.stop { return 0; }

    // Rilevamento patta per ripetizione
    if ply > 0 && info.history.contains(&board.hash) { return 0; }

    // Raggiunta la profondità zero, passiamo alla ricerca di quiescenza
    if depth <= 0 { return quiescence(board, alpha, beta, info, z); }

    // Consultazione Transposition Table (TT)
    let mut tt_m = Mossa { data: 0 };
    if let Some(entry) = tt.probe(board.hash) {
        tt_m = Mossa { data: entry.move_data };
        if entry.depth as i32 >= depth {
            let s = tt.get_score(&entry, ply);
            match entry.bound {
                1 => return s,             // Exact
                2 => alpha = alpha.max(s),  // Lower Bound
                3 => beta = beta.min(s),   // Upper Bound
                _ => {}
            }
            if alpha >= beta { return s; }
        }
    }

    // Null Move Pruning (NMP)
    // Non si usa se siamo sotto scacco o se siamo vicini al Re
    if allow_null && depth >= 3 && !board.re_in_scacco(board.turno) {
        let mut temp_board = board.clone();
        temp_board.make_null_move_hash(z); // Metodo specifico per aggiornare hash della null move
        let score = -negamax(&mut temp_board, -beta, -beta + 1, depth - 3, ply + 1, false, info, tt, z);
        if score >= beta { return beta; }
    }

    let mut moves = board.genera_mosse();
    
    // Ordinamento Mosse (Move Ordering)
    // 1. Mossa TT, 2. Catture, 3. Altro
    moves.sort_by_key(|m: &Mossa| {
        if m.data == tt_m.data { -1000000 }
        else if m.move_flag().is_capture() { -10000 }
        else { 0 }
    });

    let mut legal_moves = 0;
    let mut best_score = -MATE_VALUE;
    let mut best_move = Mossa { data: 0 };

    info.history.push(board.hash);

    for m in moves {
        let mut temp_board = board.clone();
        if !temp_board.esegui_mossa(&m, z) { continue; }
        legal_moves += 1;

        let score = -negamax(&mut temp_board, -beta, -alpha, depth - 1, ply + 1, true, info, tt, z);
        
        if score > best_score {
            best_score = score;
            best_move = m;
            if score > alpha {
                alpha = score;
            }
        }

        if alpha >= beta {
            tt.store(board.hash, beta, m, depth, Bound::Lower, ply);
            info.history.pop();
            return beta;
        }
    }

    info.history.pop();

    // Gestione Scacco Matto e Stallo
    if legal_moves == 0 {
        return if board.re_in_scacco(board.turno) { -MATE_VALUE + ply as i32 } else { 0 };
    }

    // Salvataggio dei risultati nella TT
    let bound = if best_score >= beta { Bound::Lower } else if best_score > alpha { Bound::Exact } else { Bound::Upper };
    tt.store(board.hash, best_score, best_move, depth, bound, ply);

    best_score
}

fn quiescence(board: &mut Scacchiera, mut alpha: i32, beta: i32, info: &mut SearchInfo, z: &ZobristKeys) -> i32 {
    let stand_pat = evaluate(board);
    if stand_pat >= beta { return beta; }
    alpha = alpha.max(stand_pat);

    let mut moves = board.genera_mosse();
    // In Quiescence consideriamo solo le catture
    moves.retain(|m: &Mossa| m.move_flag().is_capture());
    
    for m in moves {
        let mut temp_board = board.clone();
        if !temp_board.esegui_mossa(&m, z) { continue; }
        info.nodes += 1;
        
        let score = -quiescence(&mut temp_board, -beta, -alpha, info, z);
        
        if score >= beta { return beta; }
        alpha = alpha.max(score);
    }
    alpha
}