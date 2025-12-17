const readline = require('readline');

// ============= COSTANTI =============
const PieceType = {
    PAWN: 0, KNIGHT: 1, BISHOP: 2, ROOK: 3, QUEEN: 4, KING: 5, NONE: 6
};

const Color = {
    WHITE: 0, BLACK: 1, NONE: 2
};

const GameResult = {
    ONGOING: 0,
    WHITE_WINS: 1,
    BLACK_WINS: 2,
    DRAW: 3,
    STALEMATE: 4
};

const TTFlag = {
    EXACT: 0,
    LOWERBOUND: 1,
    UPPERBOUND: 2
};

// ============= CLASSI =============
class Move {
    constructor(fromRow, fromCol, toRow, toCol, promotion = PieceType.NONE, isCastling = false, isEnPassant = false) {
        this.fromRow = fromRow;
        this.fromCol = fromCol;
        this.toRow = toRow;
        this.toCol = toCol;
        this.promotion = promotion;
        this.isCastling = isCastling;
        this.isEnPassant = isEnPassant;
        this.isCapture = false;
        this.capturedPiece = null;
    }

    equals(other) {
        return this.fromRow === other.fromRow &&
               this.fromCol === other.fromCol &&
               this.toRow === other.toRow &&
               this.toCol === other.toCol &&
               this.promotion === other.promotion;
    }

    toString() {
        const cols = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h'];
        const from = cols[this.fromCol] + (8 - this.fromRow);
        const to = cols[this.toCol] + (8 - this.toRow);
        
        if (this.isCastling) {
            return this.toCol === 6 ? 'O-O' : 'O-O-O';
        }
        
        let result = from + to;
        
        if (this.promotion !== PieceType.NONE) {
            const promSymbols = ['', 'n', 'b', 'r', 'q', ''];
            result += promSymbols[this.promotion];
        }
        
        return result;
    }

    getSAN(piece, board, isCheck = false, isMate = false) {
        // Controllo di sicurezza per piece null
        if (!piece || piece.type === undefined) {
            return this.toString(); // Fallback alla notazione semplice
        }
        
        const pieceSymbol = piece.type !== PieceType.PAWN ? 
            ['', 'N', 'B', 'R', 'Q', 'K'][piece.type] : '';
        const capture = this.isCapture || this.isEnPassant ? 'x' : '';
        const cols = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h'];
        const destination = cols[this.toCol] + (8 - this.toRow);
        const promotion = this.promotion !== PieceType.NONE ? 
            '=' + ['', 'N', 'B', 'R', 'Q'][this.promotion] : '';
        
        let san = pieceSymbol;
        
        // Per pedoni, aggiungi la colonna in caso di cattura
        if (piece.type === PieceType.PAWN && capture) {
            san += cols[this.fromCol];
        }
        
        san += capture + destination + promotion;
        
        if (isMate) san += '#';
        else if (isCheck) san += '+';
        
        return san;
    }
}

class Piece {
    constructor(type, color) {
        this.type = type;
        this.color = color;
        this.hasMoved = false;
    }

    copy() {
        const newPiece = new Piece(this.type, this.color);
        newPiece.hasMoved = this.hasMoved;
        return newPiece;
    }

    getSymbol() {
        const symbols = ['♙♟', '♘♞', '♗♝', '♖♜', '♕♛', '♔♚'];
        const index = this.color === Color.WHITE ? 0 : 1;
        return symbols[this.type] ? symbols[this.type][index] : '?';
    }

    getFEN() {
        const pieces = 'PNBRQK';
        const pieceChar = pieces[this.type];
        return this.color === Color.WHITE ? pieceChar : pieceChar.toLowerCase();
    }

    getValue() {
        const pieceValues = [100, 320, 330, 500, 900, 20000];
        return pieceValues[this.type] || 0;
    }
}

class TTableEntry {
    constructor(key, depth, score, move, flag) {
        this.key = key;
        this.depth = depth;
        this.score = score;
        this.move = move;
        this.flag = flag;
    }
}

class TranspositionTable {
    constructor(size = 100000) {
        this.table = new Map();
        this.size = size;
    }

    store(key, depth, score, move, flag) {
        if (this.table.size >= this.size) {
            // Rimuovi qualche entry vecchia se la tabella è piena
            const firstKey = this.table.keys().next().value;
            this.table.delete(firstKey);
        }
        this.table.set(key.toString(), new TTableEntry(key, depth, score, move, flag));
    }

    retrieve(key, depth, alpha, beta) {
        const entry = this.table.get(key.toString());
        if (!entry || entry.depth < depth) return null;
        
        if (entry.flag === TTFlag.EXACT) return entry;
        if (entry.flag === TTFlag.LOWERBOUND && entry.score >= beta) return entry;
        if (entry.flag === TTFlag.UPPERBOUND && entry.score <= alpha) return entry;
        
        return null;
    }

    clear() {
        this.table.clear();
    }
}

class OpeningBook {
    constructor() {
        this.openings = {
            // Posizione iniziale
            'rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1': [
                'e2e4', 'd2d4', 'c2c4', 'g1f3', 'b1c3'
            ],
            // Risposta a e4
            'rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1': [
                'e7e5', 'c7c5', 'e7e6', 'c7c6', 'g8f6'
            ],
            // Risposta a d4
            'rnbqkbnr/pppppppp/8/8/3P4/8/PPP1PPPP/RNBQKBNR b KQkq d3 0 1': [
                'd7d5', 'g8f6', 'e7e6', 'c7c5'
            ]
        };
    }

    getMove(fen) {
        const simplifiedFEN = fen.split(' ').slice(0, 4).join(' ');
        const moves = this.openings[simplifiedFEN];
        if (moves && moves.length > 0) {
            return moves[Math.floor(Math.random() * moves.length)];
        }
        return null;
    }
}

// ============= MOTORE SCACCHI COMPLETO =============
class CompleteChessEngine {
    constructor(fen = 'rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1') {
        this.board = Array(8).fill().map(() => Array(8).fill(null));
        this.currentPlayer = Color.WHITE;
        this.moveHistory = [];
        this.moveSANHistory = [];
        this.enPassantTarget = null;
        this.castlingRights = {
            [Color.WHITE]: { kingside: true, queenside: true },
            [Color.BLACK]: { kingside: true, queenside: true }
        };
        this.halfMoveClock = 0;
        this.fullMoveNumber = 1;
        this.gameState = GameResult.ONGOING;
        this.positionHistory = new Map();
        
        // Componenti avanzati
        this.tt = new TranspositionTable(50000);
        this.openingBook = new OpeningBook();
        this.killerMoves = Array(100).fill().map(() => [null, null]); // killer moves per ogni profondità
        this.historyHeuristic = Array(2).fill().map(() => 
            Array(6).fill().map(() => 
                Array(8).fill().map(() => 
                    Array(8).fill(0)
                )
            )
        );
        
        this.initializePieceSquareTables();
        
        if (fen) {
            this.loadFEN(fen);
        } else {
            this.initializeBoard();
        }
    }

    // ============= INIZIALIZZAZIONE =============
    initializePieceSquareTables() {
        this.pieceSquareTables = {
            [PieceType.PAWN]: [
                [0, 0, 0, 0, 0, 0, 0, 0],
                [50, 50, 50, 50, 50, 50, 50, 50],
                [10, 10, 20, 30, 30, 20, 10, 10],
                [5, 5, 10, 25, 25, 10, 5, 5],
                [0, 0, 0, 20, 20, 0, 0, 0],
                [5, -5, -10, 0, 0, -10, -5, 5],
                [5, 10, 10, -20, -20, 10, 10, 5],
                [0, 0, 0, 0, 0, 0, 0, 0]
            ],
            [PieceType.KNIGHT]: [
                [-50, -40, -30, -30, -30, -30, -40, -50],
                [-40, -20, 0, 0, 0, 0, -20, -40],
                [-30, 0, 10, 15, 15, 10, 0, -30],
                [-30, 5, 15, 20, 20, 15, 5, -30],
                [-30, 0, 15, 20, 20, 15, 0, -30],
                [-30, 5, 10, 15, 15, 10, 5, -30],
                [-40, -20, 0, 5, 5, 0, -20, -40],
                [-50, -40, -30, -30, -30, -30, -40, -50]
            ],
            [PieceType.BISHOP]: [
                [-20, -10, -10, -10, -10, -10, -10, -20],
                [-10, 0, 0, 0, 0, 0, 0, -10],
                [-10, 0, 5, 10, 10, 5, 0, -10],
                [-10, 5, 5, 10, 10, 5, 5, -10],
                [-10, 0, 10, 10, 10, 10, 0, -10],
                [-10, 10, 10, 10, 10, 10, 10, -10],
                [-10, 5, 0, 0, 0, 0, 5, -10],
                [-20, -10, -10, -10, -10, -10, -10, -20]
            ],
            [PieceType.ROOK]: [
                [0, 0, 0, 0, 0, 0, 0, 0],
                [5, 10, 10, 10, 10, 10, 10, 5],
                [-5, 0, 0, 0, 0, 0, 0, -5],
                [-5, 0, 0, 0, 0, 0, 0, -5],
                [-5, 0, 0, 0, 0, 0, 0, -5],
                [-5, 0, 0, 0, 0, 0, 0, -5],
                [-5, 0, 0, 0, 0, 0, 0, -5],
                [0, 0, 0, 5, 5, 0, 0, 0]
            ],
            [PieceType.QUEEN]: [
                [-20, -10, -10, -5, -5, -10, -10, -20],
                [-10, 0, 0, 0, 0, 0, 0, -10],
                [-10, 0, 5, 5, 5, 5, 0, -10],
                [-5, 0, 5, 5, 5, 5, 0, -5],
                [0, 0, 5, 5, 5, 5, 0, -5],
                [-10, 5, 5, 5, 5, 5, 0, -10],
                [-10, 0, 5, 0, 0, 0, 0, -10],
                [-20, -10, -10, -5, -5, -10, -10, -20]
            ],
            [PieceType.KING]: [
                [-30, -40, -40, -50, -50, -40, -40, -30],
                [-30, -40, -40, -50, -50, -40, -40, -30],
                [-30, -40, -40, -50, -50, -40, -40, -30],
                [-30, -40, -40, -50, -50, -40, -40, -30],
                [-20, -30, -30, -40, -40, -30, -30, -20],
                [-10, -20, -20, -20, -20, -20, -20, -10],
                [20, 20, 0, 0, 0, 0, 20, 20],
                [20, 30, 10, 0, 0, 10, 30, 20]
            ]
        };
        
        // Tabelle per fasi di gioco (apertura e finale)
        this.kingEndgameTable = [
            [-50, -40, -30, -20, -20, -30, -40, -50],
            [-30, -20, -10, 0, 0, -10, -20, -30],
            [-30, -10, 20, 30, 30, 20, -10, -30],
            [-30, -10, 30, 40, 40, 30, -10, -30],
            [-30, -10, 30, 40, 40, 30, -10, -30],
            [-30, -10, 20, 30, 30, 20, -10, -30],
            [-30, -30, 0, 0, 0, 0, -30, -30],
            [-50, -30, -30, -30, -30, -30, -30, -50]
        ];
    }

    initializeBoard() {
        // Reset board
        for (let row = 0; row < 8; row++) {
            for (let col = 0; col < 8; col++) {
                this.board[row][col] = null;
            }
        }

        // Setup standard
        const pieces = [
            // Bianchi
            [7, 0, PieceType.ROOK, Color.WHITE],
            [7, 1, PieceType.KNIGHT, Color.WHITE],
            [7, 2, PieceType.BISHOP, Color.WHITE],
            [7, 3, PieceType.QUEEN, Color.WHITE],
            [7, 4, PieceType.KING, Color.WHITE],
            [7, 5, PieceType.BISHOP, Color.WHITE],
            [7, 6, PieceType.KNIGHT, Color.WHITE],
            [7, 7, PieceType.ROOK, Color.WHITE],
            [6, 0, PieceType.PAWN, Color.WHITE],
            [6, 1, PieceType.PAWN, Color.WHITE],
            [6, 2, PieceType.PAWN, Color.WHITE],
            [6, 3, PieceType.PAWN, Color.WHITE],
            [6, 4, PieceType.PAWN, Color.WHITE],
            [6, 5, PieceType.PAWN, Color.WHITE],
            [6, 6, PieceType.PAWN, Color.WHITE],
            [6, 7, PieceType.PAWN, Color.WHITE],
            
            // Neri
            [0, 0, PieceType.ROOK, Color.BLACK],
            [0, 1, PieceType.KNIGHT, Color.BLACK],
            [0, 2, PieceType.BISHOP, Color.BLACK],
            [0, 3, PieceType.QUEEN, Color.BLACK],
            [0, 4, PieceType.KING, Color.BLACK],
            [0, 5, PieceType.BISHOP, Color.BLACK],
            [0, 6, PieceType.KNIGHT, Color.BLACK],
            [0, 7, PieceType.ROOK, Color.BLACK],
            [1, 0, PieceType.PAWN, Color.BLACK],
            [1, 1, PieceType.PAWN, Color.BLACK],
            [1, 2, PieceType.PAWN, Color.BLACK],
            [1, 3, PieceType.PAWN, Color.BLACK],
            [1, 4, PieceType.PAWN, Color.BLACK],
            [1, 5, PieceType.PAWN, Color.BLACK],
            [1, 6, PieceType.PAWN, Color.BLACK],
            [1, 7, PieceType.PAWN, Color.BLACK]
        ];

        for (const [row, col, type, color] of pieces) {
            this.board[row][col] = new Piece(type, color);
        }
    }

    // ============= FEN SUPPORT =============
    loadFEN(fen) {
        const parts = fen.split(' ');
        if (parts.length < 4) return false;

        // Reset board
        for (let row = 0; row < 8; row++) {
            for (let col = 0; col < 8; col++) {
                this.board[row][col] = null;
            }
        }

        // Parse board
        const rows = parts[0].split('/');
        for (let row = 0; row < 8; row++) {
            let col = 0;
            for (const ch of rows[row]) {
                if ('12345678'.includes(ch)) {
                    col += parseInt(ch);
                } else {
                    const color = ch === ch.toUpperCase() ? Color.WHITE : Color.BLACK;
                    const pieceType = this.charToPieceType(ch.toUpperCase());
                    if (pieceType !== PieceType.NONE) {
                        this.board[row][col] = new Piece(pieceType, color);
                    }
                    col++;
                }
            }
        }

        // Parse active color
        this.currentPlayer = parts[1] === 'w' ? Color.WHITE : Color.BLACK;

        // Parse castling rights
        this.castlingRights[Color.WHITE] = { kingside: false, queenside: false };
        this.castlingRights[Color.BLACK] = { kingside: false, queenside: false };
        
        if (parts[2] !== '-') {
            for (const ch of parts[2]) {
                switch (ch) {
                    case 'K': this.castlingRights[Color.WHITE].kingside = true; break;
                    case 'Q': this.castlingRights[Color.WHITE].queenside = true; break;
                    case 'k': this.castlingRights[Color.BLACK].kingside = true; break;
                    case 'q': this.castlingRights[Color.BLACK].queenside = true; break;
                }
            }
        }

        // Parse en passant
        this.enPassantTarget = parts[3] === '-' ? null : {
            row: 8 - parseInt(parts[3][1]),
            col: parts[3].charCodeAt(0) - 'a'.charCodeAt(0)
        };

        // Parse half move clock and full move number
        this.halfMoveClock = parts.length > 4 ? parseInt(parts[4]) : 0;
        this.fullMoveNumber = parts.length > 5 ? parseInt(parts[5]) : 1;

        this.gameState = GameResult.ONGOING;
        this.moveHistory = [];
        this.moveSANHistory = [];
        this.positionHistory.clear();
        this.tt.clear();
        
        return true;
    }

    getFEN() {
        let fen = '';

        // Board
        for (let row = 0; row < 8; row++) {
            let empty = 0;
            for (let col = 0; col < 8; col++) {
                const piece = this.board[row][col];
                if (piece) {
                    if (empty > 0) {
                        fen += empty;
                        empty = 0;
                    }
                    fen += piece.getFEN();
                } else {
                    empty++;
                }
            }
            if (empty > 0) fen += empty;
            if (row < 7) fen += '/';
        }

        // Active color
        fen += this.currentPlayer === Color.WHITE ? ' w ' : ' b ';

        // Castling
        let castling = '';
        if (this.castlingRights[Color.WHITE].kingside) castling += 'K';
        if (this.castlingRights[Color.WHITE].queenside) castling += 'Q';
        if (this.castlingRights[Color.BLACK].kingside) castling += 'k';
        if (this.castlingRights[Color.BLACK].queenside) castling += 'q';
        fen += castling || '-';
        fen += ' ';

        // En passant
        if (this.enPassantTarget) {
            const cols = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h'];
            fen += cols[this.enPassantTarget.col] + (8 - this.enPassantTarget.row);
        } else {
            fen += '-';
        }

        // Move clocks
        fen += ' ' + this.halfMoveClock + ' ' + this.fullMoveNumber;

        return fen;
    }

    charToPieceType(ch) {
        switch (ch) {
            case 'P': return PieceType.PAWN;
            case 'N': return PieceType.KNIGHT;
            case 'B': return PieceType.BISHOP;
            case 'R': return PieceType.ROOK;
            case 'Q': return PieceType.QUEEN;
            case 'K': return PieceType.KING;
            default: return PieceType.NONE;
        }
    }

    // ============= GENERAZIONE MOSSE =============
    getAllPossibleMoves(forColor) {
        const moves = [];
        const color = forColor !== undefined ? forColor : this.currentPlayer;

        for (let row = 0; row < 8; row++) {
            for (let col = 0; col < 8; col++) {
                const piece = this.board[row][col];
                if (piece && piece.color === color) {
                    this.getMovesForPiece(row, col, moves);
                }
            }
        }

        return moves;
    }

    getMovesForPiece(row, col, moves) {
        const piece = this.board[row][col];
        if (!piece) return;

        switch (piece.type) {
            case PieceType.PAWN:
                this.getPawnMoves(row, col, moves);
                break;
            case PieceType.KNIGHT:
                this.getKnightMoves(row, col, moves);
                break;
            case PieceType.BISHOP:
                this.getBishopMoves(row, col, moves);
                break;
            case PieceType.ROOK:
                this.getRookMoves(row, col, moves);
                break;
            case PieceType.QUEEN:
                this.getQueenMoves(row, col, moves);
                break;
            case PieceType.KING:
                this.getKingMoves(row, col, moves);
                break;
        }
    }

    getPawnMoves(row, col, moves) {
        const piece = this.board[row][col];
        const direction = piece.color === Color.WHITE ? -1 : 1;
        const startRow = piece.color === Color.WHITE ? 6 : 1;
        const promotionRow = piece.color === Color.WHITE ? 0 : 7;

        // Avanti 1
        let newRow = row + direction;
        if (newRow >= 0 && newRow < 8) {
            if (!this.board[newRow][col]) {
                if (newRow === promotionRow) {
                    // Promozione
                    [PieceType.QUEEN, PieceType.ROOK, PieceType.BISHOP, PieceType.KNIGHT].forEach(promType => {
                        moves.push(new Move(row, col, newRow, col, promType));
                    });
                } else {
                    moves.push(new Move(row, col, newRow, col));
                }
                
                // Avanti 2 dalla posizione iniziale
                if (row === startRow) {
                    const newRow2 = row + 2 * direction;
                    if (newRow2 >= 0 && newRow2 < 8 && !this.board[newRow2][col]) {
                        moves.push(new Move(row, col, newRow2, col));
                    }
                }
            }
        }

        // Catture
        const captureCols = [col - 1, col + 1];
        for (const newCol of captureCols) {
            if (newCol >= 0 && newCol < 8) {
                newRow = row + direction;
                if (newRow >= 0 && newRow < 8) {
                    const target = this.board[newRow][newCol];
                    if (target && target.color !== piece.color) {
                        if (newRow === promotionRow) {
                            // Promozione con cattura
                            [PieceType.QUEEN, PieceType.ROOK, PieceType.BISHOP, PieceType.KNIGHT].forEach(promType => {
                                moves.push(new Move(row, col, newRow, newCol, promType));
                            });
                        } else {
                            moves.push(new Move(row, col, newRow, newCol));
                        }
                    }
                    
                    // En passant
                    if (this.enPassantTarget && 
                        newRow === this.enPassantTarget.row && 
                        newCol === this.enPassantTarget.col) {
                        moves.push(new Move(row, col, newRow, newCol, PieceType.NONE, false, true));
                    }
                }
            }
        }
    }

    getKnightMoves(row, col, moves) {
        const piece = this.board[row][col];
        const knightMoves = [
            [-2, -1], [-2, 1],
            [-1, -2], [-1, 2],
            [1, -2], [1, 2],
            [2, -1], [2, 1]
        ];

        for (const [dr, dc] of knightMoves) {
            const newRow = row + dr;
            const newCol = col + dc;
            if (newRow >= 0 && newRow < 8 && newCol >= 0 && newCol < 8) {
                const target = this.board[newRow][newCol];
                if (!target || target.color !== piece.color) {
                    moves.push(new Move(row, col, newRow, newCol));
                }
            }
        }
    }

    getBishopMoves(row, col, moves) {
        this.getSlidingMoves(row, col, moves, [
            [-1, -1], [-1, 1],
            [1, -1], [1, 1]
        ]);
    }

    getRookMoves(row, col, moves) {
        this.getSlidingMoves(row, col, moves, [
            [-1, 0], [1, 0],
            [0, -1], [0, 1]
        ]);
    }

    getQueenMoves(row, col, moves) {
        this.getSlidingMoves(row, col, moves, [
            [-1, -1], [-1, 1],
            [1, -1], [1, 1],
            [-1, 0], [1, 0],
            [0, -1], [0, 1]
        ]);
    }

    getSlidingMoves(row, col, moves, directions) {
        const piece = this.board[row][col];
        
        for (const [dr, dc] of directions) {
            let newRow = row + dr;
            let newCol = col + dc;
            
            while (newRow >= 0 && newRow < 8 && newCol >= 0 && newCol < 8) {
                const target = this.board[newRow][newCol];
                
                if (!target) {
                    moves.push(new Move(row, col, newRow, newCol));
                } else {
                    if (target.color !== piece.color) {
                        moves.push(new Move(row, col, newRow, newCol));
                    }
                    break;
                }
                
                newRow += dr;
                newCol += dc;
            }
        }
    }

    getKingMoves(row, col, moves) {
        const piece = this.board[row][col];
        const kingMoves = [
            [-1, -1], [-1, 0], [-1, 1],
            [0, -1],           [0, 1],
            [1, -1],  [1, 0],  [1, 1]
        ];

        for (const [dr, dc] of kingMoves) {
            const newRow = row + dr;
            const newCol = col + dc;
            if (newRow >= 0 && newRow < 8 && newCol >= 0 && newCol < 8) {
                const target = this.board[newRow][newCol];
                if (!target || target.color !== piece.color) {
                    moves.push(new Move(row, col, newRow, newCol));
                }
            }
        }

        // Arrocco - solo se il re non è sotto scacco
        if (!piece.hasMoved && !this.isInCheck(piece.color)) {
            const opponentColor = piece.color === Color.WHITE ? Color.BLACK : Color.WHITE;
            
            // Arrocco corto
            if (this.castlingRights[piece.color].kingside) {
                const rook = this.board[row][7];
                if (rook && rook.type === PieceType.ROOK && !rook.hasMoved) {
                    if (!this.board[row][5] && !this.board[row][6]) {
                        if (!this.isSquareAttacked(row, 4, opponentColor) &&
                            !this.isSquareAttacked(row, 5, opponentColor) &&
                            !this.isSquareAttacked(row, 6, opponentColor)) {
                            const move = new Move(row, col, row, 6);
                            move.isCastling = true;
                            moves.push(move);
                        }
                    }
                }
            }
            
            // Arrocco lungo
            if (this.castlingRights[piece.color].queenside) {
                const rook = this.board[row][0];
                if (rook && rook.type === PieceType.ROOK && !rook.hasMoved) {
                    if (!this.board[row][1] && !this.board[row][2] && !this.board[row][3]) {
                        if (!this.isSquareAttacked(row, 4, opponentColor) &&
                            !this.isSquareAttacked(row, 3, opponentColor) &&
                            !this.isSquareAttacked(row, 2, opponentColor)) {
                            const move = new Move(row, col, row, 2);
                            move.isCastling = true;
                            moves.push(move);
                        }
                    }
                }
            }
        }
    }

    isSquareAttacked(row, col, byColor) {
        // Controlla pedoni
        const pawnDirection = byColor === Color.WHITE ? -1 : 1;
        for (const dc of [-1, 1]) {
            const newRow = row + pawnDirection;
            const newCol = col + dc;
            if (newRow >= 0 && newRow < 8 && newCol >= 0 && newCol < 8) {
                const piece = this.board[newRow][newCol];
                if (piece && piece.type === PieceType.PAWN && piece.color === byColor) {
                    return true;
                }
            }
        }

        // Controlla cavalli
        const knightMoves = [
            [-2, -1], [-2, 1],
            [-1, -2], [-1, 2],
            [1, -2], [1, 2],
            [2, -1], [2, 1]
        ];
        for (const [dr, dc] of knightMoves) {
            const newRow = row + dr;
            const newCol = col + dc;
            if (newRow >= 0 && newRow < 8 && newCol >= 0 && newCol < 8) {
                const piece = this.board[newRow][newCol];
                if (piece && piece.type === PieceType.KNIGHT && piece.color === byColor) {
                    return true;
                }
            }
        }

        // Controlla re
        const kingMoves = [
            [-1, -1], [-1, 0], [-1, 1],
            [0, -1],           [0, 1],
            [1, -1],  [1, 0],  [1, 1]
        ];
        for (const [dr, dc] of kingMoves) {
            const newRow = row + dr;
            const newCol = col + dc;
            if (newRow >= 0 && newRow < 8 && newCol >= 0 && newCol < 8) {
                const piece = this.board[newRow][newCol];
                if (piece && piece.type === PieceType.KING && piece.color === byColor) {
                    return true;
                }
            }
        }

        // Controlla pezzi che slittano
        const slidingDirections = [
            [-1, -1], [-1, 1],
            [1, -1], [1, 1],
            [-1, 0], [1, 0],
            [0, -1], [0, 1]
        ];

        for (const [dr, dc] of slidingDirections) {
            let newRow = row + dr;
            let newCol = col + dc;
            
            while (newRow >= 0 && newRow < 8 && newCol >= 0 && newCol < 8) {
                const piece = this.board[newRow][newCol];
                if (piece) {
                    if (piece.color === byColor) {
                        // Diagonali (alfiere/regina)
                        if ((dr === -1 || dr === 1) && (dc === -1 || dc === 1)) {
                            if (piece.type === PieceType.BISHOP || piece.type === PieceType.QUEEN) {
                                return true;
                            }
                        }
                        // Orizzontali/verticali (torre/regina)
                        if ((dr === 0 || dc === 0)) {
                            if (piece.type === PieceType.ROOK || piece.type === PieceType.QUEEN) {
                                return true;
                            }
                        }
                    }
                    break;
                }
                
                newRow += dr;
                newCol += dc;
            }
        }

        return false;
    }

    findKing(color) {
        for (let row = 0; row < 8; row++) {
            for (let col = 0; col < 8; col++) {
                const piece = this.board[row][col];
                if (piece && piece.type === PieceType.KING && piece.color === color) {
                    return { row, col };
                }
            }
        }
        return null;
    }

    isInCheck(color) {
        const kingPos = this.findKing(color);
        if (!kingPos) return false;
        
        const opponentColor = color === Color.WHITE ? Color.BLACK : Color.WHITE;
        return this.isSquareAttacked(kingPos.row, kingPos.col, opponentColor);
    }

    getLegalMoves(forColor) {
        const color = forColor !== undefined ? forColor : this.currentPlayer;
        const allMoves = this.getAllPossibleMoves(color);
        const legalMoves = [];

        for (const move of allMoves) {
            if (this.isMoveLegal(move)) {
                legalMoves.push(move);
            }
        }

        return legalMoves;
    }

    isMoveLegal(move) {
        // Salva lo stato corrente
        const originalBoard = this.copyBoard();
        const originalPlayer = this.currentPlayer;
        const originalCastling = JSON.parse(JSON.stringify(this.castlingRights));
        const originalEnPassant = this.enPassantTarget;
        const originalHalfMove = this.halfMoveClock;
        
        // Esegui la mossa
        if (!this.makeMoveInternal(move, false)) {
            return false;
        }
        
        // Controlla se il re è sotto scacco
        const inCheck = this.isInCheck(originalPlayer);
        
        // Ripristina lo stato
        this.board = originalBoard;
        this.currentPlayer = originalPlayer;
        this.castlingRights = originalCastling;
        this.enPassantTarget = originalEnPassant;
        this.halfMoveClock = originalHalfMove;
        
        return !inCheck;
    }

    // ============= ESECUZIONE MOSSE =============
    makeMove(moveStr) {
        if (!moveStr || moveStr.length < 2) return false;
        
        let move;
        
        // Controlla se è arrocco
        if (moveStr === 'O-O' || moveStr === 'O-O-O') {
            const color = this.currentPlayer;
            const row = color === Color.WHITE ? 7 : 0;
            const kingCol = 4;
            const newKingCol = moveStr === 'O-O' ? 6 : 2;
            
            // Verifica che ci sia il re
            const king = this.board[row][kingCol];
            if (!king || king.type !== PieceType.KING || king.color !== color) {
                return false;
            }
            
            move = new Move(row, kingCol, row, newKingCol);
            move.isCastling = true;
        } else {
            // Mossa normale
            const cols = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h'];
            
            // Supporta varie forme di input
            let fromCol, fromRow, toCol, toRow, promotion = PieceType.NONE;
            
            if (moveStr.length >= 4) {
                fromCol = cols.indexOf(moveStr[0].toLowerCase());
                fromRow = 8 - parseInt(moveStr[1]);
                toCol = cols.indexOf(moveStr[2].toLowerCase());
                toRow = 8 - parseInt(moveStr[3]);
                
                // Promozione
                if (moveStr.length > 4) {
                    const promChar = moveStr[4].toLowerCase();
                    switch (promChar) {
                        case 'q': promotion = PieceType.QUEEN; break;
                        case 'r': promotion = PieceType.ROOK; break;
                        case 'b': promotion = PieceType.BISHOP; break;
                        case 'n': promotion = PieceType.KNIGHT; break;
                    }
                }
            } else {
                return false;
            }
            
            if (fromCol < 0 || fromRow < 0 || toCol < 0 || toRow < 0) {
                return false;
            }
            
            move = new Move(fromRow, fromCol, toRow, toCol, promotion);
        }
        
        // Controlla se la mossa è legale
        const legalMoves = this.getLegalMoves();
        const isValid = legalMoves.some(m => 
            m.fromRow === move.fromRow && 
            m.fromCol === move.fromCol && 
            m.toRow === move.toRow && 
            m.toCol === move.toCol
        );
        
        if (!isValid) return false;
        
        // Esegui la mossa
        return this.makeMoveInternal(move, true);
    }

    makeMoveInternal(move, updateHistory = true) {
        const piece = this.board[move.fromRow][move.fromCol];
        if (!piece) return false;
        
        // Salva lo stato per eventuale undo
        const captured = this.board[move.toRow][move.toCol];
        const oldCastlingRights = JSON.parse(JSON.stringify(this.castlingRights));
        const oldEnPassant = this.enPassantTarget;
        const oldHalfMoveClock = this.halfMoveClock;
        
        // Gestione cattura
        if (captured) {
            move.isCapture = true;
            move.capturedPiece = captured.copy();
        }
        
        // Gestione en passant
        if (move.isEnPassant) {
            const capturedPawnRow = move.fromRow;
            const capturedPawnCol = move.toCol;
            const capturedPawn = this.board[capturedPawnRow][capturedPawnCol];
            if (capturedPawn && capturedPawn.type === PieceType.PAWN) {
                move.isCapture = true;
                move.capturedPiece = capturedPawn.copy();
                this.board[capturedPawnRow][capturedPawnCol] = null;
            }
        }
        
        // Esegui la mossa
        this.board[move.toRow][move.toCol] = piece;
        this.board[move.fromRow][move.fromCol] = null;
        
        // Promozione
        if (move.promotion !== PieceType.NONE) {
            this.board[move.toRow][move.toCol] = new Piece(move.promotion, piece.color);
        }
        
        // Arrocco
        if (move.isCastling) {
            const row = move.fromRow;
            const isKingside = move.toCol === 6;
            const rookFromCol = isKingside ? 7 : 0;
            const rookToCol = isKingside ? 5 : 3;
            
            const rook = this.board[row][rookFromCol];
            if (rook && rook.type === PieceType.ROOK) {
                this.board[row][rookToCol] = rook;
                this.board[row][rookFromCol] = null;
                rook.hasMoved = true;
            }
        }
        
        // Aggiorna stato del pezzo
        piece.hasMoved = true;
        
        // Aggiorna diritti di arrocco
        if (piece.type === PieceType.KING) {
            this.castlingRights[piece.color] = { kingside: false, queenside: false };
        } else if (piece.type === PieceType.ROOK) {
            if (move.fromRow === 7) { // Torre bianca
                if (move.fromCol === 0) this.castlingRights[Color.WHITE].queenside = false;
                if (move.fromCol === 7) this.castlingRights[Color.WHITE].kingside = false;
            } else if (move.fromRow === 0) { // Torre nera
                if (move.fromCol === 0) this.castlingRights[Color.BLACK].queenside = false;
                if (move.fromCol === 7) this.castlingRights[Color.BLACK].kingside = false;
            }
        }
        
        // Aggiorna en passant
        this.enPassantTarget = null;
        if (piece.type === PieceType.PAWN && Math.abs(move.fromRow - move.toRow) === 2) {
            const enPassantRow = (move.fromRow + move.toRow) / 2;
            this.enPassantTarget = { row: enPassantRow, col: move.toCol };
        }
        
        // Aggiorna orologio mosse
        if (piece.type === PieceType.PAWN || move.isCapture) {
            this.halfMoveClock = 0;
        } else {
            this.halfMoveClock++;
        }
        
        if (this.currentPlayer === Color.BLACK) {
            this.fullMoveNumber++;
        }
        
        // Aggiorna giocatore corrente
        this.currentPlayer = this.currentPlayer === Color.WHITE ? Color.BLACK : Color.WHITE;
        
        // Salva nella history
        if (updateHistory) {
            // Calcola SAN per la mossa
            const isCheck = this.isInCheck(this.currentPlayer);
            const isMate = isCheck && this.getLegalMoves().length === 0;
            
            // Controllo di sicurezza per il pezzo
            if (piece) {
                const san = move.getSAN(piece, this.board, isCheck, isMate);
                this.moveHistory.push({
                    move: move.toString(),
                    san: san,
                    piece: piece.copy(),
                    captured: move.capturedPiece,
                    castlingRights: oldCastlingRights,
                    enPassantTarget: oldEnPassant,
                    halfMoveClock: oldHalfMoveClock
                });
                
                this.moveSANHistory.push(san);
            } else {
                // Fallback
                this.moveHistory.push({
                    move: move.toString(),
                    san: move.toString(),
                    piece: null,
                    captured: move.capturedPiece,
                    castlingRights: oldCastlingRights,
                    enPassantTarget: oldEnPassant,
                    halfMoveClock: oldHalfMoveClock
                });
                
                this.moveSANHistory.push(move.toString());
            }
            
            // Aggiungi posizione alla history
            const fen = this.getFEN().split(' ').slice(0, 4).join(' ');
            this.positionHistory.set(fen, (this.positionHistory.get(fen) || 0) + 1);
            
            // Controlla condizioni di fine partita
            this.updateGameState();
        }
        
        return true;
    }

    undoMove() {
        if (this.moveHistory.length === 0) return false;
        
        const lastMove = this.moveHistory.pop();
        this.moveSANHistory.pop();
        const move = this.parseMove(lastMove.move);
        
        // Ripristina giocatore corrente
        this.currentPlayer = this.currentPlayer === Color.WHITE ? Color.BLACK : Color.WHITE;
        
        // Ripristina pezzo
        const piece = lastMove.piece;
        this.board[move.fromRow][move.fromCol] = piece;
        
        // Ripristina pezzo nella casella di arrivo (o null)
        if (move.isEnPassant) {
            // Per en passant, la casella di arrivo è vuota
            this.board[move.toRow][move.toCol] = null;
            // Rimetti il pedone catturato nella sua posizione originale
            const capturedRow = move.fromRow;
            const capturedCol = move.toCol;
            if (lastMove.captured) {
                this.board[capturedRow][capturedCol] = lastMove.captured;
            }
        } else {
            this.board[move.toRow][move.toCol] = lastMove.captured;
        }
        
        // Ripristina promozione
        if (move.promotion !== PieceType.NONE) {
            this.board[move.fromRow][move.fromCol] = new Piece(PieceType.PAWN, piece.color);
            this.board[move.fromRow][move.fromCol].hasMoved = piece.hasMoved;
        }
        
        // Ripristina arrocco
        if (move.isCastling) {
            const row = move.fromRow;
            const isKingside = move.toCol === 6;
            const rookFromCol = isKingside ? 5 : 3;
            const rookToCol = isKingside ? 7 : 0;
            
            const rook = this.board[row][rookFromCol];
            if (rook && rook.type === PieceType.ROOK) {
                this.board[row][rookToCol] = rook;
                this.board[row][rookFromCol] = null;
                rook.hasMoved = false;
            }
        }
        
        // Ripristina diritti di arrocco
        this.castlingRights = lastMove.castlingRights;
        this.enPassantTarget = lastMove.enPassantTarget;
        this.halfMoveClock = lastMove.halfMoveClock;
        
        if (this.currentPlayer === Color.BLACK) {
            this.fullMoveNumber--;
        }
        
        // Rimuovi dalla history delle posizioni
        const fen = this.getFEN().split(' ').slice(0, 4).join(' ');
        const count = (this.positionHistory.get(fen) || 1) - 1;
        if (count <= 0) {
            this.positionHistory.delete(fen);
        } else {
            this.positionHistory.set(fen, count);
        }
        
        this.gameState = GameResult.ONGOING;
        return true;
    }

    parseMove(moveStr) {
        const cols = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h'];
        
        if (moveStr === 'O-O') {
            const row = this.currentPlayer === Color.WHITE ? 7 : 0;
            return new Move(row, 4, row, 6, PieceType.NONE, true);
        } else if (moveStr === 'O-O-O') {
            const row = this.currentPlayer === Color.WHITE ? 7 : 0;
            return new Move(row, 4, row, 2, PieceType.NONE, true);
        }
        
        const fromCol = cols.indexOf(moveStr[0]);
        const fromRow = 8 - parseInt(moveStr[1]);
        const toCol = cols.indexOf(moveStr[2]);
        const toRow = 8 - parseInt(moveStr[3]);
        let promotion = PieceType.NONE;
        
        if (moveStr.length > 4) {
            const promChar = moveStr[4].toLowerCase();
            switch (promChar) {
                case 'q': promotion = PieceType.QUEEN; break;
                case 'r': promotion = PieceType.ROOK; break;
                case 'b': promotion = PieceType.BISHOP; break;
                case 'n': promotion = PieceType.KNIGHT; break;
            }
        }
        
        return new Move(fromRow, fromCol, toRow, toCol, promotion);
    }

    copyBoard() {
        const newBoard = Array(8).fill().map(() => Array(8).fill(null));
        for (let row = 0; row < 8; row++) {
            for (let col = 0; col < 8; col++) {
                if (this.board[row][col]) {
                    newBoard[row][col] = this.board[row][col].copy();
                }
            }
        }
        return newBoard;
    }

    // ============= VALUTAZIONE AVANZATA =============
    evaluate() {
        if (this.gameState !== GameResult.ONGOING) {
            switch (this.gameState) {
                case GameResult.WHITE_WINS: return 100000;
                case GameResult.BLACK_WINS: return -100000;
                case GameResult.DRAW:
                case GameResult.STALEMATE: return 0;
            }
        }
        
        let score = 0;
        
        // Valore materiale e posizionale
        for (let row = 0; row < 8; row++) {
            for (let col = 0; col < 8; col++) {
                const piece = this.board[row][col];
                if (piece) {
                    // Valore materiale
                    score += piece.color === Color.WHITE ? piece.getValue() : -piece.getValue();
                    
                    // Valore posizionale
                    score += this.getPositionalBonus(piece, row, col);
                }
            }
        }
        
        // Bonus per coppia degli alfieri
        score += this.getBishopPairBonus();
        
        // Mobilità
        const whiteMobility = this.getLegalMoves(Color.WHITE).length;
        const blackMobility = this.getLegalMoves(Color.BLACK).length;
        score += (whiteMobility - blackMobility) * 2;
        
        // Struttura pedonale
        score += this.evaluatePawnStructure(Color.WHITE) - this.evaluatePawnStructure(Color.BLACK);
        
        // Controllo del centro
        score += this.evaluateCenterControl(Color.WHITE) - this.evaluateCenterControl(Color.BLACK);
        
        // Penalità per re sotto scacco
        if (this.isInCheck(Color.WHITE)) score -= 25;
        if (this.isInCheck(Color.BLACK)) score += 25;
        
        // Bonus per sviluppo in apertura
        if (this.fullMoveNumber < 20) {
            score += this.getDevelopmentScore(Color.WHITE) - this.getDevelopmentScore(Color.BLACK);
        }
        
        // Fase di gioco (0 = apertura, 1 = finale)
        const gamePhase = this.getGamePhase();
        if (gamePhase > 0.7) { // Finale
            // Bonus per re attivo in finale
            score += this.getKingActivityBonus(Color.WHITE) - this.getKingActivityBonus(Color.BLACK);
        }
        
        return Math.round(score);
    }

    getPositionalBonus(piece, row, col) {
        const table = this.pieceSquareTables[piece.type];
        if (!table) return 0;
        
        const actualRow = piece.color === Color.WHITE ? 7 - row : row;
        const value = table[actualRow][col] || 0;
        
        // Per il re, usa tabella diversa in finale
        if (piece.type === PieceType.KING && this.getGamePhase() > 0.7) {
            const endgameValue = this.kingEndgameTable[actualRow][col] || 0;
            return piece.color === Color.WHITE ? endgameValue : -endgameValue;
        }
        
        return piece.color === Color.WHITE ? value : -value;
    }

    getBishopPairBonus() {
        let whiteBishops = 0;
        let blackBishops = 0;
        
        for (let row = 0; row < 8; row++) {
            for (let col = 0; col < 8; col++) {
                const piece = this.board[row][col];
                if (piece && piece.type === PieceType.BISHOP) {
                    if (piece.color === Color.WHITE) whiteBishops++;
                    else blackBishops++;
                }
            }
        }
        
        return (whiteBishops >= 2 ? 50 : 0) - (blackBishops >= 2 ? 50 : 0);
    }

    evaluatePawnStructure(color) {
        let score = 0;
        const pawns = [];
        
        // Raccogli tutti i pedoni
        for (let row = 0; row < 8; row++) {
            for (let col = 0; col < 8; col++) {
                const piece = this.board[row][col];
                if (piece && piece.type === PieceType.PAWN && piece.color === color) {
                    pawns.push({row, col});
                }
            }
        }
        
        // Valuta ogni pedone
        for (const pawn of pawns) {
            // Bonus per pedoni passati
            if (this.isPassedPawn(pawn.row, pawn.col, color)) {
                const advancement = color === Color.WHITE ? 7 - pawn.row : pawn.row;
                score += 20 + advancement * 10;
            }
            
            // Penalità per pedoni doppiati
            for (const other of pawns) {
                if (other.col === pawn.col && other.row !== pawn.row) {
                    score -= 15;
                }
            }
            
            // Penalità per pedoni isolati
            if (!this.hasPawnSupport(pawn.row, pawn.col, color)) {
                score -= 10;
            }
        }
        
        return score;
    }

    isPassedPawn(row, col, color) {
        const direction = color === Color.WHITE ? -1 : 1;
        const opponentColor = color === Color.WHITE ? Color.BLACK : Color.WHITE;
        
        // Controlla le colonne adiacenti davanti al pedone
        for (let c = Math.max(0, col - 1); c <= Math.min(7, col + 1); c++) {
            for (let r = row + direction; r >= 0 && r < 8; r += direction) {
                const piece = this.board[r][c];
                if (piece && piece.type === PieceType.PAWN && piece.color === opponentColor) {
                    return false;
                }
            }
        }
        
        return true;
    }

    hasPawnSupport(row, col, color) {
        const leftCol = col - 1;
        const rightCol = col + 1;
        
        if (leftCol >= 0) {
            for (let r = 0; r < 8; r++) {
                const piece = this.board[r][leftCol];
                if (piece && piece.type === PieceType.PAWN && piece.color === color) {
                    return true;
                }
            }
        }
        
        if (rightCol < 8) {
            for (let r = 0; r < 8; r++) {
                const piece = this.board[r][rightCol];
                if (piece && piece.type === PieceType.PAWN && piece.color === color) {
                    return true;
                }
            }
        }
        
        return false;
    }

    evaluateCenterControl(color) {
        let score = 0;
        const centerSquares = [[3, 3], [3, 4], [4, 3], [4, 4]];
        
        for (const [row, col] of centerSquares) {
            const piece = this.board[row][col];
            if (piece && piece.color === color) {
                score += 5;
            }
            
            // Controllo da lontano
            const legalMoves = this.getLegalMoves(color);
            for (const move of legalMoves) {
                if (move.toRow === row && move.toCol === col) {
                    score += 2;
                }
            }
        }
        
        return score;
    }

    getDevelopmentScore(color) {
        let score = 0;
        const homeRow = color === Color.WHITE ? 7 : 0;
        
        // Bonus per pezzi sviluppati
        for (let col = 0; col < 8; col++) {
            const piece = this.board[homeRow][col];
            if (piece && piece.color === color) {
                if (piece.hasMoved) {
                    score += 10; // Pezzo sviluppato
                } else if (piece.type === PieceType.KNIGHT || piece.type === PieceType.BISHOP) {
                    score -= 5; // Pezzo non sviluppato
                }
            }
        }
        
        // Bonus per controllo del centro con cavalli/alfieri
        const minorPieces = [PieceType.KNIGHT, PieceType.BISHOP];
        for (let row = 0; row < 8; row++) {
            for (let col = 0; col < 8; col++) {
                const piece = this.board[row][col];
                if (piece && piece.color === color && minorPieces.includes(piece.type)) {
                    if (row >= 2 && row <= 5 && col >= 2 && col <= 5) {
                        score += 5; // Pezzo minore nel centro
                    }
                }
            }
        }
        
        return score;
    }

    getGamePhase() {
        let material = 0;
        let maxMaterial = 0;
        
        // Calcola materiale totale
        for (let row = 0; row < 8; row++) {
            for (let col = 0; col < 8; col++) {
                const piece = this.board[row][col];
                if (piece && piece.type !== PieceType.KING) {
                    material += piece.getValue();
                    maxMaterial += piece.getValue();
                }
            }
        }
        
        // Fase di gioco: 0 = apertura, 1 = finale
        const maxNonPawnMaterial = 4*320 + 4*330 + 4*500 + 2*900; // Max senza pedoni
        const phase = Math.min(1, Math.max(0, (maxMaterial - material) / maxNonPawnMaterial));
        
        return phase;
    }

    getKingActivityBonus(color) {
        const kingPos = this.findKing(color);
        if (!kingPos) return 0;
        
        // In finale, re vicino al centro è buono
        const centerDistance = Math.abs(3.5 - kingPos.row) + Math.abs(3.5 - kingPos.col);
        return 20 - centerDistance * 5;
    }

    // ============= CONTROLLO FINE PARTITA =============
    updateGameState() {
        const legalMoves = this.getLegalMoves();
        
        // 1. Scaccomatto
        if (this.isInCheck(this.currentPlayer) && legalMoves.length === 0) {
            this.gameState = this.currentPlayer === Color.WHITE ? 
                GameResult.BLACK_WINS : GameResult.WHITE_WINS;
            return;
        }
        
        // 2. Stallo
        if (!this.isInCheck(this.currentPlayer) && legalMoves.length === 0) {
            this.gameState = GameResult.STALEMATE;
            return;
        }
        
        // 3. Regola 50 mosse
        if (this.halfMoveClock >= 100) {
            this.gameState = GameResult.DRAW;
            return;
        }
        
        // 4. Materiale insufficiente
        if (this.isInsufficientMaterial()) {
            this.gameState = GameResult.DRAW;
            return;
        }
        
        // 5. Ripetizione tripla
        for (const count of this.positionHistory.values()) {
            if (count >= 3) {
                this.gameState = GameResult.DRAW;
                return;
            }
        }
    }

    isInsufficientMaterial() {
        let pieces = [];
        
        for (let row = 0; row < 8; row++) {
            for (let col = 0; col < 8; col++) {
                const piece = this.board[row][col];
                if (piece) {
                    pieces.push(piece);
                }
            }
        }
        
        // Solo re vs re
        if (pieces.length === 2) return true;
        
        // Re e alfiere vs re
        if (pieces.length === 3) {
            const bishops = pieces.filter(p => p.type === PieceType.BISHOP);
            const knights = pieces.filter(p => p.type === PieceType.KNIGHT);
            return bishops.length === 1 || knights.length === 1;
        }
        
        // Re e alfiere vs re e alfiere (stesso colore)
        if (pieces.length === 4) {
            const bishops = pieces.filter(p => p.type === PieceType.BISHOP);
            if (bishops.length === 2) {
                return true;
            }
        }
        
        return false;
    }

    isGameOver() {
        return this.gameState !== GameResult.ONGOING;
    }

    getResult() {
        switch (this.gameState) {
            case GameResult.WHITE_WINS: return "1-0";
            case GameResult.BLACK_WINS: return "0-1";
            case GameResult.DRAW:
            case GameResult.STALEMATE: return "1/2-1/2";
            default: return "*";
        }
    }

    getGameOverReason() {
        switch (this.gameState) {
            case GameResult.WHITE_WINS: return "SCACCOMATTO - Bianco vince";
            case GameResult.BLACK_WINS: return "SCACCOMATTO - Nero vince";
            case GameResult.STALEMATE: return "STALLO";
            case GameResult.DRAW: 
                if (this.halfMoveClock >= 100) return "REGOLE 50 MOSSE";
                if (this.isInsufficientMaterial()) return "MATERIALE INSUFFICIENTE";
                return "PAREGGIO PER RIPETIZIONE";
            default: return "Partita in corso";
        }
    }

    // ============= ALGORITMO DI RICERCA AVANZATO =============
    orderMoves(moves, depth, ttMove = null) {
        const scoredMoves = [];
        
        for (const move of moves) {
            let score = 0;
            
            // 1. Mossa dalla transposition table
            if (ttMove && move.equals(ttMove)) {
                score += 1000000;
            }
            
            // 2. Catture - MVV/LVA (Most Valuable Victim / Least Valuable Aggressor)
            if (move.isCapture) {
                const victim = this.board[move.toRow]?.[move.toCol];
                const aggressor = this.board[move.fromRow][move.fromCol];
                if (victim && aggressor) {
                    score += 10000 + victim.getValue() - aggressor.getValue() / 100;
                }
            }
            
            // 3. Killer moves
            if (depth < this.killerMoves.length) {
                if (this.killerMoves[depth][0] && move.equals(this.killerMoves[depth][0])) {
                    score += 9000;
                } else if (this.killerMoves[depth][1] && move.equals(this.killerMoves[depth][1])) {
                    score += 8000;
                }
            }
            
            // 4. History heuristic
            const piece = this.board[move.fromRow][move.fromCol];
            if (piece) {
                score += this.historyHeuristic[piece.color][piece.type][move.toRow][move.toCol];
            }
            
            // 5. Promozioni
            if (move.promotion !== PieceType.NONE) {
                score += 5000;
            }
            
            scoredMoves.push({move, score});
        }
        
        // Ordina in ordine decrescente di score
        scoredMoves.sort((a, b) => b.score - a.score);
        return scoredMoves.map(sm => sm.move);
    }

    updateHistoryHeuristic(piece, move, depth) {
        if (piece) {
            this.historyHeuristic[piece.color][piece.type][move.toRow][move.toCol] += depth * depth;
        }
    }

    updateKillerMove(move, depth) {
        if (depth < this.killerMoves.length) {
            if (!this.killerMoves[depth][0] || !move.equals(this.killerMoves[depth][0])) {
                this.killerMoves[depth][1] = this.killerMoves[depth][0];
                this.killerMoves[depth][0] = move;
            }
        }
    }

    quiescenceSearch(alpha, beta, depth = 3) {
        const standPat = this.evaluate();
        
        if (depth <= 0) return standPat;
        
        if (standPat >= beta) return beta;
        if (standPat > alpha) alpha = standPat;
        
        // Genera solo mosse di cattura
        const captureMoves = this.getLegalMoves().filter(move => 
            move.isCapture || move.promotion !== PieceType.NONE
        );
        
        const orderedMoves = this.orderMoves(captureMoves, 0);
        
        for (const move of orderedMoves) {
            // Filtra catture svantaggiose (SEE approssimativo)
            const movingPiece = this.board[move.fromRow][move.fromCol];
            const capturedPiece = this.board[move.toRow]?.[move.toCol];
            
            if (capturedPiece && capturedPiece.getValue() + 200 < movingPiece.getValue()) {
                continue; // Salta catture svantaggiose
            }
            
            // Esegui mossa
            const piece = this.board[move.fromRow][move.fromCol];
            const captured = this.board[move.toRow]?.[move.toCol];
            
            this.board[move.toRow][move.toCol] = piece;
            this.board[move.fromRow][move.fromCol] = null;
            
            if (move.promotion !== PieceType.NONE) {
                this.board[move.toRow][move.toCol] = new Piece(move.promotion, piece.color);
            }
            
            const oldPlayer = this.currentPlayer;
            this.currentPlayer = this.currentPlayer === Color.WHITE ? Color.BLACK : Color.WHITE;
            
            const score = -this.quiescenceSearch(-beta, -alpha, depth - 1);
            
            // Annulla mossa
            this.currentPlayer = oldPlayer;
            this.board[move.fromRow][move.fromCol] = piece;
            this.board[move.toRow][move.toCol] = captured;
            
            if (score >= beta) return beta;
            if (score > alpha) alpha = score;
        }
        
        return alpha;
    }

    negamax(depth, alpha, beta, color) {
        const originalAlpha = alpha;
        
        // Controllo transposition table
        const ttKey = this.getZobristKey();
        const ttEntry = this.tt.retrieve(ttKey, depth, alpha, beta);
        if (ttEntry) {
            return ttEntry.score;
        }
        
        // Controllo fine partita o profondità 0
        if (depth <= 0 || this.isGameOver()) {
            const score = this.quiescenceSearch(alpha, beta, 3);
            return color * score;
        }
        
        // Null move pruning
        if (depth >= 3 && !this.isInCheck(this.currentPlayer)) {
            const R = 2; // Riduzione
            this.currentPlayer = this.currentPlayer === Color.WHITE ? Color.BLACK : Color.WHITE;
            const score = -this.negamax(depth - 1 - R, -beta, -beta + 1, -color);
            this.currentPlayer = this.currentPlayer === Color.WHITE ? Color.BLACK : Color.WHITE;
            
            if (score >= beta) {
                return score;
            }
        }
        
        const legalMoves = this.getLegalMoves();
        if (legalMoves.length === 0) {
            if (this.isInCheck(this.currentPlayer)) {
                return color * (-100000 + depth); // Scaccomatto (meglio se più profondo)
            }
            return 0; // Stallo
        }
        
        // Mossa dalla TT se disponibile
        let ttMove = null;
        if (ttEntry && ttEntry.move) {
            ttMove = ttEntry.move;
        }
        
        // Ordina mosse
        const orderedMoves = this.orderMoves(legalMoves, depth, ttMove);
        
        let bestScore = -Infinity;
        let bestMove = null;
        let movesSearched = 0;
        
        for (const move of orderedMoves) {
            // Esegui mossa
            const piece = this.board[move.fromRow][move.fromCol];
            const captured = this.board[move.toRow]?.[move.toCol];
            
            this.board[move.toRow][move.toCol] = piece;
            this.board[move.fromRow][move.fromCol] = null;
            
            if (move.promotion !== PieceType.NONE) {
                this.board[move.toRow][move.toCol] = new Piece(move.promotion, piece.color);
            }
            
            const oldPlayer = this.currentPlayer;
            this.currentPlayer = this.currentPlayer === Color.WHITE ? Color.BLACK : Color.WHITE;
            
            let score;
            if (movesSearched === 0) {
                // Ricerca completa per prima mossa
                score = -this.negamax(depth - 1, -beta, -alpha, -color);
            } else {
                // Late Move Reduction (LMR)
                let reduction = 0;
                if (depth >= 3 && movesSearched >= 4 && !move.isCapture && move.promotion === PieceType.NONE) {
                    reduction = 1;
                }
                
                // Principal Variation Search (PVS)
                score = -this.negamax(depth - 1 - reduction, -alpha - 1, -alpha, -color);
                if (score > alpha && score < beta) {
                    score = -this.negamax(depth - 1, -beta, -alpha, -color);
                }
            }
            
            // Annulla mossa
            this.currentPlayer = oldPlayer;
            this.board[move.fromRow][move.fromCol] = piece;
            this.board[move.toRow][move.toCol] = captured;
            
            if (score > bestScore) {
                bestScore = score;
                bestMove = move;
                
                if (score > alpha) {
                    alpha = score;
                    
                    // Update history heuristic
                    this.updateHistoryHeuristic(piece, move, depth);
                }
                
                if (alpha >= beta) {
                    // Update killer moves
                    this.updateKillerMove(move, depth);
                    break;
                }
            }
            
            movesSearched++;
        }
        
        // Store in transposition table
        let flag = TTFlag.EXACT;
        if (bestScore <= originalAlpha) {
            flag = TTFlag.UPPERBOUND;
        } else if (bestScore >= beta) {
            flag = TTFlag.LOWERBOUND;
        }
        
        this.tt.store(ttKey, depth, bestScore, bestMove, flag);
        
        return bestScore;
    }

    getZobristKey() {
        // Implementazione semplice di Zobrist hashing
        let key = 0;
        for (let row = 0; row < 8; row++) {
            for (let col = 0; col < 8; col++) {
                const piece = this.board[row][col];
                if (piece) {
                    const pieceIndex = piece.type * 2 + piece.color;
                    const squareIndex = row * 8 + col;
                    key ^= (pieceIndex + 1) * (squareIndex + 1) * 2654435761;
                }
            }
        }
        
        // Aggiungi turno
        if (this.currentPlayer === Color.BLACK) {
            key ^= 0x12345678;
        }
        
        // Aggiungi diritti di arrocco
        if (this.castlingRights[Color.WHITE].kingside) key ^= 0x11111111;
        if (this.castlingRights[Color.WHITE].queenside) key ^= 0x22222222;
        if (this.castlingRights[Color.BLACK].kingside) key ^= 0x33333333;
        if (this.castlingRights[Color.BLACK].queenside) key ^= 0x44444444;
        
        // Aggiungi en passant
        if (this.enPassantTarget) {
            key ^= (this.enPassantTarget.row * 8 + this.enPassantTarget.col + 1) * 0x55555555;
        }
        
        return key;
    }

    search(depth = 4, maxTime = 10000) {
        console.log(`🤖 Computer pensa a profondità ${depth}...`);
        
        const startTime = Date.now();
        
        // Controlla opening book
        if (this.fullMoveNumber <= 8) {
            const bookMove = this.openingBook.getMove(this.getFEN());
            if (bookMove) {
                console.log('📚 Opening book move');
                return this.parseMove(bookMove);
            }
        }
        
        const legalMoves = this.getLegalMoves();
        if (legalMoves.length === 0) return null;
        
        // Reset heuristics per nuova ricerca
        for (let i = 0; i < this.killerMoves.length; i++) {
            this.killerMoves[i] = [null, null];
        }
        
        let bestMove = null;
        let bestScore = this.currentPlayer === Color.WHITE ? -Infinity : Infinity;
        
        // Principal Variation Search
        for (let i = 0; i < legalMoves.length; i++) {
            const move = legalMoves[i];
            
            // Time management
            if (Date.now() - startTime > maxTime) {
                console.log('⏰ Tempo scaduto!');
                break;
            }
            
            // Esegui la mossa
            const piece = this.board[move.fromRow][move.fromCol];
            const captured = this.board[move.toRow]?.[move.toCol];
            
            this.board[move.toRow][move.toCol] = piece;
            this.board[move.fromRow][move.fromCol] = null;
            
            if (move.promotion !== PieceType.NONE) {
                this.board[move.toRow][move.toCol] = new Piece(move.promotion, piece.color);
            }
            
            const oldPlayer = this.currentPlayer;
            this.currentPlayer = this.currentPlayer === Color.WHITE ? Color.BLACK : Color.WHITE;
            
            let score;
            if (i === 0) {
                // Prima mossa: ricerca completa
                score = this.negamax(depth - 1, -Infinity, Infinity, 1);
            } else {
                // Altre mosse: ricerca con finestra zero
                score = this.negamax(depth - 1, -bestScore - 1, -bestScore, 1);
                
                // Se fallisce alta, ricerca completa
                if (score > bestScore) {
                    score = this.negamax(depth - 1, -Infinity, -bestScore, 1);
                }
            }
            
            // Annulla mossa
            this.currentPlayer = oldPlayer;
            this.board[move.fromRow][move.fromCol] = piece;
            this.board[move.toRow][move.toCol] = captured;
            
            if (this.currentPlayer === Color.WHITE) {
                if (score > bestScore) {
                    bestScore = score;
                    bestMove = move;
                }
            } else {
                if (score < bestScore) {
                    bestScore = score;
                    bestMove = move;
                }
            }
            
            // Output parziale
            if (i < 3) {
                console.log(`  ${move.toString()}: ${score}`);
            }
        }
        
        const timeUsed = Date.now() - startTime;
        console.log(`🤖 Migliore mossa: ${bestMove ? bestMove.toString() : 'Nessuna'} (valutazione: ${bestScore})`);
        console.log(`⏱️  Tempo impiegato: ${timeUsed}ms`);
        
        return bestMove;
    }

    iterativeDeepening(maxDepth = 6, maxTime = 5000) {
        const startTime = Date.now();
        let bestMove = null;
        let bestScore = this.currentPlayer === Color.WHITE ? -Infinity : Infinity;
        
        for (let depth = 1; depth <= maxDepth; depth++) {
            if (Date.now() - startTime > maxTime * 0.9) {
                break; // Riserva il 10% del tempo per sicurezza
            }
            
            console.log(`🔍 Ricerca a profondità ${depth}...`);
            const move = this.search(depth, maxTime - (Date.now() - startTime));
            
            if (move) {
                bestMove = move;
                
                // Aggiorna punteggio migliore
                const tempEngine = new CompleteChessEngine(this.getFEN());
                tempEngine.makeMoveInternal(move, false);
                const score = tempEngine.evaluate();
                
                if (this.currentPlayer === Color.WHITE) {
                    if (score > bestScore) bestScore = score;
                } else {
                    if (score < bestScore) bestScore = score;
                }
            }
            
            if (this.isGameOver()) {
                break;
            }
        }
        
        return bestMove;
    }

    // ============= ANALISI E UTILITY =============
    analyzePosition() {
        console.log('\n🔍 Analisi posizione:');
        
        const score = this.evaluate();
        console.log(`💰 Valutazione: ${score > 0 ? '+' : ''}${score}`);
        
        const legalMoves = this.getLegalMoves();
        console.log(`📋 Mosse legali: ${legalMoves.length}`);
        
        // Analisi materiale
        let whiteMaterial = 0;
        let blackMaterial = 0;
        let whitePieces = [];
        let blackPieces = [];
        
        for (let row = 0; row < 8; row++) {
            for (let col = 0; col < 8; col++) {
                const piece = this.board[row][col];
                if (piece) {
                    if (piece.color === Color.WHITE) {
                        whiteMaterial += piece.getValue();
                        whitePieces.push(piece);
                    } else {
                        blackMaterial += piece.getValue();
                        blackPieces.push(piece);
                    }
                }
            }
        }
        
        console.log(`⚪ Materiale bianco: ${whiteMaterial}`);
        console.log(`⚫ Materiale nero: ${blackMaterial}`);
        console.log(`📊 Vantaggio materiale: ${whiteMaterial - blackMaterial > 0 ? '+' : ''}${whiteMaterial - blackMaterial}`);
        
        // Controllo del centro
        const centerControl = this.evaluateCenterControl(Color.WHITE) - this.evaluateCenterControl(Color.BLACK);
        console.log(`🎯 Controllo centro: ${centerControl > 0 ? '+' : ''}${centerControl}`);
        
        // Sviluppo
        if (this.fullMoveNumber < 20) {
            const development = this.getDevelopmentScore(Color.WHITE) - this.getDevelopmentScore(Color.BLACK);
            console.log(`♟️  Sviluppo: ${development > 0 ? '+' : ''}${development}`);
        }
        
        // Fase di gioco
        const gamePhase = this.getGamePhase();
        console.log(`🌓 Fase di gioco: ${gamePhase < 0.3 ? 'Apertura' : gamePhase < 0.7 ? 'Mediogioco' : 'Finale'}`);
        
        // Suggerisci mosse
        if (legalMoves.length > 0) {
            console.log('\n🎯 Mosse consigliate:');
            
            const scoredMoves = legalMoves.map(move => {
                const tempEngine = new CompleteChessEngine(this.getFEN());
                tempEngine.makeMoveInternal(move, false);
                return {
                    move,
                    score: tempEngine.evaluate()
                };
            }).sort((a, b) => {
                return this.currentPlayer === Color.WHITE ? 
                    b.score - a.score : a.score - b.score;
            });
            
            for (let i = 0; i < Math.min(3, scoredMoves.length); i++) {
                const {move, score} = scoredMoves[i];
                // Controllo di sicurezza per il pezzo
                const piece = this.board[move.fromRow][move.fromCol];
                if (piece) {
                    const san = move.getSAN(piece, this.board, false, false);
                    console.log(`${i+1}. ${san} (${score > 0 ? '+' : ''}${score})`);
                } else {
                    console.log(`${i+1}. ${move.toString()} (${score > 0 ? '+' : ''}${score})`);
                }
            }
        }
        
        // Controllo scacchi
        if (this.isInCheck(Color.WHITE)) console.log('⚔️  Bianco in scacco!');
        if (this.isInCheck(Color.BLACK)) console.log('⚔️  Nero in scacco!');
    }

    saveGameToPGN() {
        let pgn = '[Event "Chess Game"]\n';
        pgn += '[Site "JavaScript Chess Engine"]\n';
        pgn += `[Date "${new Date().toISOString().split('T')[0]}"]\n`;
        pgn += '[Round "1"]\n';
        pgn += `[White "${this.currentPlayer === Color.BLACK ? 'Human' : 'Computer'}"]\n`;
        pgn += `[Black "${this.currentPlayer === Color.WHITE ? 'Human' : 'Computer'}"]\n`;
        pgn += `[Result "${this.getResult()}"]\n`;
        pgn += '[Setup "1"]\n';
        pgn += `[FEN "${this.getFEN()}"]\n\n`;
        
        // Aggiungi mosse
        for (let i = 0; i < this.moveSANHistory.length; i++) {
            if (i % 2 === 0) {
                pgn += `${Math.floor(i/2) + 1}. `;
            }
            pgn += `${this.moveSANHistory[i]} `;
        }
        
        pgn += this.getResult();
        
        return pgn;
    }

    // ============= VISUALIZZAZIONE =============
    printBoard(highlightMoves = []) {
        console.log('\n  a b c d e f g h');
        console.log('  ────────────────');
        
        for (let row = 0; row < 8; row++) {
            let line = `${8 - row} `;
            
            for (let col = 0; col < 8; col++) {
                const piece = this.board[row][col];
                if (piece) {
                    // Evidenzia caselle di partenza/arrivo delle mosse evidenziate
                    const isHighlighted = highlightMoves.some(move => 
                        (move.fromRow === row && move.fromCol === col) ||
                        (move.toRow === row && move.toCol === col)
                    );
                    
                    if (isHighlighted) {
                        line += `[${piece.getSymbol()}]`;
                    } else {
                        line += piece.getSymbol() + ' ';
                    }
                } else {
                    // Evidenzia caselle vuote che sono destinazioni di mosse
                    const isDestination = highlightMoves.some(move => 
                        move.toRow === row && move.toCol === col
                    );
                    
                    if (isDestination) {
                        line += '[ ]';
                    } else {
                        line += ((row + col) % 2 === 1 ? '· ' : '  ');
                    }
                }
            }
            
            line += ` ${8 - row}`;
            console.log(line);
        }
        
        console.log('  ────────────────');
        console.log('  a b c d e f g h\n');
        
        const player = this.currentPlayer === Color.WHITE ? "Bianco ♔" : "Nero ♚";
        console.log(`🎮 Turno: ${player}`);
        
        // Correzione per evitare FEN troppo lungo
        const fenParts = this.getFEN().split(' ');
        // Assicurati che il fullMoveNumber sia un numero valido
        const fullMoveNum = Math.max(1, parseInt(fenParts[5] || 1));
        const shortFEN = fenParts.slice(0, 4).join(' ') + ` ${fenParts[4] || 0} ${fullMoveNum}`;
        console.log(`📊 FEN: ${shortFEN}`);
        
        if (this.isInCheck(this.currentPlayer)) {
            console.log("⚔️  SCACCO!");
        }
        
        const legalMoves = this.getLegalMoves();
        console.log(`📋 Mosse legali: ${legalMoves.length}`);
        
        if (this.isGameOver()) {
            const result = this.getResult();
            console.log(`🏁 ${this.getGameOverReason()}: ${result}`);
        } else {
            const score = this.evaluate();
            console.log(`💰 Valutazione: ${score > 0 ? '+' : ''}${score}`);
            
            // Mostra prime 5 mosse possibili
            if (legalMoves.length > 0 && highlightMoves.length === 0) {
                console.log('🎯 Prime mosse possibili:');
                const preview = legalMoves.slice(0, 5).map(m => {
                    const piece = this.board[m.fromRow][m.fromCol];
                    // Controllo di sicurezza per piece null
                    if (piece) {
                        return m.getSAN(piece, this.board, false, false);
                    }
                    return m.toString();
                }).join(', ');
                console.log(`  ${preview}${legalMoves.length > 5 ? '...' : ''}`);
            }
        }
        
        // Mostra mosse evidenziate se presenti
        if (highlightMoves.length > 0) {
            console.log('🔍 Mosse evidenziate:');
            highlightMoves.forEach((move, i) => {
                const piece = this.board[move.fromRow][move.fromCol];
                // Controllo di sicurezza per piece null
                if (piece) {
                    const san = move.getSAN(piece, this.board, false, false);
                    console.log(`  ${i+1}. ${san} (${move.toString()})`);
                } else {
                    console.log(`  ${i+1}. ${move.toString()}`);
                }
            });
        }
        
        // Mostra history mosse recenti
        if (this.moveSANHistory.length > 0) {
            console.log('\n📜 Mosse recenti:');
            const recentMoves = this.moveSANHistory.slice(-6);
            let movesLine = '';
            for (let i = 0; i < recentMoves.length; i++) {
                if (i % 2 === 0) {
                    movesLine += `${Math.floor(i/2) + Math.max(0, this.moveSANHistory.length - 6) + 1}. `;
                }
                movesLine += `${recentMoves[i]} `;
            }
            console.log(`  ${movesLine.trim()}`);
        }
    }
}

// ============= GIOCO COMPLETO =============
class CompleteChessGame {
    constructor() {
        this.engine = new CompleteChessEngine();
        this.rl = readline.createInterface({
            input: process.stdin,
            output: process.stdout
        });
        this.isGameRunning = true;
        this.computerColor = Color.BLACK;
        this.difficulty = 3; // 1-5
        this.gameMode = 'player-vs-computer';
        this.highlightedMoves = [];
    }

    start() {
        console.clear();
        console.log('╔══════════════════════════════════════════════════════╗');
        console.log('║              ♔ COMPLETE CHESS ENGINE ♚             ║');
        console.log('║     Con tutte le feature avanzate implementate     ║');
        console.log('║  Promozione • En passant • Arrocco • IA avanzata   ║');
        console.log('╚══════════════════════════════════════════════════════╝\n');
        
        this.showMenu();
    }

    showMenu() {
        console.log('\n📋 MENU PRINCIPALE:');
        console.log('  1. Nuova partita (Bianco vs Computer)');
        console.log('  2. Nuova partita (Computer vs Nero)');
        console.log('  3. Partita tra umani');
        console.log('  4. Computer vs Computer (demo)');
        console.log('  5. Carica posizione da FEN');
        console.log('  6. Analizza posizione corrente');
        console.log('  7. Imposta difficoltà (attuale: ' + this.difficulty + ')');
        console.log('  8. Esporta partita in PGN');
        console.log('  9. Esci');
        
        this.rl.question('\n🔹 Scegli un\'opzione: ', (choice) => {
            switch (choice) {
                case '1':
                    this.computerColor = Color.BLACK;
                    this.gameMode = 'player-vs-computer';
                    this.newGame();
                    break;
                case '2':
                    this.computerColor = Color.WHITE;
                    this.gameMode = 'player-vs-computer';
                    this.newGame();
                    break;
                case '3':
                    this.gameMode = 'player-vs-player';
                    this.newGame();
                    break;
                case '4':
                    this.gameMode = 'computer-vs-computer';
                    this.computerVsComputer();
                    break;
                case '5':
                    this.loadFENPosition();
                    break;
                case '6':
                    this.engine.analyzePosition();
                    setTimeout(() => {
                        this.showMenu();
                    }, 1000);
                    break;
                case '7':
                    this.setDifficulty();
                    break;
                case '8':
                    this.exportPGN();
                    break;
                case '9':
                    this.isGameRunning = false;
                    this.rl.close();
                    break;
                default:
                    console.log('❌ Scelta non valida');
                    this.showMenu();
            }
        });
    }

    newGame() {
        this.engine = new CompleteChessEngine();
        this.highlightedMoves = [];
        console.log('\n♔ Nuova partita iniziata! ♚');
        console.log(`Modalità: ${this.gameMode}`);
        console.log(`Difficoltà: ${this.difficulty}`);
        
        if (this.gameMode === 'computer-vs-computer') {
            this.computerVsComputer();
        } else {
            this.gameLoop();
        }
    }

    gameLoop() {
        if (!this.isGameRunning) {
            console.log('\n🎉 Grazie per aver giocato!');
            this.rl.close();
            return;
        }
        
        this.engine.printBoard(this.highlightedMoves);
        
        if (this.engine.isGameOver()) {
            this.showGameResult();
            return;
        }
        
        if (this.gameMode === 'player-vs-player') {
            this.humanTurn();
        } else if (this.gameMode === 'player-vs-computer') {
            if (this.engine.currentPlayer === this.computerColor) {
                this.computerTurn();
            } else {
                this.humanTurn();
            }
        }
    }

    humanTurn() {
        console.log('\n🎮 La tua mossa (es: e2e4, O-O, e7e8q):');
        console.log('   ? = mostra mosse, h <casella> = evidenzia mosse');
        console.log('   undo = annulla, menu = ritorna al menu');
        
        this.rl.question('> ', (input) => {
            this.processInput(input.trim());
        });
    }

    computerTurn() {
        console.log('\n🤖 Computer pensa...');
        
        const depth = this.difficulty + 2; // 3-7
        const maxTime = this.difficulty * 1500; // 1.5-7.5 secondi
        
        setTimeout(() => {
            const bestMove = this.engine.iterativeDeepening(depth, maxTime);
            
            if (bestMove) {
                const moveStr = bestMove.toString();
                console.log(`🤖 Computer gioca: ${moveStr}`);
                
                if (this.engine.makeMove(moveStr)) {
                    this.highlightedMoves = [bestMove];
                    setTimeout(() => {
                        this.gameLoop();
                    }, 800);
                } else {
                    console.log('❌ ERRORE: Mossa del computer illegale!');
                    // Prova mossa casuale
                    const legalMoves = this.engine.getLegalMoves();
                    if (legalMoves.length > 0) {
                        const randomMove = legalMoves[Math.floor(Math.random() * legalMoves.length)];
                        console.log(`🤖 Provo mossa casuale: ${randomMove.toString()}`);
                        this.engine.makeMove(randomMove.toString());
                        this.highlightedMoves = [randomMove];
                        setTimeout(() => {
                            this.gameLoop();
                        }, 800);
                    } else {
                        this.gameLoop();
                    }
                }
            } else {
                console.log('🤖 Computer non ha mosse disponibili');
                this.gameLoop();
            }
        }, 500);
    }

    computerVsComputer() {
        console.log('\n🤖🤖 Modalità Computer vs Computer');
        console.log('Premi Ctrl+C per interrompere\n');
        
        let moveCount = 0;
        const maxMoves = 150;
        
        const playNextMove = () => {
            if (moveCount >= maxMoves || this.engine.isGameOver() || !this.isGameRunning) {
                this.showGameResult();
                this.showMenu();
                return;
            }
            
            this.engine.printBoard(this.highlightedMoves);
            
            const depth = 3;
            const bestMove = this.engine.search(depth);
            
            if (bestMove) {
                const moveStr = bestMove.toString();
                const piece = this.engine.board[bestMove.fromRow][bestMove.fromCol];
                let san;
                if (piece) {
                    san = bestMove.getSAN(piece, this.engine.board, false, false);
                } else {
                    san = moveStr;
                }
                
                console.log(`\n🤖 Mossa ${moveCount + 1}: ${san} (${moveStr})`);
                
                if (this.engine.makeMove(moveStr)) {
                    moveCount++;
                    this.highlightedMoves = [bestMove];
                    
                    // Pausa per leggibilità
                    setTimeout(playNextMove, 1200);
                } else {
                    console.log('❌ Mossa illegale!');
                    this.showMenu();
                }
            } else {
                console.log('🤖 Nessuna mossa disponibile');
                this.showGameResult();
                this.showMenu();
            }
        };
        
        playNextMove();
    }

    processInput(input) {
        if (!input) {
            this.gameLoop();
            return;
        }
        
        const lowerInput = input.toLowerCase();
        
        switch(lowerInput) {
            case 'quit':
            case 'exit':
                this.isGameRunning = false;
                this.gameLoop();
                return;
                
            case 'menu':
                this.showMenu();
                return;
                
            case 'new':
                this.newGame();
                return;
                
            case 'undo':
                if (this.engine.undoMove()) {
                    console.log('↩️  Mossa annullata');
                    this.highlightedMoves = [];
                    this.gameLoop();
                } else {
                    console.log('❌ Nessuna mossa da annullare');
                    this.gameLoop();
                }
                return;
                
            case '?':
            case 'help':
                this.showAvailableMoves();
                return;
                
            case 'fen':
                console.log(`📊 FEN attuale: ${this.engine.getFEN()}`);
                this.gameLoop();
                return;
                
            case 'analyze':
            case 'eval':
                this.engine.analyzePosition();
                setTimeout(() => {
                    this.gameLoop();
                }, 2000);
                return;
                
            default:
                // Controlla se è un comando speciale
                if (input.startsWith('/')) {
                    this.processCommand(input.slice(1));
                    return;
                }
                
                // Controlla se è un comando di highlight
                if (input.startsWith('h ') || input.startsWith('H ')) {
                    const square = input.slice(2).trim();
                    this.highlightMovesFromSquare(square);
                    return;
                }
                
                // Esegui mossa
                if (this.engine.makeMove(input)) {
                    console.log(`✅ Mossa eseguita: ${input}`);
                    this.highlightedMoves = [];
                    
                    setTimeout(() => {
                        this.gameLoop();
                    }, 500);
                } else {
                    console.log('❌ Mossa illegale!');
                    
                    // Suggerisci mosse simili
                    this.suggestMoves(input);
                }
        }
    }

    processCommand(cmd) {
        const parts = cmd.split(' ');
        const command = parts[0].toLowerCase();
        
        switch(command) {
            case 'fen':
                if (parts.length > 1) {
                    const fen = parts.slice(1).join(' ');
                    if (this.engine.loadFEN(fen)) {
                        console.log('✅ Posizione caricata da FEN');
                        this.highlightedMoves = [];
                        this.gameLoop();
                    } else {
                        console.log('❌ FEN non valido');
                        this.gameLoop();
                    }
                } else {
                    console.log('Usage: /fen <fen-string>');
                    this.gameLoop();
                }
                break;
                
            case 'pgn':
                const pgn = this.engine.saveGameToPGN();
                console.log('\n📄 PGN della partita:');
                console.log('='.repeat(50));
                console.log(pgn);
                console.log('='.repeat(50));
                this.gameLoop();
                break;
                
            default:
                console.log('❌ Comando non riconosciuto');
                console.log('Comandi disponibili: /fen, /pgn');
                this.gameLoop();
        }
    }

    highlightMovesFromSquare(square) {
        if (square.length < 2) {
            console.log('❌ Casella non valida. Esempio: h e2');
            this.gameLoop();
            return;
        }
        
        const cols = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h'];
        const col = cols.indexOf(square[0].toLowerCase());
        const row = 8 - parseInt(square[1]);
        
        if (col < 0 || row < 0 || row > 7) {
            console.log('❌ Casella non valida');
            this.gameLoop();
            return;
        }
        
        const piece = this.engine.board[row][col];
        if (!piece || piece.color !== this.engine.currentPlayer) {
            console.log('❌ Nessun tuo pezzo in quella casella');
            this.gameLoop();
            return;
        }
        
        // Trova tutte le mosse da quella casella
        const legalMoves = this.engine.getLegalMoves();
        this.highlightedMoves = legalMoves.filter(move => 
            move.fromRow === row && move.fromCol === col
        );
        
        if (this.highlightedMoves.length === 0) {
            console.log('❌ Nessuna mossa disponibile da quella casella');
            this.highlightedMoves = [];
        } else {
            console.log(`🔍 ${this.highlightedMoves.length} mosse disponibili da ${square}`);
        }
        
        this.gameLoop();
    }

    suggestMoves(badMove) {
        const legalMoves = this.engine.getLegalMoves();
        
        if (legalMoves.length === 0) {
            console.log('⚠️  Non ci sono mosse legali disponibili!');
            return;
        }
        
        console.log('📋 Mosse legali disponibili:');
        
        // Cerca mosse simili
        let similarMoves = [];
        const cols = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h'];
        
        if (badMove.length >= 2) {
            const fromChar = badMove[0].toLowerCase();
            const fromRow = parseInt(badMove[1]);
            
            similarMoves = legalMoves.filter(move => {
                const fromCol = cols[move.fromCol];
                const moveFromRow = 8 - move.fromRow;
                return fromCol === fromChar && moveFromRow === fromRow;
            });
        }
        
        if (similarMoves.length > 0) {
            console.log(`Mosse dalla casella ${badMove.slice(0, 2)}:`);
            const moveStrs = similarMoves.slice(0, 8).map(m => {
                const piece = this.engine.board[m.fromRow][m.fromCol];
                if (piece) {
                    return m.getSAN(piece, this.engine.board, false, false);
                }
                return m.toString();
            });
            console.log('  ' + moveStrs.join(', '));
        } else {
            const moveStrs = legalMoves.slice(0, 12).map(m => {
                const piece = this.engine.board[m.fromRow][m.fromCol];
                if (piece) {
                    return m.getSAN(piece, this.engine.board, false, false);
                }
                return m.toString();
            });
            console.log('  ' + moveStrs.join(', '));
            if (legalMoves.length > 12) {
                console.log(`  ... e altre ${legalMoves.length - 12} mosse`);
            }
        }
        
        this.gameLoop();
    }

    showAvailableMoves() {
        const legalMoves = this.engine.getLegalMoves();
        
        if (legalMoves.length === 0) {
            console.log('⚠️  Non ci sono mosse legali disponibili!');
        } else {
            console.log(`📋 ${legalMoves.length} mosse legali disponibili:`);
            
            // Raggruppa per pezzo
            const movesByPiece = {};
            
            for (const move of legalMoves) {
                const piece = this.engine.board[move.fromRow][move.fromCol];
                if (piece) {
                    const pieceSymbol = piece.getSymbol();
                    const san = move.getSAN(piece, this.engine.board, false, false);
                    
                    if (!movesByPiece[pieceSymbol]) {
                        movesByPiece[pieceSymbol] = [];
                    }
                    
                    movesByPiece[pieceSymbol].push(san);
                }
            }
            
            for (const [piece, moves] of Object.entries(movesByPiece)) {
                console.log(`  ${piece}: ${moves.slice(0, 6).join(', ')}${moves.length > 6 ? '...' : ''}`);
            }
        }
        
        this.gameLoop();
    }

    showGameResult() {
        const result = this.engine.getResult();
        const reason = this.engine.getGameOverReason();
        
        console.log('\n' + '═'.repeat(50));
        console.log(`🏁 ${reason}`);
        console.log(`📊 Risultato finale: ${result}`);
        console.log('═'.repeat(50));
        
        this.engine.printBoard();
        
        this.rl.question('\n🔄 Premere Invio per tornare al menu... ', () => {
            this.showMenu();
        });
    }

    loadFENPosition() {
        console.log('\n📝 Inserisci la posizione in notazione FEN:');
        console.log('Esempio: rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1');
        console.log('Lascia vuoto per la posizione iniziale');
        
        this.rl.question('FEN: ', (fen) => {
            if (!fen.trim()) {
                fen = 'rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1';
            }
            
            if (this.engine.loadFEN(fen)) {
                console.log('✅ Posizione caricata con successo!');
                this.gameMode = 'player-vs-player';
                this.highlightedMoves = [];
                this.gameLoop();
            } else {
                console.log('❌ FEN non valido!');
                this.loadFENPosition();
            }
        });
    }

    exportPGN() {
        const pgn = this.engine.saveGameToPGN();
        console.log('\n📄 PGN della partita:');
        console.log('='.repeat(60));
        console.log(pgn);
        console.log('='.repeat(60));
        
        this.rl.question('\n📋 Copia il testo sopra (Premi Invio per continuare)... ', () => {
            this.showMenu();
        });
    }

    setDifficulty() {
        console.log('\n🎯 Imposta difficoltà del computer:');
        console.log('  1. Principiante (profondità 3, 1.5s)');
        console.log('  2. Intermedio (profondità 4, 3s)');
        console.log('  3. Avanzato (profondità 5, 4.5s)');
        console.log('  4. Esperto (profondità 6, 6s)');
        console.log('  5. Maestro (profondità 7, 7.5s)');
        
        this.rl.question('\nScegli livello (1-5): ', (level) => {
            const newLevel = parseInt(level);
            
            if (newLevel >= 1 && newLevel <= 5) {
                this.difficulty = newLevel;
                console.log(`✅ Difficoltà impostata a ${newLevel}`);
            } else {
                console.log('❌ Livello non valido');
            }
            
            this.showMenu();
        });
    }
}

// ============= AVVIA GIOCO =============
if (require.main === module) {
    const game = new CompleteChessGame();
    game.start();
}

module.exports = { CompleteChessEngine, CompleteChessGame, PieceType, Color };