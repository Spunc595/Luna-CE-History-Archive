// main.rs - Rust Chess Engine
use std::io::{self, BufRead, Write};
use std::time::{Instant, Duration};
use rand::random;
use lazy_static::lazy_static;

// ==================== ENUM E STRUCT BASE ====================
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

    pub fn from_index(index: u8) -> Option<Self> {
        if index < 64 {
            Some(Square(index))
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

#[derive(Copy, Clone, PartialEq)]
pub struct CastlingRights {
    pub white_kingside: bool,
    pub white_queenside: bool,
    pub black_kingside: bool,
    pub black_queenside: bool,
}

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

// ==================== ZOBRIST HASHING ====================
struct ZobristTable {
    pieces: [[[u64; 64]; 6]; 2], // [color][piece][square]
    black_to_move: u64,
    castling: [u64; 4],
    en_passant: [u64; 8],
}

impl ZobristTable {
    fn new() -> Self {
        let mut z = ZobristTable {
            pieces: [[[0; 64]; 6]; 2],
            black_to_move: 0,
            castling: [0; 4],
            en_passant: [0; 8],
        };
        
        for color in 0..2 {
            for piece in 0..6 {
                for square in 0..64 {
                    z.pieces[color][piece][square] = random();
                }
            }
        }
        
        z.black_to_move = random();
        
        for i in 0..4 {
            z.castling[i] = random();
        }
        
        for i in 0..8 {
            z.en_passant[i] = random();
        }
        
        z
    }
}

lazy_static! {
    static ref ZOBRIST: ZobristTable = ZobristTable::new();
}

// ==================== BOARD ====================
#[derive(Clone)]
pub struct Board {
    pieces: [[Option<(Piece, Color)>; 8]; 8],
    active_color: Color,
    castling_rights: CastlingRights,
    en_passant: Option<Square>,
    halfmove_clock: u32,
    fullmove_number: u32,
    hash: u64,
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
            hash: 0,
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
                board.en_passant = Square::new(file, rank);
            }
        }

        // Parse halfmove clock and fullmove number
        if parts.len() >= 5 {
            board.halfmove_clock = parts[4].parse().unwrap_or(0);
        }
        if parts.len() >= 6 {
            board.fullmove_number = parts[5].parse().unwrap_or(1);
        }

        board.hash = board.calculate_hash();
        Ok(board)
    }

    fn calculate_hash(&self) -> u64 {
        let mut hash = 0;

        for rank in 0..8 {
            for file in 0..8 {
                if let Some((piece, color)) = self.pieces[rank][file] {
                    let piece_index = match piece {
                        Piece::Pawn => 0,
                        Piece::Knight => 1,
                        Piece::Bishop => 2,
                        Piece::Rook => 3,
                        Piece::Queen => 4,
                        Piece::King => 5,
                    };
                    let color_index = match color {
                        Color::White => 0,
                        Color::Black => 1,
                    };
                    let square = rank * 8 + file;
                    hash ^= ZOBRIST.pieces[color_index][piece_index][square];
                }
            }
        }

        if self.active_color == Color::Black {
            hash ^= ZOBRIST.black_to_move;
        }

        if self.castling_rights.white_kingside {
            hash ^= ZOBRIST.castling[0];
        }
        if self.castling_rights.white_queenside {
            hash ^= ZOBRIST.castling[1];
        }
        if self.castling_rights.black_kingside {
            hash ^= ZOBRIST.castling[2];
        }
        if self.castling_rights.black_queenside {
            hash ^= ZOBRIST.castling[3];
        }

        if let Some(ep) = self.en_passant {
            hash ^= ZOBRIST.en_passant[ep.file() as usize];
        }

        hash
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

    pub fn hash(&self) -> u64 {
        self.hash
    }

    pub fn print(&self) {
        println!("  +-----------------+");
        for rank in (0..8).rev() {
            print!("{} |", rank + 1);
            for file in 0..8 {
                let c = match self.pieces[rank][file] {
                    Some((Piece::Pawn, Color::White)) => "P",
                    Some((Piece::Knight, Color::White)) => "N",
                    Some((Piece::Bishop, Color::White)) => "B",
                    Some((Piece::Rook, Color::White)) => "R",
                    Some((Piece::Queen, Color::White)) => "Q",
                    Some((Piece::King, Color::White)) => "K",
                    Some((Piece::Pawn, Color::Black)) => "p",
                    Some((Piece::Knight, Color::Black)) => "n",
                    Some((Piece::Bishop, Color::Black)) => "b",
                    Some((Piece::Rook, Color::Black)) => "r",
                    Some((Piece::Queen, Color::Black)) => "q",
                    Some((Piece::King, Color::Black)) => "k",
                    None => ".",
                };
                print!(" {}", c);
            }
            println!(" |");
        }
        println!("  +-----------------+");
        println!("    a b c d e f g h");
        println!("Active color: {:?}", self.active_color);
    }
}

// ==================== MOVE GENERATION ====================
pub struct MoveGenerator;

impl MoveGenerator {
    pub fn generate_legal_moves(board: &Board) -> Vec<Move> {
        let mut moves = Vec::new();
        let pseudo_moves = Self::generate_pseudo_legal_moves(board);
        
        for mv in pseudo_moves {
            let mut new_board = board.clone();
            if Self::make_move(&mut new_board, &mv) {
                if !Self::is_in_check(&new_board, board.active_color()) {
                    moves.push(mv);
                }
            }
        }
        
        moves
    }

    pub fn generate_pseudo_legal_moves(board: &Board) -> Vec<Move> {
        let mut moves = Vec::new();
        
        for rank in 0..8 {
            for file in 0..8 {
                if let Some(square) = Square::from_index((rank * 8 + file) as u8) {
                    if let Some((piece, color)) = board.get_piece(square) {
                        if color == board.active_color() {
                            match piece {
                                Piece::Pawn => Self::generate_pawn_moves(board, square, color, &mut moves),
                                Piece::Knight => Self::generate_knight_moves(board, square, color, &mut moves),
                                Piece::Bishop => Self::generate_bishop_moves(board, square, color, &mut moves),
                                Piece::Rook => Self::generate_rook_moves(board, square, color, &mut moves),
                                Piece::Queen => Self::generate_queen_moves(board, square, color, &mut moves),
                                Piece::King => Self::generate_king_moves(board, square, color, &mut moves),
                            }
                        }
                    }
                }
            }
        }
        
        moves
    }

    fn generate_pawn_moves(board: &Board, square: Square, color: Color, moves: &mut Vec<Move>) {
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

        // Single push
        if let Some(target) = Square::new(current_file as u8, (current_rank + direction) as u8) {
            if board.get_piece(target).is_none() {
                if target.rank() as i8 == promotion_rank as i8 {
                    let pieces = [Piece::Queen, Piece::Rook, Piece::Bishop, Piece::Knight];
                    for &piece in &pieces {
                        moves.push(Move::new_promotion(square, target, piece));
                    }
                } else {
                    moves.push(Move::new(square, target));
                }

                // Double push from starting rank
                if current_rank == start_rank as i8 {
                    if let Some(target2) = Square::new(current_file as u8, (current_rank + 2 * direction) as u8) {
                        if board.get_piece(target2).is_none() {
                            moves.push(Move::new(square, target2));
                        }
                    }
                }
            }
        }

        // Captures
        for file_offset in [-1, 1] {
            let target_file = current_file + file_offset;
            if target_file >= 0 && target_file < 8 {
                if let Some(target) = Square::new(target_file as u8, (current_rank + direction) as u8) {
                    // Normal capture
                    if let Some((_, target_color)) = board.get_piece(target) {
                        if target_color != color {
                            if target.rank() as i8 == promotion_rank as i8 {
                                let pieces = [Piece::Queen, Piece::Rook, Piece::Bishop, Piece::Knight];
                                for &piece in &pieces {
                                    moves.push(Move::new_promotion(square, target, piece));
                                }
                            } else {
                                moves.push(Move::new(square, target));
                            }
                        }
                    }
                    // En passant
                    if let Some(ep_square) = board.en_passant {
                        if ep_square.index() == target.index() {
                            moves.push(Move::new(square, target));
                        }
                    }
                }
            }
        }
    }

    fn generate_knight_moves(board: &Board, square: Square, color: Color, moves: &mut Vec<Move>) {
        let offsets = [
            (2, 1), (2, -1), (-2, 1), (-2, -1),
            (1, 2), (1, -2), (-1, 2), (-1, -2),
        ];

        let current_rank = square.rank() as i8;
        let current_file = square.file() as i8;

        for &(rank_offset, file_offset) in &offsets {
            let target_rank = current_rank + rank_offset;
            let target_file = current_file + file_offset;

            if target_rank >= 0 && target_rank < 8 && target_file >= 0 && target_file < 8 {
                if let Some(target) = Square::new(target_file as u8, target_rank as u8) {
                    if let Some((_, target_color)) = board.get_piece(target) {
                        if target_color != color {
                            moves.push(Move::new(square, target));
                        }
                    } else {
                        moves.push(Move::new(square, target));
                    }
                }
            }
        }
    }

    fn generate_sliding_moves(
        board: &Board,
        square: Square,
        color: Color,
        directions: &[(i8, i8)],
        moves: &mut Vec<Move>
    ) {
        let current_rank = square.rank() as i8;
        let current_file = square.file() as i8;

        for &(rank_dir, file_dir) in directions {
            let mut rank = current_rank + rank_dir;
            let mut file = current_file + file_dir;

            while rank >= 0 && rank < 8 && file >= 0 && file < 8 {
                if let Some(target) = Square::new(file as u8, rank as u8) {
                    match board.get_piece(target) {
                        Some((_, target_color)) => {
                            if target_color != color {
                                moves.push(Move::new(square, target));
                            }
                            break;
                        }
                        None => {
                            moves.push(Move::new(square, target));
                        }
                    }
                }
                rank += rank_dir;
                file += file_dir;
            }
        }
    }

    fn generate_bishop_moves(board: &Board, square: Square, color: Color, moves: &mut Vec<Move>) {
        let directions = [(1, 1), (1, -1), (-1, 1), (-1, -1)];
        Self::generate_sliding_moves(board, square, color, &directions, moves);
    }

    fn generate_rook_moves(board: &Board, square: Square, color: Color, moves: &mut Vec<Move>) {
        let directions = [(1, 0), (-1, 0), (0, 1), (0, -1)];
        Self::generate_sliding_moves(board, square, color, &directions, moves);
    }

    fn generate_queen_moves(board: &Board, square: Square, color: Color, moves: &mut Vec<Move>) {
        let directions = [
            (1, 1), (1, -1), (-1, 1), (-1, -1),
            (1, 0), (-1, 0), (0, 1), (0, -1),
        ];
        Self::generate_sliding_moves(board, square, color, &directions, moves);
    }

    fn generate_king_moves(board: &Board, square: Square, color: Color, moves: &mut Vec<Move>) {
        let directions = [
            (1, 1), (1, 0), (1, -1),
            (0, 1), (0, -1),
            (-1, 1), (-1, 0), (-1, -1),
        ];

        let current_rank = square.rank() as i8;
        let current_file = square.file() as i8;

        for &(rank_offset, file_offset) in &directions {
            let target_rank = current_rank + rank_offset;
            let target_file = current_file + file_offset;

            if target_rank >= 0 && target_rank < 8 && target_file >= 0 && target_file < 8 {
                if let Some(target) = Square::new(target_file as u8, target_rank as u8) {
                    if let Some((_, target_color)) = board.get_piece(target) {
                        if target_color != color {
                            moves.push(Move::new(square, target));
                        }
                    } else {
                        moves.push(Move::new(square, target));
                    }
                }
            }
        }
    }

    pub fn make_move(board: &mut Board, mv: &Move) -> bool {
        let Some((piece, color)) = board.get_piece(mv.from) else {
            return false;
        };

        // Update hash for piece removal
        board.hash ^= Self::get_piece_hash(piece, color, mv.from.index() as usize);
        
        // Handle captures
        if let Some((captured_piece, captured_color)) = board.get_piece(mv.to) {
            board.hash ^= Self::get_piece_hash(captured_piece, captured_color, mv.to.index() as usize);
        }

        // Move the piece
        let moved_piece = match mv.promotion {
            Some(promotion_piece) => promotion_piece,
            None => piece,
        };
        
        board.set_piece(mv.to, Some((moved_piece, color)));
        board.set_piece(mv.from, None);
        
        // Update hash for piece placement
        board.hash ^= Self::get_piece_hash(moved_piece, color, mv.to.index() as usize);

        // Update active color in hash
        board.hash ^= ZOBRIST.black_to_move;
        board.active_color = match board.active_color {
            Color::White => Color::Black,
            Color::Black => Color::White,
        };

        true
    }

    fn get_piece_hash(piece: Piece, color: Color, square: usize) -> u64 {
        let piece_index = match piece {
            Piece::Pawn => 0,
            Piece::Knight => 1,
            Piece::Bishop => 2,
            Piece::Rook => 3,
            Piece::Queen => 4,
            Piece::King => 5,
        };
        let color_index = match color {
            Color::White => 0,
            Color::Black => 1,
        };
        ZOBRIST.pieces[color_index][piece_index][square]
    }

    pub fn is_in_check(board: &Board, color: Color) -> bool {
        // Find king position
        let mut king_square = None;
        for rank in 0..8 {
            for file in 0..8 {
                if let Some(square) = Square::from_index((rank * 8 + file) as u8) {
                    if let Some((Piece::King, piece_color)) = board.get_piece(square) {
                        if piece_color == color {
                            king_square = Some(square);
                            break;
                        }
                    }
                }
            }
        }

        let Some(king_square) = king_square else {
            return false;
        };

        let opponent_color = match color {
            Color::White => Color::Black,
            Color::Black => Color::White,
        };

        // Check for knights
        let knight_offsets = [
            (2, 1), (2, -1), (-2, 1), (-2, -1),
            (1, 2), (1, -2), (-1, 2), (-1, -2),
        ];

        let king_rank = king_square.rank() as i8;
        let king_file = king_square.file() as i8;

        for &(rank_offset, file_offset) in &knight_offsets {
            let target_rank = king_rank + rank_offset;
            let target_file = king_file + file_offset;

            if target_rank >= 0 && target_rank < 8 && target_file >= 0 && target_file < 8 {
                if let Some(target) = Square::new(target_file as u8, target_rank as u8) {
                    if let Some((Piece::Knight, piece_color)) = board.get_piece(target) {
                        if piece_color == opponent_color {
                            return true;
                        }
                    }
                }
            }
        }

        // Check for sliding pieces
        let directions = [
            (1, 1, vec![Piece::Bishop, Piece::Queen]),
            (1, -1, vec![Piece::Bishop, Piece::Queen]),
            (-1, 1, vec![Piece::Bishop, Piece::Queen]),
            (-1, -1, vec![Piece::Bishop, Piece::Queen]),
            (1, 0, vec![Piece::Rook, Piece::Queen]),
            (-1, 0, vec![Piece::Rook, Piece::Queen]),
            (0, 1, vec![Piece::Rook, Piece::Queen]),
            (0, -1, vec![Piece::Rook, Piece::Queen]),
        ];

        for &(rank_dir, file_dir, ref pieces) in &directions {
            let mut rank = king_rank + rank_dir;
            let mut file = king_file + file_dir;

            while rank >= 0 && rank < 8 && file >= 0 && file < 8 {
                if let Some(target) = Square::new(file as u8, rank as u8) {
                    if let Some((piece, piece_color)) = board.get_piece(target) {
                        if piece_color == opponent_color && pieces.contains(&piece) {
                            return true;
                        }
                        break;
                    }
                }
                rank += rank_dir;
                file += file_dir;
            }
        }

        // Check for pawn attacks
        let pawn_direction = match opponent_color {
            Color::White => 1,
            Color::Black => -1,
        };

        for file_offset in [-1, 1] {
            let target_rank = king_rank - pawn_direction;
            let target_file = king_file + file_offset;

            if target_rank >= 0 && target_rank < 8 && target_file >= 0 && target_file < 8 {
                if let Some(target) = Square::new(target_file as u8, target_rank as u8) {
                    if let Some((Piece::Pawn, piece_color)) = board.get_piece(target) {
                        if piece_color == opponent_color {
                            return true;
                        }
                    }
                }
            }
        }

        // Check for king
        let king_offsets = [
            (1, 1), (1, 0), (1, -1),
            (0, 1), (0, -1),
            (-1, 1), (-1, 0), (-1, -1),
        ];

        for &(rank_offset, file_offset) in &king_offsets {
            let target_rank = king_rank + rank_offset;
            let target_file = king_file + file_offset;

            if target_rank >= 0 && target_rank < 8 && target_file >= 0 && target_file < 8 {
                if let Some(target) = Square::new(target_file as u8, target_rank as u8) {
                    if let Some((Piece::King, piece_color)) = board.get_piece(target) {
                        if piece_color == opponent_color {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }
}

// ==================== EVALUATION ====================
pub struct Evaluation;

impl Evaluation {
    pub fn evaluate(board: &Board) -> i32 {
        let mut score = 0;
        
        // Material evaluation
        score += Self::material_score(board);
        
        // Piece-square tables
        score += Self::piece_square_score(board);
        
        match board.active_color() {
            Color::White => score,
            Color::Black => -score,
        }
    }

    fn material_score(board: &Board) -> i32 {
        let mut score = 0;
        
        for rank in 0..8 {
            for file in 0..8 {
                if let Some(square) = Square::from_index((rank * 8 + file) as u8) {
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
        
        let pawn_table = [
            [0,   0,   0,   0,   0,   0,  0,   0],
            [50,  50,  50,  50,  50,  50, 50,  50],
            [10,  10,  20,  30,  30,  20, 10,  10],
            [5,   5,  10,  25,  25,  10,  5,   5],
            [0,   0,   0,  20,  20,   0,  0,   0],
            [5,  -5, -10,   0,   0, -10, -5,   5],
            [5,  10,  10, -20, -20,  10, 10,   5],
            [0,   0,   0,   0,   0,   0,  0,   0],
        ];
        
        let knight_table = [
            [-50, -40, -30, -30, -30, -30, -40, -50],
            [-40, -20,   0,   0,   0,   0, -20, -40],
            [-30,   0,  10,  15,  15,  10,   0, -30],
            [-30,   5,  15,  20,  20,  15,   5, -30],
            [-30,   0,  15,  20,  20,  15,   0, -30],
            [-30,   5,  10,  15,  15,  10,   5, -30],
            [-40, -20,   0,   5,   5,   0, -20, -40],
            [-50, -40, -30, -30, -30, -30, -40, -50],
        ];

        for rank in 0..8 {
            for file in 0..8 {
                if let Some(square) = Square::from_index((rank * 8 + file) as u8) {
                    if let Some((piece, color)) = board.get_piece(square) {
                        let table_value = match piece {
                            Piece::Pawn => pawn_table[rank][file],
                            Piece::Knight => knight_table[rank][file],
                            _ => 0,
                        };
                        
                        match color {
                            Color::White => score += table_value,
                            Color::Black => score -= table_value,
                        }
                    }
                }
            }
        }
        
        score
    }
}

// ==================== TRANSPOSITION TABLE ====================
#[derive(Clone, Copy)]
pub enum TTBound {
    Exact,
    Lower,
    Upper,
}

pub struct TTEntry {
    pub hash: u64,
    pub depth: u32,
    pub value: i32,
    pub bound: TTBound,
    pub best_move: Option<Move>,
}

pub struct TranspositionTable {
    entries: Vec<Option<TTEntry>>,
    size: usize,
}

impl TranspositionTable {
    pub fn new(mb_size: usize) -> Self {
        let size = mb_size * 1024 * 1024 / std::mem::size_of::<Option<TTEntry>>();
        TranspositionTable {
            entries: vec![None; size],
            size,
        }
    }

    pub fn store(&mut self, entry: TTEntry) {
        let index = (entry.hash as usize) % self.size;
        self.entries[index] = Some(entry);
    }

    pub fn get(&self, hash: u64) -> Option<&TTEntry> {
        let index = (hash as usize) % self.size;
        if let Some(entry) = &self.entries[index] {
            if entry.hash == hash {
                return Some(entry);
            }
        }
        None
    }

    pub fn clear(&mut self) {
        for entry in self.entries.iter_mut() {
            *entry = None;
        }
    }
}

// ==================== SEARCH ====================
pub struct Search {
    pub nodes_searched: u64,
    pub start_time: Instant,
    pub time_limit: Option<Duration>,
    pub depth_limit: Option<u32>,
    pub tt: TranspositionTable,
}

impl Search {
    pub fn new() -> Self {
        Search {
            nodes_searched: 0,
            start_time: Instant::now(),
            time_limit: None,
            depth_limit: None,
            tt: TranspositionTable::new(128),
        }
    }

    pub fn best_move(&mut self, board: &Board, depth: u32) -> Option<Move> {
        self.nodes_searched = 0;
        self.start_time = Instant::now();
        
        let mut best_move = None;
        let mut best_value = -i32::MAX;
        
        let moves = MoveGenerator::generate_legal_moves(board);
        
        if let Some(entry) = self.tt.get(board.hash()) {
            if let Some(mv) = entry.best_move {
                if moves.iter().any(|&m| m.from.index() == mv.from.index() && m.to.index() == mv.to.index()) {
                    best_move = Some(mv);
                }
            }
        }
        
        for mv in moves {
            let mut new_board = board.clone();
            if MoveGenerator::make_move(&mut new_board, &mv) {
                let value = -self.negamax(&new_board, depth - 1, -i32::MAX, i32::MAX);
                
                if value > best_value {
                    best_value = value;
                    best_move = Some(mv);
                }
            }
        }
        
        best_move
    }

    fn negamax(&mut self, board: &Board, depth: u32, alpha: i32, beta: i32) -> i32 {
        self.nodes_searched += 1;
        
        if let Some(time_limit) = self.time_limit {
            if self.start_time.elapsed() > time_limit {
                return Evaluation::evaluate(board);
            }
        }
        
        let hash = board.hash();
        if let Some(entry) = self.tt.get(hash) {
            if entry.depth >= depth {
                match entry.bound {
                    TTBound::Exact => return entry.value,
                    TTBound::Lower => return entry.value.max(alpha),
                    TTBound::Upper => return entry.value.min(beta),
                }
            }
        }
        
        if depth == 0 {
            return Self::quiescence_search(board, 3, alpha, beta);
        }
        
        let moves = MoveGenerator::generate_legal_moves(board);
        if moves.is_empty() {
            if MoveGenerator::is_in_check(board, board.active_color()) {
                return -i32::MAX + 1;
            } else {
                return 0;
            }
        }
        
        let mut alpha = alpha;
        let mut best_value = -i32::MAX;
        let mut best_move: Option<Move> = None;
        
        for mv in moves {
            let mut new_board = board.clone();
            if MoveGenerator::make_move(&mut new_board, &mv) {
                let value = -self.negamax(&new_board, depth - 1, -beta, -alpha);
                
                if value > best_value {
                    best_value = value;
                    best_move = Some(mv);
                }
                
                alpha = alpha.max(value);
                if alpha >= beta {
                    break;
                }
            }
        }
        
        let bound = if best_value <= alpha {
            TTBound::Upper
        } else if best_value >= beta {
            TTBound::Lower
        } else {
            TTBound::Exact
        };
        
        let entry = TTEntry {
            hash,
            depth,
            value: best_value,
            bound,
            best_move,
        };
        self.tt.store(entry);
        
        best_value
    }

    fn quiescence_search(board: &Board, depth: u32, alpha: i32, beta: i32) -> i32 {
        let stand_pat = Evaluation::evaluate(board);
        
        if depth == 0 {
            return stand_pat;
        }
        
        if stand_pat >= beta {
            return beta;
        }
        
        let mut alpha = alpha.max(stand_pat);
        
        let all_moves = MoveGenerator::generate_pseudo_legal_moves(board);
        let captures: Vec<Move> = all_moves.into_iter()
            .filter(|mv| {
                let (from_piece, from_color) = board.get_piece(mv.from).unwrap();
                if let Some((to_piece, to_color)) = board.get_piece(mv.to) {
                    to_color != from_color
                } else {
                    false
                }
            })
            .collect();
        
        for mv in captures {
            let mut new_board = board.clone();
            if MoveGenerator::make_move(&mut new_board, &mv) {
                let value = -Self::quiescence_search(&new_board, depth - 1, -beta, -alpha);
                
                if value >= beta {
                    return beta;
                }
                
                alpha = alpha.max(value);
            }
        }
        
        alpha
    }

    pub fn iterative_deepening(&mut self, board: &Board, max_depth: u32) -> Option<Move> {
        let mut best_move = None;
        
        for depth in 1..=max_depth {
            if let Some(time_limit) = self.time_limit {
                if self.start_time.elapsed() > time_limit {
                    break;
                }
            }
            
            if let Some(mv) = self.best_move(board, depth) {
                best_move = Some(mv);
                
                let elapsed = self.start_time.elapsed();
                let nps = (self.nodes_searched as f64 / elapsed.as_secs_f64()) as u64;
                println!("info depth {} nodes {} time {} nps {}",
                         depth, self.nodes_searched, elapsed.as_millis(), nps);
            }
        }
        
        best_move
    }
}

// ==================== UCI INTERFACE ====================
pub struct UciHandler {
    board: Board,
    search: Search,
}

impl UciHandler {
    pub fn new() -> Self {
        UciHandler {
            board: Board::new(),
            search: Search::new(),
        }
    }

    pub fn run(&mut self) {
        let stdin = io::stdin();
        
        println!("id name RustChess 1.0");
        println!("id author RustChess Engine");
        println!("uciok");
        
        for line in stdin.lock().lines() {
            let line = line.unwrap();
            let tokens: Vec<&str> = line.split_whitespace().collect();
            
            if tokens.is_empty() {
                continue;
            }
            
            match tokens[0] {
                "isready" => {
                    println!("readyok");
                }
                "position" => {
                    self.handle_position(&tokens);
                }
                "go" => {
                    self.handle_go(&tokens);
                }
                "quit" => {
                    break;
                }
                "stop" => {
                    // Stop searching
                }
                "uci" => {
                    println!("id name RustChess 1.0");
                    println!("id author RustChess Engine");
                    println!("uciok");
                }
                "ucinewgame" => {
                    self.board = Board::new();
                    self.search.tt.clear();
                }
                "print" => {
                    self.board.print();
                }
                "moves" => {
                    let moves = MoveGenerator::generate_legal_moves(&self.board);
                    println!("Legal moves: {}", moves.len());
                    for mv in moves {
                        println!("  {:?}", mv);
                    }
                }
                "eval" => {
                    let eval = Evaluation::evaluate(&self.board);
                    println!("Evaluation: {}", eval);
                }
                _ => {
                    // Ignore unknown commands
                }
            }
        }
    }

    fn handle_position(&mut self, tokens: &[&str]) {
        let mut index = 1;
        
        if index >= tokens.len() {
            return;
        }
        
        if tokens[index] == "startpos" {
            self.board = Board::new();
            index += 1;
        } else if tokens[index] == "fen" {
            index += 1;
            let fen_parts = &tokens[index..];
            let fen = fen_parts.join(" ");
            match Board::from_fen(&fen) {
                Ok(board) => self.board = board,
                Err(e) => println!("Error parsing FEN: {}", e),
            }
            index += fen_parts.len();
        }
        
        if index < tokens.len() && tokens[index] == "moves" {
            println!("Note: Move parsing not fully implemented in this version");
        }
    }

    fn handle_go(&mut self, tokens: &[&str]) {
        let mut depth = 6;
        let mut movetime = None;
        
        for i in 1..tokens.len() {
            match tokens[i] {
                "depth" if i + 1 < tokens.len() => {
                    depth = tokens[i + 1].parse().unwrap_or(6);
                }
                "movetime" if i + 1 < tokens.len() => {
                    movetime = Some(Duration::from_millis(tokens[i + 1].parse().unwrap_or(1000)));
                }
                "infinite" => {
                    depth = 100;
                }
                _ => {}
            }
        }
        
        if let Some(time) = movetime {
            self.search.time_limit = Some(time);
        }
        
        if let Some(best_move) = self.search.iterative_deepening(&self.board, depth) {
            // Convert to UCI move format
            let from_file = (b'a' + best_move.from.file()) as char;
            let from_rank = (b'1' + best_move.from.rank()) as char;
            let to_file = (b'a' + best_move.to.file()) as char;
            let to_rank = (b'1' + best_move.to.rank()) as char;
            
            let promotion = match best_move.promotion {
                Some(Piece::Queen) => "q",
                Some(Piece::Rook) => "r",
                Some(Piece::Bishop) => "b",
                Some(Piece::Knight) => "n",
                _ => "",
            };
            
            println!("bestmove {}{}{}{}{}", from_file, from_rank, to_file, to_rank, promotion);
        } else {
            println!("bestmove 0000"); // Null move if no legal moves
        }
    }
}

// ==================== MAIN ====================
fn main() {
    // Crea il file Cargo.toml automaticamente se non esiste
    let cargo_toml = r#"[package]
name = "rust-chess"
version = "0.1.0"
edition = "2021"

[dependencies]
rand = "0.8"
lazy_static = "1.4"
"#;

    // Controlla se siamo in un progetto Cargo
    if !std::path::Path::new("Cargo.toml").exists() {
        println!("Nota: Crea un progetto Cargo con: cargo new rust-chess");
        println!("Poi sostituisci il contenuto di src/main.rs con questo codice");
        println!("Ed aggiungi le dipendenze a Cargo.toml:\n");
        println!("{}", cargo_toml);
        println!("\nAvvio dell'engine in modalità standalone...\n");
    }
    
    println!("RustChess Engine v1.0");
    println!("Comandi disponibili:");
    println!("  uci - Inizia modalità UCI");
    println!("  print - Mostra la scacchiera");
    println!("  moves - Mostra mosse legali");
    println!("  eval - Valuta posizione");
    println!("  quit - Esci");
    
    let mut uci_handler = UciHandler::new();
    
    // Modalità interattiva semplice
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line.unwrap();
        match line.trim() {
            "uci" => {
                uci_handler.run();
                break;
            }
            "print" => {
                uci_handler.board.print();
            }
            "moves" => {
                let moves = MoveGenerator::generate_legal_moves(&uci_handler.board);
                println!("Mosse legali: {}", moves.len());
                for mv in &moves {
                    println!("  Da: ({},{}) A: ({},{})", 
                             mv.from.file(), mv.from.rank(),
                             mv.to.file(), mv.to.rank());
                }
            }
            "eval" => {
                let eval = Evaluation::evaluate(&uci_handler.board);
                println!("Valutazione: {} cp", eval);
            }
            "test" => {
                println!("Test del motore...");
                let moves = MoveGenerator::generate_legal_moves(&uci_handler.board);
                println!("Mosse nella posizione iniziale: {}", moves.len());
                println!("Hash della posizione: {}", uci_handler.board.hash());
            }
            "quit" => {
                break;
            }
            _ => {
                println!("Comando non riconosciuto. Usa: uci, print, moves, eval, test, quit");
            }
        }
    }
}