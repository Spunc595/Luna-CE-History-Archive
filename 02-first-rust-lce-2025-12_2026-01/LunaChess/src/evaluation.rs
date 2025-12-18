use crate::board::{Board, Piece, Color};

pub fn evaluate(board: &Board) -> i32 {
    let mut score = 0;
    
    // Material evaluation
    score += material_score(board);
    
    // Piece-square tables
    score += piece_square_score(board);
    
    // Adjust for active color
    match board.active_color() {
        Color::White => score,
        Color::Black => -score,
    }
}

fn material_score(board: &Board) -> i32 {
    let mut score = 0;
    
    for rank in 0..8 {
        for file in 0..8 {
            if let Some(square) = crate::board::Square::new(file, rank) {
                if let Some((piece, color)) = board.get_piece(square) {
                    let piece_value = match piece {
                        Piece::Pawn => 100,
                        Piece::Knight => 320,
                        Piece::Bishop => 330,
                        Piece::Rook => 500,
                        Piece::Queen => 900,
                        Piece::King => 20000,
                    };
                    
                    match color {
                        Color::White => score += piece_value,
                        Color::Black => score -= piece_value,
                    }
                }
            }
        }
    }
    
    score
}

fn piece_square_score(board: &Board) -> i32 {
    let mut score = 0;
    
    // Simplified piece-square tables
    // Pawns are better in the center and advancing
    let pawn_table = [
        [0,  0,  0,  0,  0,  0,  0,  0],
        [50, 50, 50, 50, 50, 50, 50, 50],
        [10, 10, 20, 30, 30, 20, 10, 10],
        [5,  5, 10, 25, 25, 10,  5,  5],
        [0,  0,  0, 20, 20,  0,  0,  0],
        [5, -5,-10,  0,  0,-10, -5,  5],
        [5, 10, 10,-20,-20, 10, 10,  5],
        [0,  0,  0,  0,  0,  0,  0,  0],
    ];
    
    // Knights are better in the center
    let knight_table = [
        [-50,-40,-30,-30,-30,-30,-40,-50],
        [-40,-20,  0,  0,  0,  0,-20,-40],
        [-30,  0, 10, 15, 15, 10,  0,-30],
        [-30,  5, 15, 20, 20, 15,  5,-30],
        [-30,  0, 15, 20, 20, 15,  0,-30],
        [-30,  5, 10, 15, 15, 10,  5,-30],
        [-40,-20,  0,  5,  5,  0,-20,-40],
        [-50,-40,-30,-30,-30,-30,-40,-50],
    ];
    
    for rank in 0..8 {
        for file in 0..8 {
            if let Some(square) = crate::board::Square::new(file, rank) {
                if let Some((piece, color)) = board.get_piece(square) {
                    let table_rank = rank as usize;
                    let table_file = file as usize;
                    
                    let piece_score = match piece {
                        Piece::Pawn => pawn_table[table_rank][table_file],
                        Piece::Knight => knight_table[table_rank][table_file],
                        _ => 0, // Simplified for now
                    };
                    
                    match color {
                        Color::White => score += piece_score,
                        Color::Black => {
                            // Flip the table for black pieces
                            let flipped_rank = 7 - table_rank;
                            score -= match piece {
                                Piece::Pawn => pawn_table[flipped_rank][table_file],
                                Piece::Knight => knight_table[flipped_rank][table_file],
                                _ => 0,
                            };
                        }
                    }
                }
            }
        }
    }
    
    score
}