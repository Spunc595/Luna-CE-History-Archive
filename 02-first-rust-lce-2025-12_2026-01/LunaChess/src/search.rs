use crate::board::Board;
use crate::movegen::MoveGenerator;
use crate::evaluation::evaluate;

const INFINITY: i32 = 1000000;

pub fn search(board: &Board, depth: i32) -> (Option<crate::movegen::Move>, i32, u64) {
    let mut nodes_searched = 0;
    
    if depth == 0 {
        return (None, evaluate(board), 1);
    }
    
    let moves = MoveGenerator::generate_legal_moves(board);
    
    if moves.is_empty() {
        return (None, evaluate(board), 1);
    }
    
    let mut best_score = -INFINITY;
    let mut best_move = None;
    
    for mv in moves {
        let mut new_board = board.clone();
        new_board.make_move(&mv);
        
        let (_, score, nodes) = negamax(&new_board, depth - 1, -INFINITY, INFINITY);
        nodes_searched += nodes;
        
        let score = -score;
        
        if score > best_score {
            best_score = score;
            best_move = Some(mv);
        }
    }
    
    (best_move, best_score, nodes_searched)
}

fn negamax(board: &Board, depth: i32, alpha: i32, beta: i32) -> (Option<crate::movegen::Move>, i32, u64) {
    let mut alpha = alpha;
    let mut nodes_searched = 1;
    
    if depth == 0 {
        return (None, evaluate(board), 1);
    }
    
    let moves = MoveGenerator::generate_legal_moves(board);
    
    if moves.is_empty() {
        return (None, evaluate(board), 1);
    }
    
    let mut best_score = -INFINITY;
    let mut best_move = None;
    
    for mv in moves {
        let mut new_board = board.clone();
        new_board.make_move(&mv);
        
        let (_, score, nodes) = negamax(&new_board, depth - 1, -beta, -alpha);
        nodes_searched += nodes;
        
        let score = -score;
        
        if score > best_score {
            best_score = score;
            best_move = Some(mv);
        }
        
        if score > alpha {
            alpha = score;
        }
        
        if alpha >= beta {
            break;
        }
    }
    
    (best_move, best_score, nodes_searched)
}

pub fn iterative_deepening(board: &Board, max_depth: i32, time_limit_ms: u64) -> (Option<crate::movegen::Move>, i32, u64) {
    use std::time::Instant;
    
    let start_time = Instant::now();
    let mut best_result = (None, 0, 0);
    
    for depth in 1..=max_depth {
        let elapsed = start_time.elapsed().as_millis() as u64;
        if elapsed > time_limit_ms {
            break;
        }
        
        let result = search(board, depth);
        best_result = result;
        
        println!("Depth {}: score = {}, nodes = {}", depth, best_result.1, best_result.2);
        
        if elapsed * 2 > time_limit_ms {
            break;
        }
    }
    
    best_result
}