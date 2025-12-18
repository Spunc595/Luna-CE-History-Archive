use std::io::{self, BufRead};
use crate::board::Board;
use crate::search::iterative_deepening;
use crate::movegen::MoveGenerator;

pub struct UciHandler {
    board: Board,
}

impl UciHandler {
    pub fn new() -> Self {
        UciHandler {
            board: Board::new(),
        }
    }

    pub fn run(&mut self) {
        let stdin = io::stdin();
        
        println!("RustChess Engine v1.0");
        println!("Type 'uci' to start UCI protocol");
        
        for line in stdin.lock().lines() {
            let line = line.unwrap_or_default();
            self.process_command(&line);
        }
    }

    fn process_command(&mut self, command: &str) {
        let parts: Vec<&str> = command.split_whitespace().collect();
        
        match parts.get(0) {
            Some(&"uci") => self.handle_uci(),
            Some(&"isready") => println!("readyok"),
            Some(&"ucinewgame") => self.board = Board::new(),
            Some(&"position") => self.handle_position(&parts),
            Some(&"go") => self.handle_go(),
            Some(&"stop") => (),
            Some(&"quit") => std::process::exit(0),
            Some(&"print") => println!("{}", self.board),
            Some(&"moves") => {
                let moves = MoveGenerator::generate_legal_moves(&self.board);
                println!("Legal moves ({}):", moves.len());
                for mv in moves {
                    println!("  {}", mv);
                }
            }
            _ => println!("Unknown command: {}", command),
        }
    }

    fn handle_uci(&self) {
        println!("id name RustChess Engine v1.0");
        println!("id author AI Assistant");
        println!("uciok");
    }

    fn handle_position(&mut self, parts: &[&str]) {
        if parts.len() < 2 {
            return;
        }
        
        if parts[1] == "startpos" {
            self.board = Board::new();
            
            if parts.len() > 2 && parts[2] == "moves" {
                for &mv_str in &parts[3..] {
                    if let Some(mv) = self.parse_move(mv_str) {
                        self.board.make_move(&mv);
                    }
                }
            }
        } else if parts[1] == "fen" && parts.len() >= 8 {
            let fen = parts[2..8].join(" ");
            if let Ok(board) = Board::from_fen(&fen) {
                self.board = board;
                
                if parts.len() > 8 && parts[8] == "moves" {
                    for &mv_str in &parts[9..] {
                        if let Some(mv) = self.parse_move(mv_str) {
                            self.board.make_move(&mv);
                        }
                    }
                }
            }
        }
    }

    fn handle_go(&self) {
        let depth = 3;
        let time_limit_ms = 5000;
        
        let (best_move, score, nodes) = iterative_deepening(&self.board, depth, time_limit_ms);
        
        if let Some(best_move) = best_move {
            println!("bestmove {}", best_move);
        } else {
            println!("bestmove 0000");
        }
    }

    fn parse_move(&self, move_str: &str) -> Option<crate::movegen::Move> {
        if move_str.len() < 4 {
            return None;
        }
        
        let chars: Vec<char> = move_str.chars().collect();
        let from_file = (chars[0] as u8) - b'a';
        let from_rank = (chars[1] as u8) - b'1';
        let to_file = (chars[2] as u8) - b'a';
        let to_rank = (chars[3] as u8) - b'1';
        
        if from_file >= 8 || from_rank >= 8 || to_file >= 8 || to_rank >= 8 {
            return None;
        }
        
        let from = crate::board::Square::new(from_file, from_rank)?;
        let to = crate::board::Square::new(to_file, to_rank)?;
        
        if chars.len() >= 5 {
            let promotion = match chars[4] {
                'q' => Some(crate::board::Piece::Queen),
                'r' => Some(crate::board::Piece::Rook),
                'b' => Some(crate::board::Piece::Bishop),
                'n' => Some(crate::board::Piece::Knight),
                _ => None,
            };
            
            if let Some(promotion_piece) = promotion {
                return Some(crate::movegen::Move::new_promotion(from, to, promotion_piece));
            }
        }
        
        Some(crate::movegen::Move::new(from, to))
    }
}