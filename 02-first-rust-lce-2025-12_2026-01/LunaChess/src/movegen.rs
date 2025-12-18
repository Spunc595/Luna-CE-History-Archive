use std::fmt;
use crate::board::{Board, Piece, Color, Square};

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Move {
    pub from: Square,
    pub to: Square,
    pub promotion: Option<Piece>,
}

impl Move {
    pub fn new(from: Square, to: Square) -> Self {
        Move { from, to, promotion: None }
    }

    pub fn new_promotion(from: Square, to: Square, promotion: Piece) -> Self {
        Move { from, to, promotion: Some(promotion) }
    }
}

impl fmt::Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}", self.from, self.to)?;
        if let Some(promotion) = self.promotion {
            let ch = match promotion {
                Piece::Queen => 'q',
                Piece::Rook => 'r',
                Piece::Bishop => 'b',
                Piece::Knight => 'n',
                _ => 'q', // Default to queen
            };
            write!(f, "{}", ch)?;
        }
        Ok(())
    }
}

pub struct MoveGenerator;

impl MoveGenerator {
    pub fn generate_legal_moves(board: &Board) -> Vec<Move> {
        let mut moves = Vec::new();
        let color = board.active_color();
        
        for rank in 0..8 {
            for file in 0..8 {
                if let Some(square) = Square::new(file, rank) {
                    if let Some((piece, piece_color)) = board.get_piece(square) {
                        if piece_color == color {
                            let mut piece_moves = Self::generate_moves_for_piece(board, square, piece, color);
                            moves.append(&mut piece_moves);
                        }
                    }
                }
            }
        }
        
        // Filter out moves that leave king in check
        moves.into_iter()
            .filter(|mv| {
                let mut new_board = board.clone();
                new_board.make_move(mv);
                !new_board.is_in_check(color)
            })
            .collect()
    }

    fn generate_moves_for_piece(board: &Board, square: Square, piece: Piece, color: Color) -> Vec<Move> {
        match piece {
            Piece::Pawn => Self::generate_pawn_moves(board, square, color),
            Piece::Knight => Self::generate_knight_moves(board, square, color),
            Piece::Bishop => Self::generate_bishop_moves(board, square, color),
            Piece::Rook => Self::generate_rook_moves(board, square, color),
            Piece::Queen => Self::generate_queen_moves(board, square, color),
            Piece::King => Self::generate_king_moves(board, square, color),
        }
    }

    fn generate_pawn_moves(board: &Board, square: Square, color: Color) -> Vec<Move> {
        let mut moves = Vec::new();
        let direction = match color {
            Color::White => 1,
            Color::Black => -1,
        };
        let start_rank = match color {
            Color::White => 1,
            Color::Black => 6,
        };
        let promotion_rank = match color {
            Color::White => 7,
            Color::Black => 0,
        };

        let current_rank = square.rank() as i8;
        let current_file = square.file() as i8;

        // Single move forward
        let new_rank = current_rank + direction;
        if new_rank >= 0 && new_rank < 8 {
            let target_square = Square::new(current_file as u8, new_rank as u8).unwrap();
            if board.get_piece(target_square).is_none() {
                if new_rank as u8 == promotion_rank {
                    // Promotion moves
                    moves.push(Move::new_promotion(square, target_square, Piece::Queen));
                    moves.push(Move::new_promotion(square, target_square, Piece::Rook));
                    moves.push(Move::new_promotion(square, target_square, Piece::Bishop));
                    moves.push(Move::new_promotion(square, target_square, Piece::Knight));
                } else {
                    moves.push(Move::new(square, target_square));
                }

                // Double move from starting position
                if current_rank as u8 == start_rank {
                    let double_rank = current_rank + 2 * direction;
                    let double_square = Square::new(current_file as u8, double_rank as u8).unwrap();
                    if board.get_piece(double_square).is_none() {
                        moves.push(Move::new(square, double_square));
                    }
                }
            }
        }

        // Captures
        for file_offset in [-1, 1] {
            let target_file = current_file + file_offset;
            let target_rank = current_rank + direction;
            
            if target_file >= 0 && target_file < 8 && target_rank >= 0 && target_rank < 8 {
                let target_square = Square::new(target_file as u8, target_rank as u8).unwrap();
                
                // Normal capture
                if let Some((_, target_color)) = board.get_piece(target_square) {
                    if target_color != color {
                        if target_rank as u8 == promotion_rank {
                            // Promotion captures
                            moves.push(Move::new_promotion(square, target_square, Piece::Queen));
                            moves.push(Move::new_promotion(square, target_square, Piece::Rook));
                            moves.push(Move::new_promotion(square, target_square, Piece::Bishop));
                            moves.push(Move::new_promotion(square, target_square, Piece::Knight));
                        } else {
                            moves.push(Move::new(square, target_square));
                        }
                    }
                }
                
                // En passant
                if let Some(ep_square) = board.en_passant() {
                    if ep_square == target_square {
                        moves.push(Move::new(square, target_square));
                    }
                }
            }
        }

        moves
    }

    fn generate_knight_moves(board: &Board, square: Square, color: Color) -> Vec<Move> {
        let mut moves = Vec::new();
        let knight_moves = [
            (-2, -1), (-2, 1), (-1, -2), (-1, 2),
            (1, -2), (1, 2), (2, -1), (2, 1),
        ];

        let current_file = square.file() as i8;
        let current_rank = square.rank() as i8;

        for &(dx, dy) in &knight_moves {
            let target_file = current_file + dx;
            let target_rank = current_rank + dy;
            
            if target_file >= 0 && target_file < 8 && target_rank >= 0 && target_rank < 8 {
                let target_square = Square::new(target_file as u8, target_rank as u8).unwrap();
                if let Some((_, target_color)) = board.get_piece(target_square) {
                    if target_color != color {
                        moves.push(Move::new(square, target_square));
                    }
                } else {
                    moves.push(Move::new(square, target_square));
                }
            }
        }

        moves
    }

    fn generate_bishop_moves(board: &Board, square: Square, color: Color) -> Vec<Move> {
        Self::generate_sliding_moves(board, square, color, &[(1, 1), (1, -1), (-1, 1), (-1, -1)])
    }

    fn generate_rook_moves(board: &Board, square: Square, color: Color) -> Vec<Move> {
        Self::generate_sliding_moves(board, square, color, &[(1, 0), (-1, 0), (0, 1), (0, -1)])
    }

    fn generate_queen_moves(board: &Board, square: Square, color: Color) -> Vec<Move> {
        Self::generate_sliding_moves(board, square, color, &[
            (1, 0), (-1, 0), (0, 1), (0, -1),
            (1, 1), (1, -1), (-1, 1), (-1, -1)
        ])
    }

    fn generate_sliding_moves(board: &Board, square: Square, color: Color, directions: &[(i8, i8)]) -> Vec<Move> {
        let mut moves = Vec::new();
        let current_file = square.file() as i8;
        let current_rank = square.rank() as i8;

        for &(dx, dy) in directions {
            let mut file = current_file + dx;
            let mut rank = current_rank + dy;
            
            while file >= 0 && file < 8 && rank >= 0 && rank < 8 {
                let target_square = Square::new(file as u8, rank as u8).unwrap();
                
                if let Some((_, target_color)) = board.get_piece(target_square) {
                    if target_color != color {
                        moves.push(Move::new(square, target_square));
                    }
                    break; // Stop at first piece
                } else {
                    moves.push(Move::new(square, target_square));
                }
                
                file += dx;
                rank += dy;
            }
        }

        moves
    }

    fn generate_king_moves(board: &Board, square: Square, color: Color) -> Vec<Move> {
        let mut moves = Vec::new();
        let king_moves = [
            (-1, -1), (-1, 0), (-1, 1),
            (0, -1),          (0, 1),
            (1, -1),  (1, 0), (1, 1),
        ];

        let current_file = square.file() as i8;
        let current_rank = square.rank() as i8;
        let castling_rights = board.castling_rights();

        // Normal moves
        for &(dx, dy) in &king_moves {
            let target_file = current_file + dx;
            let target_rank = current_rank + dy;
            
            if target_file >= 0 && target_file < 8 && target_rank >= 0 && target_rank < 8 {
                let target_square = Square::new(target_file as u8, target_rank as u8).unwrap();
                if let Some((_, target_color)) = board.get_piece(target_square) {
                    if target_color != color {
                        moves.push(Move::new(square, target_square));
                    }
                } else {
                    moves.push(Move::new(square, target_square));
                }
            }
        }

        // Castling
        if color == Color::White {
            // Kingside castling
            if castling_rights.white_kingside {
                // Check if squares between king and rook are empty
                let f1 = Square::new(5, 0).unwrap();
                let g1 = Square::new(6, 0).unwrap();
                
                if board.get_piece(f1).is_none() && board.get_piece(g1).is_none() {
                    // Check if king is not in check and doesn't move through check
                    if !board.is_in_check(color) {
                        let mut test_board = board.clone();
                        test_board.set_piece(f1, Some((Piece::King, color)));
                        test_board.set_piece(square, None);
                        if !test_board.is_in_check(color) {
                            moves.push(Move::new(square, g1));
                        }
                    }
                }
            }
            
            // Queenside castling
            if castling_rights.white_queenside {
                let b1 = Square::new(1, 0).unwrap();
                let c1 = Square::new(2, 0).unwrap();
                let d1 = Square::new(3, 0).unwrap();
                
                if board.get_piece(b1).is_none() && board.get_piece(c1).is_none() && board.get_piece(d1).is_none() {
                    if !board.is_in_check(color) {
                        let mut test_board = board.clone();
                        test_board.set_piece(d1, Some((Piece::King, color)));
                        test_board.set_piece(square, None);
                        if !test_board.is_in_check(color) {
                            moves.push(Move::new(square, c1));
                        }
                    }
                }
            }
        } else {
            // Black castling
            // Kingside castling
            if castling_rights.black_kingside {
                let f8 = Square::new(5, 7).unwrap();
                let g8 = Square::new(6, 7).unwrap();
                
                if board.get_piece(f8).is_none() && board.get_piece(g8).is_none() {
                    if !board.is_in_check(color) {
                        let mut test_board = board.clone();
                        test_board.set_piece(f8, Some((Piece::King, color)));
                        test_board.set_piece(square, None);
                        if !test_board.is_in_check(color) {
                            moves.push(Move::new(square, g8));
                        }
                    }
                }
            }
            
            // Queenside castling
            if castling_rights.black_queenside {
                let b8 = Square::new(1, 7).unwrap();
                let c8 = Square::new(2, 7).unwrap();
                let d8 = Square::new(3, 7).unwrap();
                
                if board.get_piece(b8).is_none() && board.get_piece(c8).is_none() && board.get_piece(d8).is_none() {
                    if !board.is_in_check(color) {
                        let mut test_board = board.clone();
                        test_board.set_piece(d8, Some((Piece::King, color)));
                        test_board.set_piece(square, None);
                        if !test_board.is_in_check(color) {
                            moves.push(Move::new(square, c8));
                        }
                    }
                }
            }
        }

        moves
    }
}