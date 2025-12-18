use std::fmt;

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Piece {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Color {
    White,
    Black,
}

impl Color {
    pub fn opposite(&self) -> Color {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Square(u8);

impl Square {
    pub fn new(file: u8, rank: u8) -> Option<Self> {
        if file < 8 && rank < 8 {
            Some(Square(rank * 8 + file))
        } else {
            None
        }
    }

    pub fn index(&self) -> u8 {
        self.0
    }

    pub fn file(&self) -> u8 {
        self.0 % 8
    }

    pub fn rank(&self) -> u8 {
        self.0 / 8
    }
}

impl fmt::Display for Square {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let file = (b'a' + self.file()) as char;
        let rank = self.rank() + 1;
        write!(f, "{}{}", file, rank)
    }
}

#[derive(Clone)]
pub struct Board {
    pieces: [[Option<(Piece, Color)>; 8]; 8],
    active_color: Color,
    castling_rights: CastlingRights,
    en_passant: Option<Square>,
    halfmove_clock: u32,
    fullmove_number: u32,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct CastlingRights {
    pub white_kingside: bool,
    pub white_queenside: bool,
    pub black_kingside: bool,
    pub black_queenside: bool,
}

impl Board {
    pub fn new() -> Self {
        Board::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap()
    }

    pub fn from_fen(fen: &str) -> Result<Self, &'static str> {
        let mut board = Board {
            pieces: [[None; 8]; 8],
            active_color: Color::White,
            castling_rights: CastlingRights {
                white_kingside: false,
                white_queenside: false,
                black_kingside: false,
                black_queenside: false,
            },
            en_passant: None,
            halfmove_clock: 0,
            fullmove_number: 1,
        };

        let parts: Vec<&str> = fen.split_whitespace().collect();
        if parts.len() < 4 {
            return Err("FEN string too short");
        }

        // Parse piece placement
        let ranks: Vec<&str> = parts[0].split('/').collect();
        if ranks.len() != 8 {
            return Err("Invalid number of ranks in FEN");
        }

        for (rank_idx, rank_str) in ranks.iter().enumerate() {
            let mut file_idx = 0;
            for c in rank_str.chars() {
                if file_idx >= 8 {
                    return Err("Too many files in rank");
                }

                if let Some(digit) = c.to_digit(10) {
                    file_idx += digit as usize;
                } else {
                    let piece = match c.to_ascii_lowercase() {
                        'p' => Piece::Pawn,
                        'n' => Piece::Knight,
                        'b' => Piece::Bishop,
                        'r' => Piece::Rook,
                        'q' => Piece::Queen,
                        'k' => Piece::King,
                        _ => return Err("Invalid piece character"),
                    };
                    let color = if c.is_uppercase() { Color::White } else { Color::Black };
                    board.pieces[7 - rank_idx][file_idx] = Some((piece, color));
                    file_idx += 1;
                }
            }
        }

        // Parse active color
        board.active_color = match parts[1] {
            "w" => Color::White,
            "b" => Color::Black,
            _ => return Err("Invalid active color"),
        };

        // Parse castling rights
        if parts[2] != "-" {
            for c in parts[2].chars() {
                match c {
                    'K' => board.castling_rights.white_kingside = true,
                    'Q' => board.castling_rights.white_queenside = true,
                    'k' => board.castling_rights.black_kingside = true,
                    'q' => board.castling_rights.black_queenside = true,
                    _ => return Err("Invalid castling right"),
                }
            }
        }

        // Parse en passant
        if parts[3] != "-" {
            let chars: Vec<char> = parts[3].chars().collect();
            if chars.len() == 2 {
                let file = (chars[0] as u8) - b'a';
                let rank = (chars[1] as u8) - b'1';
                if file < 8 && rank < 8 {
                    board.en_passant = Square::new(file, rank);
                }
            }
        }

        // Parse halfmove clock and fullmove number
        if parts.len() >= 5 {
            board.halfmove_clock = parts[4].parse().unwrap_or(0);
        }
        if parts.len() >= 6 {
            board.fullmove_number = parts[5].parse().unwrap_or(1);
        }

        Ok(board)
    }

    pub fn get_piece(&self, square: Square) -> Option<(Piece, Color)> {
        let rank = (square.index() / 8) as usize;
        let file = (square.index() % 8) as usize;
        self.pieces[rank][file]
    }

    pub fn set_piece(&mut self, square: Square, piece: Option<(Piece, Color)>) {
        let rank = (square.index() / 8) as usize;
        let file = (square.index() % 8) as usize;
        self.pieces[rank][file] = piece;
    }

    pub fn active_color(&self) -> Color {
        self.active_color
    }

    pub fn castling_rights(&self) -> CastlingRights {
        self.castling_rights
    }

    pub fn en_passant(&self) -> Option<Square> {
        self.en_passant
    }

    pub fn make_move(&mut self, mv: &crate::movegen::Move) -> bool {
        let from_piece = self.get_piece(mv.from);
        if from_piece.is_none() {
            return false;
        }

        let (piece, color) = from_piece.unwrap();
        
        // Remove captured piece (if any)
        if self.get_piece(mv.to).is_some() {
            self.set_piece(mv.to, None);
        }
        
        // Move piece
        if let Some(promotion) = mv.promotion {
            self.set_piece(mv.to, Some((promotion, color)));
        } else {
            self.set_piece(mv.to, Some((piece, color)));
        }
        self.set_piece(mv.from, None);

        // Handle en passant capture
        if piece == Piece::Pawn {
            if let Some(ep_square) = self.en_passant {
                if mv.to == ep_square {
                    // Remove the pawn that was captured en passant
                    let captured_pawn_rank = match color {
                        Color::White => ep_square.rank() - 1,
                        Color::Black => ep_square.rank() + 1,
                    };
                    if let Some(captured_square) = Square::new(ep_square.file(), captured_pawn_rank) {
                        self.set_piece(captured_square, None);
                    }
                }
            }
        }

        // Update en passant square
        if piece == Piece::Pawn && (mv.to.rank() as i8 - mv.from.rank() as i8).abs() == 2 {
            // Pawn moved two squares
            let ep_rank = match color {
                Color::White => mv.from.rank() + 1,
                Color::Black => mv.from.rank() - 1,
            };
            self.en_passant = Square::new(mv.from.file(), ep_rank);
        } else {
            self.en_passant = None;
        }

        // Update castling rights
        if piece == Piece::King {
            match color {
                Color::White => {
                    self.castling_rights.white_kingside = false;
                    self.castling_rights.white_queenside = false;
                }
                Color::Black => {
                    self.castling_rights.black_kingside = false;
                    self.castling_rights.black_queenside = false;
                }
            }
        } else if piece == Piece::Rook {
            // Rook moved from starting position
            if color == Color::White {
                if mv.from.file() == 0 && mv.from.rank() == 0 {
                    self.castling_rights.white_queenside = false;
                } else if mv.from.file() == 7 && mv.from.rank() == 0 {
                    self.castling_rights.white_kingside = false;
                }
            } else {
                if mv.from.file() == 0 && mv.from.rank() == 7 {
                    self.castling_rights.black_queenside = false;
                } else if mv.from.file() == 7 && mv.from.rank() == 7 {
                    self.castling_rights.black_kingside = false;
                }
            }
        }

        // Handle castling
        if piece == Piece::King && (mv.from.file() as i8 - mv.to.file() as i8).abs() == 2 {
            // Castling move
            let (rook_from_file, rook_to_file) = if mv.to.file() > mv.from.file() {
                // Kingside castling
                (7, 5)
            } else {
                // Queenside castling
                (0, 3)
            };
            
            let rook_from = Square::new(rook_from_file, mv.from.rank()).unwrap();
            let rook_to = Square::new(rook_to_file, mv.from.rank()).unwrap();
            
            if let Some((Piece::Rook, rook_color)) = self.get_piece(rook_from) {
                self.set_piece(rook_to, Some((Piece::Rook, rook_color)));
                self.set_piece(rook_from, None);
            }
        }

        // Update active color
        self.active_color = self.active_color.opposite();

        true
    }

    pub fn is_square_attacked(&self, square: Square, by_color: Color) -> bool {
        use Piece::*;
        
        // Check pawn attacks
        let pawn_dir = match by_color {
            Color::White => 1,
            Color::Black => -1,
        };
        
        let pawn_attacks = [
            (square.file() as i8 - 1, square.rank() as i8 + pawn_dir),
            (square.file() as i8 + 1, square.rank() as i8 + pawn_dir),
        ];
        
        for &(file, rank) in &pawn_attacks {
            if file >= 0 && file < 8 && rank >= 0 && rank < 8 {
                if let Some(sq) = Square::new(file as u8, rank as u8) {
                    if let Some((piece, color)) = self.get_piece(sq) {
                        if piece == Pawn && color == by_color {
                            return true;
                        }
                    }
                }
            }
        }
        
        // Check knight attacks
        let knight_moves = [
            (-2, -1), (-2, 1), (-1, -2), (-1, 2),
            (1, -2), (1, 2), (2, -1), (2, 1),
        ];
        
        for &(dx, dy) in &knight_moves {
            let file = square.file() as i8 + dx;
            let rank = square.rank() as i8 + dy;
            
            if file >= 0 && file < 8 && rank >= 0 && rank < 8 {
                if let Some(sq) = Square::new(file as u8, rank as u8) {
                    if let Some((piece, color)) = self.get_piece(sq) {
                        if piece == Knight && color == by_color {
                            return true;
                        }
                    }
                }
            }
        }
        
        // Check sliding pieces (queen, rook, bishop)
        let directions = [
            (1, 0, vec![Rook, Queen]),   // right
            (-1, 0, vec![Rook, Queen]),  // left
            (0, 1, vec![Rook, Queen]),   // up
            (0, -1, vec![Rook, Queen]),  // down
            (1, 1, vec![Bishop, Queen]), // up-right
            (1, -1, vec![Bishop, Queen]),// down-right
            (-1, 1, vec![Bishop, Queen]),// up-left
            (-1, -1, vec![Bishop, Queen]),// down-left
        ];
        
        for &(dx, dy, ref pieces) in &directions {
            let mut file = square.file() as i8 + dx;
            let mut rank = square.rank() as i8 + dy;
            
            while file >= 0 && file < 8 && rank >= 0 && rank < 8 {
                if let Some(sq) = Square::new(file as u8, rank as u8) {
                    if let Some((piece, color)) = self.get_piece(sq) {
                        if color == by_color && pieces.contains(&piece) {
                            return true;
                        }
                        // Stop if we hit any piece
                        break;
                    }
                }
                file += dx;
                rank += dy;
            }
        }
        
        // Check king attacks (adjacent squares)
        let king_moves = [
            (-1, -1), (-1, 0), (-1, 1),
            (0, -1),          (0, 1),
            (1, -1),  (1, 0), (1, 1),
        ];
        
        for &(dx, dy) in &king_moves {
            let file = square.file() as i8 + dx;
            let rank = square.rank() as i8 + dy;
            
            if file >= 0 && file < 8 && rank >= 0 && rank < 8 {
                if let Some(sq) = Square::new(file as u8, rank as u8) {
                    if let Some((piece, color)) = self.get_piece(sq) {
                        if piece == King && color == by_color {
                            return true;
                        }
                    }
                }
            }
        }
        
        false
    }

    pub fn is_in_check(&self, color: Color) -> bool {
        // Find king position
        let mut king_square = None;
        for rank in 0..8 {
            for file in 0..8 {
                if let Some(sq) = Square::new(file as u8, rank as u8) {
                    if let Some((piece, piece_color)) = self.get_piece(sq) {
                        if piece == Piece::King && piece_color == color {
                            king_square = Some(sq);
                            break;
                        }
                    }
                }
            }
            if king_square.is_some() {
                break;
            }
        }
        
        if let Some(king_sq) = king_square {
            self.is_square_attacked(king_sq, color.opposite())
        } else {
            false // No king on board (shouldn't happen in valid positions)
        }
    }
}

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for rank in (0..8).rev() {
            write!(f, "{} ", rank + 1)?;
            for file in 0..8 {
                let ch = match self.pieces[rank][file] {
                    Some((Piece::Pawn, Color::White)) => '♙',
                    Some((Piece::Knight, Color::White)) => '♘',
                    Some((Piece::Bishop, Color::White)) => '♗',
                    Some((Piece::Rook, Color::White)) => '♖',
                    Some((Piece::Queen, Color::White)) => '♕',
                    Some((Piece::King, Color::White)) => '♔',
                    Some((Piece::Pawn, Color::Black)) => '♟',
                    Some((Piece::Knight, Color::Black)) => '♞',
                    Some((Piece::Bishop, Color::Black)) => '♝',
                    Some((Piece::Rook, Color::Black)) => '♜',
                    Some((Piece::Queen, Color::Black)) => '♛',
                    Some((Piece::King, Color::Black)) => '♚',
                    None => '.',
                };
                write!(f, " {} ", ch)?;
            }
            writeln!(f)?;
        }
        writeln!(f, "   a  b  c  d  e  f  g  h")?;
        
        write!(f, "Active color: ")?;
        match self.active_color {
            Color::White => writeln!(f, "White")?,
            Color::Black => writeln!(f, "Black")?,
        }
        
        Ok(())
    }
}