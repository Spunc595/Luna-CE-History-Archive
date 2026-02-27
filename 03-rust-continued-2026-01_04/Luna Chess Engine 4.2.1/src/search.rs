use crate::board::{Colore, Mossa, Scacchiera};
use crate::nnue::LunaNNUE;
use crate::tt::{Bound, TranspositionTable};
use crate::zobrist::ZobristKeys;
use std::time::Instant;

pub const MAX_DEPTH: usize = 64;
const MATE_SCORE: i32 = 49000;
const INFINITY: i32 = 50000;

pub struct SearchInfo {
    pub start_time: Instant,
    pub hard_limit: u128,
    pub soft_limit: u128,
    pub depth_limit: i32,
    pub nodes: u64,
    pub stopped: bool,
    pub killer: [[Mossa; 2]; MAX_DEPTH],
    pub history: [[[i32; 64]; 64]; 2],
}

impl SearchInfo {
    pub fn new(time_limit: u128, depth_limit: i32) -> Self {
        let soft = if time_limit > 500 { time_limit * 60 / 100 } else { time_limit };
        SearchInfo {
            start_time: Instant::now(),
            hard_limit: time_limit,
            soft_limit: soft,
            depth_limit,
            nodes: 0,
            stopped: false,
            killer: [[Mossa::null(); 2]; MAX_DEPTH],
            history: [[[0; 64]; 64]; 2],
        }
    }

    #[inline(always)]
    pub fn check_time(&mut self) -> bool {
        if (self.nodes & 2047) == 0 {
            if self.start_time.elapsed().as_millis() >= self.hard_limit {
                self.stopped = true;
            }
        }
        self.stopped
    }
}

pub fn iterative_deepening(
    board: &mut Scacchiera,
    info: &mut SearchInfo,
    tt: &mut TranspositionTable,
    z: &ZobristKeys,
    nnue: Option<&LunaNNUE>,
) -> (Mossa, i32) {
    let mut best = Mossa::null();
    let mut score = 0;
    let mut last_best = Mossa::null();
    let mut stable = 0;

    let mut alpha = -INFINITY;
    let mut beta = INFINITY;

    for depth in 1..=info.depth_limit {
        let (v, pv) = negamax(board, depth, alpha, beta, info, tt, z, nnue, true);

        if info.stopped && depth > 1 {
            break;
        }

        score = v;

        if score <= alpha || score >= beta {
            alpha = -INFINITY;
            beta = INFINITY;
            continue;
        } else {
            alpha = score - 50;
            beta = score + 50;
        }

        if !pv.is_empty() {
            best = pv[0];

            let elapsed = info.start_time.elapsed().as_millis();

            if best.data == last_best.data {
                stable += 1;
            } else {
                last_best = best;
                stable = 0;
            }

            if elapsed > info.soft_limit && (stable >= 3 || depth > 8) {
                info.stopped = true;
            }

            let nps = if elapsed > 0 { info.nodes as u128 * 1000 / elapsed } else { 0 };

            print!(
                "info depth {} score cp {} nodes {} nps {} time {} pv",
                depth, score, info.nodes, nps, elapsed
            );

            for m in &pv {
                print!(" {}", m.to_uci());
            }
            println!();
        }

        if info.stopped {
            break;
        }
    }

    if best.is_null() {
        let mosse = board.genera_mosse_legali(z);
        if !mosse.is_empty() {
            best = mosse[0];
        }
    }

    (best, score)
}

fn negamax(
    board: &mut Scacchiera,
    depth: i32,
    mut alpha: i32,
    beta: i32,
    info: &mut SearchInfo,
    tt: &mut TranspositionTable,
    z: &ZobristKeys,
    nnue: Option<&LunaNNUE>,
    allow_null: bool,
) -> (i32, Vec<Mossa>) {
    if info.check_time() {
        return (0, vec![]);
    }

    info.nodes += 1;
    let pv_node = beta - alpha > 1;

    if board.ply > 0 {
        if board.mezze_mosse >= 100 || board.is_repetition() {
            return (0, vec![]);
        }
    }

    if let Some(val) = tt.probe(board.hash, depth, alpha, beta, board.ply as i32) {
        if !pv_node {
            return (val, vec![]);
        }
    }

    let in_check = board.in_scacco();
    let new_depth = if in_check { depth + 1 } else { depth };

    if new_depth <= 0 {
        let q = quiescence(board, alpha, beta, info, z, nnue);
        return (q, vec![]);
    }

    let eval_assoluta = crate::evaluation::evaluate(board);
    let static_eval = if board.turno == Colore::Bianco { eval_assoluta } else { -eval_assoluta };

    if !pv_node && !in_check && new_depth < 4 {
        let rfp_margin = 120 * new_depth;
        if static_eval - rfp_margin >= beta {
            return (static_eval, vec![]);
        }
    }

    if allow_null && !pv_node && new_depth >= 3 && !in_check {
        if static_eval >= beta {
            let r = if new_depth > 6 { 3 } else { 2 };
            let undo = board.fai_mossa_nulla(z);
            let (v, _) = negamax(board, new_depth - r - 1, -beta, -beta + 1, info, tt, z, nnue, false);
            board.annulla_mossa_nulla(undo, z);
            if -v >= beta {
                return (beta, vec![]);
            }
        }
    }

    let tt_move = tt.get_move(board.hash);
    
    let mut best_val = -INFINITY;
    let mut best_pv = vec![];
    let mut flag = Bound::Alpha;
    let mut moves_searched = 0;
    let mut any_legal_moves = false;

    macro_rules! evaluate_move {
        ($m:expr) => {
            any_legal_moves = true;
            let undo = board.esegui_mossa(&$m, z);
            let gives_check = board.in_scacco(); 
            moves_searched += 1;

            let mut needs_full_search = true;
            let mut score_val = 0;
            let mut pv_child_val = vec![];

            if moves_searched == 1 {
                let (v, pv) = negamax(board, new_depth - 1, -beta, -alpha, info, tt, z, nnue, true);
                score_val = -v;
                pv_child_val = pv;
                needs_full_search = false;
            } else {
                if new_depth >= 3 && !in_check && !gives_check && !$m.is_cattura() && !$m.is_promozione() {
                    let r = if moves_searched >= 6 { 2 } else { 1 };
                    let (v, pv) = negamax(board, new_depth - 1 - r, -alpha - 1, -alpha, info, tt, z, nnue, true);
                    score_val = -v;
                    if score_val <= alpha {
                        pv_child_val = pv;
                        needs_full_search = false;
                    }
                }

                if needs_full_search {
                    let (v, pv) = negamax(board, new_depth - 1, -alpha - 1, -alpha, info, tt, z, nnue, true);
                    score_val = -v;
                    if score_val > alpha && score_val < beta {
                        let (v2, pv2) = negamax(board, new_depth - 1, -beta, -alpha, info, tt, z, nnue, true);
                        score_val = -v2;
                        pv_child_val = pv2;
                    } else {
                        pv_child_val = pv;
                    }
                }
            }

            board.annulla_mossa(&$m, undo, z);

            if info.stopped {
                return (0, vec![]);
            }

            if score_val > best_val {
                best_val = score_val;
                best_pv.clear();
                best_pv.push($m);
                best_pv.extend(pv_child_val);
            }

            if score_val > alpha {
                alpha = score_val;
                flag = Bound::Exact;
            }

            if alpha >= beta {
                if !$m.is_cattura() && !$m.is_promozione() {
                    let d = board.ply as usize;
                    if d < MAX_DEPTH {
                        if $m.data != info.killer[d][0].data {
                            info.killer[d][1] = info.killer[d][0];
                            info.killer[d][0] = $m;
                        }
                        let side = board.turno.opposto().indice();
                        info.history[side][$m.da()][$m.a()] += depth * depth;
                    }
                }
                tt.store(board.hash, depth, beta, Bound::Beta, $m, board.ply as i32);
                return (beta, vec![]);
            }
        };
    }

    let mut captures = board.genera_catture_legali(z);
    crate::movegen::ordina_mosse(&mut captures, board, tt_move, depth, info);
    
    for m in captures {
        evaluate_move!(m);
    }

    if !info.stopped {
        let mut quiets = board.genera_silenziose_legali(z);
        crate::movegen::ordina_mosse(&mut quiets, board, tt_move, depth, info);
        
        for m in quiets {
            if !pv_node && !in_check && new_depth < 3 {
                let futility_margin = 250;
                if static_eval + futility_margin <= alpha {
                    let undo = board.esegui_mossa(&m, z);
                    let gives_check = board.in_scacco();
                    board.annulla_mossa(&m, undo, z);
                    
                    if !gives_check {
                        any_legal_moves = true; // BUG FIX: Evita che il motore creda di non avere mosse legali
                        moves_searched += 1;
                        continue; 
                    }
                }
            }
            evaluate_move!(m);
        }
    }

    if !any_legal_moves {
        if in_check {
            return (-MATE_SCORE + board.ply as i32, vec![]);
        } else {
            return (0, vec![]);
        }
    }

    tt.store(
        board.hash,
        depth,
        best_val,
        flag,
        if !best_pv.is_empty() { best_pv[0] } else { Mossa::null() },
        board.ply as i32,
    );

    (best_val, best_pv)
}

fn quiescence(
    board: &mut Scacchiera,
    mut alpha: i32,
    beta: i32,
    info: &mut SearchInfo,
    z: &ZobristKeys,
    nnue: Option<&LunaNNUE>,
) -> i32 {
    if info.check_time() {
        return 0;
    }

    info.nodes += 1;

    // --- PROTEZIONI ANTI-CRASH ---
    if board.ply > 0 && board.is_repetition() {
        return 0; // Se c'è un loop di scacchi, dichiara patta e taglia.
    }
    if board.ply as usize >= MAX_DEPTH {
        let eval_assoluta = crate::evaluation::evaluate(board);
        return if board.turno == Colore::Bianco { eval_assoluta } else { -eval_assoluta };
    }

    let in_check = board.in_scacco();

    // Se NON siamo in scacco, applichiamo la stand_pat normale
    if !in_check {
        let eval_assoluta = crate::evaluation::evaluate(board);
        let stand = if board.turno == Colore::Bianco { eval_assoluta } else { -eval_assoluta };

        if stand >= beta {
            return beta;
        }
        if stand > alpha {
            alpha = stand;
        }
    }

    // Qui la magia: se siamo sotto scacco, generiamo ANCHE le mosse di fuga. Altrimenti solo catture.
    let mut moves = if in_check {
        board.genera_mosse_legali(z)
    } else {
        board.genera_catture_legali(z)
    };

    crate::movegen::ordina_mosse(&mut moves, board, Mossa::null(), 0, info);
    let mut legal_moves_played = 0;

    for m in moves {
        let undo = board.esegui_mossa(&m, z);
        let score = -quiescence(board, -beta, -alpha, info, z, nnue);
        board.annulla_mossa(&m, undo, z);

        legal_moves_played += 1;

        if score >= beta {
            return beta;
        }
        if score > alpha {
            alpha = score;
        }
    }

    // Se eravamo in scacco e non abbiamo trovato vie di fuga legali... MATTO!
    if in_check && legal_moves_played == 0 {
        return -MATE_SCORE + board.ply as i32; 
    }

    alpha
}