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

// ============= BITBOARD CORRETTO =============
class Bitboard {
    constructor() {
        this.reset();
    }
    
    reset() {
        this.pawns = [0n, 0n];
        this.knights = [0n, 0n];
        this.bishops = [0n, 0n];
        this.rooks = [0n, 0n];
        this.queens = [0n, 0n];
        this.kings = [0n, 0n];
        
        this.occupancy = [0n, 0n];
        this.allPieces = 0n;
        
        this.initializeMasks();
    }
    
    initializeMasks() {
        this.knightMasks = new Array(64).fill(0n);
        this.kingMasks = new Array(64).fill(0n);
        
        for (let square = 0; square < 64; square++) {
            this.knightMasks[square] = this.calculateKnightMask(square);
            this.kingMasks[square] = this.calculateKingMask(square);
        }
    }
    
    squareToBit(square) {
        if (square < 0 || square > 63) return 0n;
        return 1n << BigInt(square);
    }
    
    bitToSquare(bit) {
        if (bit === 0n) return -1;
        return Math.floor(Math.log2(Number(bit)));
    }
    
    popLsb(bitboard) {
        if (bitboard === 0n) return { square: -1, remaining: 0n };
        const lsb = bitboard & -bitboard;
        const square = this.bitToSquare(lsb);
        return { square, remaining: bitboard & (bitboard - 1n) };
    }
    
    countBits(bitboard) {
        let count = 0;
        let bb = bitboard;
        while (bb) {
            bb &= bb - 1n;
            count++;
        }
        return count;
    }
    
    forEachSquare(bitboard, callback) {
        let bb = bitboard;
        while (bb !== 0n) {
            const { square, remaining } = this.popLsb(bb);
            if (square >= 0) {
                callback(square);
            }
            bb = remaining;
        }
    }
    
    setSquare(color, pieceType, square) {
        const bit = this.squareToBit(square);
        
        this.clearSquare(square);
        
        switch(pieceType) {
            case PieceType.PAWN: this.pawns[color] |= bit; break;
            case PieceType.KNIGHT: this.knights[color] |= bit; break;
            case PieceType.BISHOP: this.bishops[color] |= bit; break;
            case PieceType.ROOK: this.rooks[color] |= bit; break;
            case PieceType.QUEEN: this.queens[color] |= bit; break;
            case PieceType.KING: this.kings[color] |= bit; break;
        }
        
        this.occupancy[color] |= bit;
        this.allPieces |= bit;
        
        return true;
    }
    
    clearSquare(square) {
        const bit = this.squareToBit(square);
        const notBit = ~bit;
        
        for (let color = 0; color < 2; color++) {
            this.pawns[color] &= notBit;
            this.knights[color] &= notBit;
            this.bishops[color] &= notBit;
            this.rooks[color] &= notBit;
            this.queens[color] &= notBit;
            this.kings[color] &= notBit;
            this.occupancy[color] &= notBit;
        }
        
        this.allPieces &= notBit;
        
        return true;
    }
    
    moveSquare(fromSquare, toSquare) {
        const fromBit = this.squareToBit(fromSquare);
        const toBit = this.squareToBit(toSquare);
        
        let pieceType = PieceType.NONE;
        let color = Color.NONE;
        
        for (let c = 0; c < 2; c++) {
            if (this.pawns[c] & fromBit) { pieceType = PieceType.PAWN; color = c; break; }
            if (this.knights[c] & fromBit) { pieceType = PieceType.KNIGHT; color = c; break; }
            if (this.bishops[c] & fromBit) { pieceType = PieceType.BISHOP; color = c; break; }
            if (this.rooks[c] & fromBit) { pieceType = PieceType.ROOK; color = c; break; }
            if (this.queens[c] & fromBit) { pieceType = PieceType.QUEEN; color = c; break; }
            if (this.kings[c] & fromBit) { pieceType = PieceType.KING; color = c; break; }
        }
        
        if (pieceType === PieceType.NONE) {
            return false;
        }
        
        this.clearSquare(toSquare);
        
        const notFromBit = ~fromBit;
        switch(pieceType) {
            case PieceType.PAWN: this.pawns[color] &= notFromBit; break;
            case PieceType.KNIGHT: this.knights[color] &= notFromBit; break;
            case PieceType.BISHOP: this.bishops[color] &= notFromBit; break;
            case PieceType.ROOK: this.rooks[color] &= notFromBit; break;
            case PieceType.QUEEN: this.queens[color] &= notFromBit; break;
            case PieceType.KING: this.kings[color] &= notFromBit; break;
        }
        this.occupancy[color] &= notFromBit;
        this.allPieces &= notFromBit;
        
        switch(pieceType) {
            case PieceType.PAWN: this.pawns[color] |= toBit; break;
            case PieceType.KNIGHT: this.knights[color] |= toBit; break;
            case PieceType.BISHOP: this.bishops[color] |= toBit; break;
            case PieceType.ROOK: this.rooks[color] |= toBit; break;
            case PieceType.QUEEN: this.queens[color] |= toBit; break;
            case PieceType.KING: this.kings[color] |= toBit; break;
        }
        this.occupancy[color] |= toBit;
        this.allPieces |= toBit;
        
        return true;
    }
    
    getPieceAt(square) {
        const bit = this.squareToBit(square);
        
        for (let color = 0; color < 2; color++) {
            if (this.pawns[color] & bit) return { color, type: PieceType.PAWN };
            if (this.knights[color] & bit) return { color, type: PieceType.KNIGHT };
            if (this.bishops[color] & bit) return { color, type: PieceType.BISHOP };
            if (this.rooks[color] & bit) return { color, type: PieceType.ROOK };
            if (this.queens[color] & bit) return { color, type: PieceType.QUEEN };
            if (this.kings[color] & bit) return { color, type: PieceType.KING };
        }
        
        return null;
    }
    
    calculateKnightMask(square) {
        let mask = 0n;
        const row = Math.floor(square / 8);
        const col = square % 8;
        
        const moves = [
            [-2, -1], [-2, 1],
            [-1, -2], [-1, 2],
            [1, -2], [1, 2],
            [2, -1], [2, 1]
        ];
        
        for (const [dr, dc] of moves) {
            const newRow = row + dr;
            const newCol = col + dc;
            if (newRow >= 0 && newRow < 8 && newCol >= 0 && newCol < 8) {
                mask |= this.squareToBit(newRow * 8 + newCol);
            }
        }
        
        return mask;
    }
    
    calculateKingMask(square) {
        let mask = 0n;
        const row = Math.floor(square / 8);
        const col = square % 8;
        
        for (let dr = -1; dr <= 1; dr++) {
            for (let dc = -1; dc <= 1; dc++) {
                if (dr === 0 && dc === 0) continue;
                const newRow = row + dr;
                const newCol = col + dc;
                if (newRow >= 0 && newRow < 8 && newCol >= 0 && newCol < 8) {
                    mask |= this.squareToBit(newRow * 8 + newCol);
                }
            }
        }
        
        return mask;
    }
    
    getPawnAttacksMask(square, color) {
        let mask = 0n;
        const row = Math.floor(square / 8);
        const col = square % 8;
        
        if (color === Color.WHITE) {
            if (row > 0 && col > 0) mask |= this.squareToBit(square - 9);
            if (row > 0 && col < 7) mask |= this.squareToBit(square - 7);
        } else {
            if (row < 7 && col > 0) mask |= this.squareToBit(square + 7);
            if (row < 7 && col < 7) mask |= this.squareToBit(square + 9);
        }
        
        return mask;
    }
    
    getKnightAttacksMask(square) {
        return this.knightMasks[square] || 0n;
    }
    
    getKingAttacksMask(square) {
        return this.kingMasks[square] || 0n;
    }
}

// ============= MOSSA CON CORREZIONI =============
class Move {
    constructor(fromRow, fromCol, toRow, toCol, promotion = PieceType.NONE, isCastling = false, isEnPassant = false) {
        this.fromRow = fromRow;
        this.fromCol = fromCol;
        this.toRow = toRow;
        this.toCol = toCol;
        this.fromSquare = fromRow * 8 + fromCol;
        this.toSquare = toRow * 8 + toCol;
        this.promotion = promotion;
        this.isCastling = isCastling;
        this.isEnPassant = isEnPassant;
        this.isCapture = false;
        this.capturedPiece = null;
        this.capturedPieceType = PieceType.NONE;
        this.score = 0;
    }

    equals(other) {
        if (!other) return false;
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
            return this.toCol === 6 ? 'e1g1' : 'e1c1';
        }
        
        let result = from + to;
        
        if (this.promotion !== PieceType.NONE) {
            const promSymbols = ['', 'n', 'b', 'r', 'q', ''];
            result += promSymbols[this.promotion];
        }
        
        return result;
    }

    getSAN(piece, board, isCheck = false, isMate = false) {
        if (!piece || piece.type === undefined) {
            return this.toString();
        }
        
        if (this.isCastling) {
            return this.toCol === 6 ? 'O-O' : 'O-O-O';
        }
        
        const pieceSymbol = piece.type !== PieceType.PAWN ? 
            ['', 'N', 'B', 'R', 'Q', 'K'][piece.type] : '';
        const capture = this.isCapture || this.isEnPassant ? 'x' : '';
        const cols = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h'];
        const destination = cols[this.toCol] + (8 - this.toRow);
        const promotion = this.promotion !== PieceType.NONE ? 
            '=' + ['', 'N', 'B', 'R', 'Q'][this.promotion] : '';
        
        let san = pieceSymbol;
        
        if (piece.type === PieceType.PAWN && capture) {
            san += cols[this.fromCol];
        }
        
        san += capture + destination + promotion;
        
        if (isMate) san += '#';
        else if (isCheck) san += '+';
        
        return san;
    }
}

// ============= PEZZO =============
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
        const symbols = {
            [Color.WHITE]: {
                [PieceType.PAWN]: '♙',
                [PieceType.KNIGHT]: '♘',
                [PieceType.BISHOP]: '♗',
                [PieceType.ROOK]: '♖',
                [PieceType.QUEEN]: '♕',
                [PieceType.KING]: '♔'
            },
            [Color.BLACK]: {
                [PieceType.PAWN]: '♟',
                [PieceType.KNIGHT]: '♞',
                [PieceType.BISHOP]: '♝',
                [PieceType.ROOK]: '♜',
                [PieceType.QUEEN]: '♛',
                [PieceType.KING]: '♚'
            }
        };
        return symbols[this.color][this.type] || '？';
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

// ============= MOTORE SCACCHI CORRETTO =============
class StrongChessEngine {
    constructor(fen = 'rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1') {
        this.board = Array(8).fill().map(() => Array(8).fill(null));
        this.bitboard = new Bitboard();
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
        
        this.nodesSearched = 0;
        this.searchTimeLimit = 5000;
        this.startSearchTime = 0;
        this.bestMoveSoFar = null;
        this.principalVariation = [];
        this.tt = new Map();
        this.killerMoves = Array(100).fill().map(() => [null, null]);
        this.historyHeuristic = Array(2).fill().map(() => 
            Array(6).fill().map(() => 
                Array(64).fill(0)
            )
        );
        
        this.initializeAdvancedPieceSquareTables();
        this.initializeOpeningBook();
        
        this.evaluationWeights = {
            material: 1.0,
            mobility: 0.1,
            pawnStructure: 0.3,
            kingSafety: 0.5,
            centerControl: 0.2,
            development: 0.2,
            bishopPair: 0.5,
            rookOnOpenFile: 0.3,
            rookOnSemiOpenFile: 0.2,
            connectedRooks: 0.2,
            knightOutpost: 0.3,
            passedPawn: 0.5,
            isolatedPawn: -0.2,
            doubledPawn: -0.15,
            backwardPawn: -0.1
        };
        
        if (fen) {
            this.loadFEN(fen);
        } else {
            this.initializeBoard();
        }
    }
    
    initializeAdvancedPieceSquareTables() {
        this.pieceSquareTablesMG = {
            [PieceType.PAWN]: [
                [  0,   0,   0,   0,   0,   0,   0,   0],
                [ 50,  50,  50,  50,  50,  50,  50,  50],
                [ 10,  10,  20,  30,  30,  20,  10,  10],
                [  5,   5,  10,  45,  45,  10,   5,   5],
                [  0,   0,   0,  30,  30,   0,   0,   0],
                [  5,  -5, -10,   0,   0, -10,  -5,   5],
                [  5,  10,  10, -30, -30,  10,  10,   5],
                [  0,   0,   0,   0,   0,   0,   0,   0]
            ],
            [PieceType.KNIGHT]: [
                [-50, -40, -30, -30, -30, -30, -40, -50],
                [-40, -20,   0,   5,   5,   0, -20, -40],
                [-30,   5,  15,  20,  20,  15,   5, -30],
                [-30,   0,  15,  20,  20,  15,   0, -30],
                [-30,   5,  15,  20,  20,  15,   5, -30],
                [-30,   0,  10,  15,  15,  10,   0, -30],
                [-40, -20,   0,   0,   0,   0, -20, -40],
                [-50, -40, -30, -30, -30, -30, -40, -50]
            ],
            [PieceType.BISHOP]: [
                [-20, -10, -10, -10, -10, -10, -10, -20],
                [-10,   5,   0,   0,   0,   0,   5, -10],
                [-10,  10,  10,  10,  10,  10,  10, -10],
                [-10,   0,  10,  10,  10,  10,   0, -10],
                [-10,   5,   5,  10,  10,   5,   5, -10],
                [-10,   0,   5,  10,  10,   5,   0, -10],
                [-10,   0,   0,   0,   0,   0,   0, -10],
                [-20, -10, -10, -10, -10, -10, -10, -20]
            ],
            [PieceType.ROOK]: [
                [  0,   0,   0,   5,   5,   0,   0,   0],
                [ -5,   0,   0,   0,   0,   0,   0,  -5],
                [ -5,   0,   0,   0,   0,   0,   0,  -5],
                [ -5,   0,   0,   0,   0,   0,   0,  -5],
                [ -5,   0,   0,   0,   0,   0,   0,  -5],
                [ -5,   0,   0,   0,   0,   0,   0,  -5],
                [  5,  10,  10,  10,  10,  10,  10,   5],
                [  0,   0,   0,   0,   0,   0,   0,   0]
            ],
            [PieceType.QUEEN]: [
                [-20, -10, -10,  -5,  -5, -10, -10, -20],
                [-10,   0,   5,   0,   0,   0,   0, -10],
                [-10,   5,   5,   5,   5,   5,   0, -10],
                [  0,   0,   5,   5,   5,   5,   0,  -5],
                [ -5,   0,   5,   5,   5,   5,   0,  -5],
                [-10,   0,   5,   5,   5,   5,   0, -10],
                [-10,   0,   0,   0,   0,   0,   0, -10],
                [-20, -10, -10,  -5,  -5, -10, -10, -20]
            ],
            [PieceType.KING]: [
                [ 20,  30,  10,   0,   0,  10,  30,  20],
                [ 20,  20,   0,   0,   0,   0,  20,  20],
                [-10, -20, -20, -20, -20, -20, -20, -10],
                [-20, -30, -30, -40, -40, -30, -30, -20],
                [-30, -40, -40, -50, -50, -40, -40, -30],
                [-30, -40, -40, -50, -50, -40, -40, -30],
                [-30, -40, -40, -50, -50, -40, -40, -30],
                [-30, -40, -40, -50, -50, -40, -40, -30]
            ]
        };
    }
    
    initializeOpeningBook() {
        this.openingBook = {
            'rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1': [
                'e2e4', 'd2d4', 'c2c4', 'g1f3'
            ],
            'rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1': [
                'e7e5', 'c7c5', 'e7e6', 'c7c6', 'g8f6'
            ],
            'rnbqkbnr/pppppppp/8/8/3P4/8/PPP1PPPP/RNBQKBNR b KQkq d3 0 1': [
                'd7d5', 'g8f6', 'e7e6', 'c7c5'
            ]
        };
    }
    
    initializeBoard() {
        for (let row = 0; row < 8; row++) {
            for (let col = 0; col < 8; col++) {
                this.board[row][col] = null;
            }
        }
        this.bitboard.reset();

        const pieces = [
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
            const piece = new Piece(type, color);
            this.board[row][col] = piece;
            const square = row * 8 + col;
            this.bitboard.setSquare(color, type, square);
        }
    }
    
    loadFEN(fen) {
        const parts = fen.split(' ');
        if (parts.length < 4) return false;

        for (let row = 0; row < 8; row++) {
            for (let col = 0; col < 8; col++) {
                this.board[row][col] = null;
            }
        }
        this.bitboard.reset();

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
                        const piece = new Piece(pieceType, color);
                        this.board[row][col] = piece;
                        const square = row * 8 + col;
                        this.bitboard.setSquare(color, pieceType, square);
                    }
                    col++;
                }
            }
        }

        this.currentPlayer = parts[1] === 'w' ? Color.WHITE : Color.BLACK;

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

        this.enPassantTarget = parts[3] === '-' ? null : {
            row: 8 - parseInt(parts[3][1]),
            col: parts[3].charCodeAt(0) - 'a'.charCodeAt(0)
        };

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

        fen += this.currentPlayer === Color.WHITE ? ' w ' : ' b ';

        let castling = '';
        if (this.castlingRights[Color.WHITE].kingside) castling += 'K';
        if (this.castlingRights[Color.WHITE].queenside) castling += 'Q';
        if (this.castlingRights[Color.BLACK].kingside) castling += 'k';
        if (this.castlingRights[Color.BLACK].queenside) castling += 'q';
        fen += castling || '-';
        fen += ' ';

        if (this.enPassantTarget) {
            const cols = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h'];
            fen += cols[this.enPassantTarget.col] + (8 - this.enPassantTarget.row);
        } else {
            fen += '-';
        }

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
    
    parseMove(moveStr) {
        const cols = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h'];
        
        // Gestione arrocco
        if (moveStr === 'O-O' || moveStr === '0-0') {
            const row = this.currentPlayer === Color.WHITE ? 7 : 0;
            return new Move(row, 4, row, 6, PieceType.NONE, true);
        } else if (moveStr === 'O-O-O' || moveStr === '0-0-0') {
            const row = this.currentPlayer === Color.WHITE ? 7 : 0;
            return new Move(row, 4, row, 2, PieceType.NONE, true);
        }
        
        if (moveStr.length < 4) {
            console.log(`❌ Formato mossa non valido: ${moveStr}`);
            return null;
        }
        
        const fromColChar = moveStr[0].toLowerCase();
        const fromRowChar = moveStr[1];
        const toColChar = moveStr[2].toLowerCase();
        const toRowChar = moveStr[3];
        
        const fromCol = cols.indexOf(fromColChar);
        const fromRow = 8 - parseInt(fromRowChar);
        const toCol = cols.indexOf(toColChar);
        const toRow = 8 - parseInt(toRowChar);
        
        if (fromCol === -1 || fromRow < 0 || fromRow > 7 ||
            toCol === -1 || toRow < 0 || toRow > 7) {
            console.log(`❌ Coordinate non valide: ${moveStr}`);
            return null;
        }
        
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
    
    makeMove(moveStr) {
        if (!moveStr || moveStr.length < 2) {
            console.log(`❌ Mossa non valida`);
            return false;
        }
        
        const move = this.parseMove(moveStr);
        if (!move) {
            return false;
        }
        
        const piece = this.board[move.fromRow][move.fromCol];
        if (!piece) {
            console.log(`❌ Nessun pezzo in ${String.fromCharCode(97 + move.fromCol)}${8 - move.fromRow}`);
            return false;
        }
        
        if (piece.color !== this.currentPlayer) {
            console.log(`❌ Non è il turno di ${piece.color === Color.WHITE ? 'Bianco' : 'Nero'}`);
            return false;
        }
        
        const legalMoves = this.getLegalMoves();
        let foundMove = null;
        for (const legalMove of legalMoves) {
            if (legalMove.fromRow === move.fromRow &&
                legalMove.fromCol === move.fromCol &&
                legalMove.toRow === move.toRow &&
                legalMove.toCol === move.toCol &&
                legalMove.promotion === move.promotion) {
                foundMove = legalMove;
                break;
            }
        }
        
        if (!foundMove) {
            console.log(`❌ Mossa non valida: ${moveStr}`);
            return false;
        }
        
        return this.makeMoveInternal(foundMove, true);
    }
    
    getMVVLVAScore(move) {
        if (!move.isCapture) return 0;
        
        const victimValue = move.capturedPiece?.getValue() || 0;
        const aggressorValue = this.board[move.fromRow][move.fromCol]?.getValue() || 0;
        
        return (victimValue * 10) - aggressorValue;
    }
    
    orderMoves(moves, ply, ttMove = null) {
        const scoredMoves = [];
        
        for (const move of moves) {
            let score = 0;
            
            if (ttMove && move.equals(ttMove)) {
                score += 10000;
            }
            
            if (move.isCapture) {
                score += 9000 + this.getMVVLVAScore(move);
            }
            
            if (move.promotion !== PieceType.NONE) {
                score += 8000;
                if (move.promotion === PieceType.QUEEN) score += 500;
            }
            
            if (this.killerMoves[ply] && this.killerMoves[ply][0] && 
                move.equals(this.killerMoves[ply][0])) {
                score += 7000;
            }
            if (this.killerMoves[ply] && this.killerMoves[ply][1] && 
                move.equals(this.killerMoves[ply][1])) {
                score += 6000;
            }
            
            const piece = this.board[move.fromRow][move.fromCol];
            if (piece) {
                score += this.historyHeuristic[piece.color][piece.type][move.toSquare];
            }
            
            move.score = score;
            scoredMoves.push(move);
        }
        
        scoredMoves.sort((a, b) => b.score - a.score);
        return scoredMoves;
    }
    
    getAllPossibleMoves(forColor) {
        const color = forColor !== undefined ? forColor : this.currentPlayer;
        const moves = [];
        
        for (let row = 0; row < 8; row++) {
            for (let col = 0; col < 8; col++) {
                const piece = this.board[row][col];
                if (piece && piece.color === color) {
                    this.generateMovesForPiece(row, col, piece, moves);
                }
            }
        }
        
        return moves;
    }
    
    generateMovesForPiece(row, col, piece, moves) {
        switch(piece.type) {
            case PieceType.PAWN:
                this.generatePawnMoves(row, col, piece.color, moves);
                break;
            case PieceType.KNIGHT:
                this.generateKnightMoves(row, col, piece.color, moves);
                break;
            case PieceType.BISHOP:
                this.generateBishopMoves(row, col, piece.color, moves);
                break;
            case PieceType.ROOK:
                this.generateRookMoves(row, col, piece.color, moves);
                break;
            case PieceType.QUEEN:
                this.generateQueenMoves(row, col, piece.color, moves);
                break;
            case PieceType.KING:
                this.generateKingMoves(row, col, piece.color, moves);
                break;
        }
    }
    
    generatePawnMoves(row, col, color, moves) {
        const direction = color === Color.WHITE ? -1 : 1;
        const startRow = color === Color.WHITE ? 6 : 1;
        const promotionRow = color === Color.WHITE ? 0 : 7;
        
        // Movimento in avanti
        let newRow = row + direction;
        if (newRow >= 0 && newRow < 8) {
            if (!this.board[newRow][col]) {
                if (newRow === promotionRow) {
                    [PieceType.QUEEN, PieceType.ROOK, PieceType.BISHOP, PieceType.KNIGHT].forEach(promType => {
                        const move = new Move(row, col, newRow, col, promType);
                        moves.push(move);
                    });
                } else {
                    moves.push(new Move(row, col, newRow, col));
                    
                    if (row === startRow) {
                        const newRow2 = row + 2 * direction;
                        if (newRow2 >= 0 && newRow2 < 8 && !this.board[newRow2][col]) {
                            moves.push(new Move(row, col, newRow2, col));
                        }
                    }
                }
            }
        }
        
        // Catture
        for (const dc of [-1, 1]) {
            const newRow = row + direction;
            const newCol = col + dc;
            if (newRow >= 0 && newRow < 8 && newCol >= 0 && newCol < 8) {
                const target = this.board[newRow][newCol];
                if (target && target.color !== color) {
                    if (newRow === promotionRow) {
                        [PieceType.QUEEN, PieceType.ROOK, PieceType.BISHOP, PieceType.KNIGHT].forEach(promType => {
                            const move = new Move(row, col, newRow, newCol, promType);
                            move.isCapture = true;
                            move.capturedPiece = target;
                            move.capturedPieceType = target.type;
                            moves.push(move);
                        });
                    } else {
                        const move = new Move(row, col, newRow, newCol);
                        move.isCapture = true;
                        move.capturedPiece = target;
                        move.capturedPieceType = target.type;
                        moves.push(move);
                    }
                }
                
                // En passant
                if (this.enPassantTarget && 
                    this.enPassantTarget.row === newRow && 
                    this.enPassantTarget.col === newCol) {
                    const move = new Move(row, col, newRow, newCol);
                    move.isEnPassant = true;
                    move.isCapture = true;
                    const capturedRow = row;
                    const capturedCol = newCol;
                    const capturedPiece = this.board[capturedRow][capturedCol];
                    if (capturedPiece && capturedPiece.type === PieceType.PAWN && capturedPiece.color !== color) {
                        move.capturedPiece = capturedPiece;
                        move.capturedPieceType = PieceType.PAWN;
                        moves.push(move);
                    }
                }
            }
        }
    }
    
    generateKnightMoves(row, col, color, moves) {
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
                if (!target || target.color !== color) {
                    const move = new Move(row, col, newRow, newCol);
                    if (target) {
                        move.isCapture = true;
                        move.capturedPiece = target;
                        move.capturedPieceType = target.type;
                    }
                    moves.push(move);
                }
            }
        }
    }
    
    generateBishopMoves(row, col, color, moves) {
        const directions = [[-1, -1], [-1, 1], [1, -1], [1, 1]];
        this.generateSlidingMoves(row, col, color, directions, moves);
    }
    
    generateRookMoves(row, col, color, moves) {
        const directions = [[-1, 0], [1, 0], [0, -1], [0, 1]];
        this.generateSlidingMoves(row, col, color, directions, moves);
    }
    
    generateQueenMoves(row, col, color, moves) {
        const directions = [[-1, -1], [-1, 1], [1, -1], [1, 1], [-1, 0], [1, 0], [0, -1], [0, 1]];
        this.generateSlidingMoves(row, col, color, directions, moves);
    }
    
    generateSlidingMoves(row, col, color, directions, moves) {
        for (const [dr, dc] of directions) {
            let newRow = row + dr;
            let newCol = col + dc;
            
            while (newRow >= 0 && newRow < 8 && newCol >= 0 && newCol < 8) {
                const target = this.board[newRow][newCol];
                if (!target) {
                    moves.push(new Move(row, col, newRow, newCol));
                } else {
                    if (target.color !== color) {
                        const move = new Move(row, col, newRow, newCol);
                        move.isCapture = true;
                        move.capturedPiece = target;
                        move.capturedPieceType = target.type;
                        moves.push(move);
                    }
                    break;
                }
                newRow += dr;
                newCol += dc;
            }
        }
    }
    
    generateKingMoves(row, col, color, moves) {
        for (let dr = -1; dr <= 1; dr++) {
            for (let dc = -1; dc <= 1; dc++) {
                if (dr === 0 && dc === 0) continue;
                const newRow = row + dr;
                const newCol = col + dc;
                if (newRow >= 0 && newRow < 8 && newCol >= 0 && newCol < 8) {
                    const target = this.board[newRow][newCol];
                    if (!target || target.color !== color) {
                        const move = new Move(row, col, newRow, newCol);
                        if (target) {
                            move.isCapture = true;
                            move.capturedPiece = target;
                            move.capturedPieceType = target.type;
                        }
                        moves.push(move);
                    }
                }
            }
        }
        
        // Arrocco
        if (!this.isInCheck(color)) {
            const king = this.board[row][col];
            if (king && !king.hasMoved) {
                if (this.castlingRights[color].kingside) {
                    const rook = this.board[row][7];
                    if (rook && rook.type === PieceType.ROOK && !rook.hasMoved) {
                        if (!this.board[row][5] && !this.board[row][6]) {
                            if (!this.isSquareAttacked(row, 5, 1 - color) &&
                                !this.isSquareAttacked(row, 6, 1 - color)) {
                                const move = new Move(row, col, row, 6);
                                move.isCastling = true;
                                moves.push(move);
                            }
                        }
                    }
                }
                
                if (this.castlingRights[color].queenside) {
                    const rook = this.board[row][0];
                    if (rook && rook.type === PieceType.ROOK && !rook.hasMoved) {
                        if (!this.board[row][1] && !this.board[row][2] && !this.board[row][3]) {
                            if (!this.isSquareAttacked(row, 2, 1 - color) &&
                                !this.isSquareAttacked(row, 3, 1 - color)) {
                                const move = new Move(row, col, row, 2);
                                move.isCastling = true;
                                moves.push(move);
                            }
                        }
                    }
                }
            }
        }
    }
    
    isSquareAttacked(row, col, byColor) {
        // Pedoni
        const pawnDirection = byColor === Color.WHITE ? -1 : 1;
        for (const dc of [-1, 1]) {
            const newRow = row + pawnDirection;
            const newCol = col + dc;
            if (newRow >= 0 && newRow < 8 && newCol >= 0 && newCol < 8) {
                const target = this.board[newRow][newCol];
                if (target && target.color === byColor && target.type === PieceType.PAWN) {
                    return true;
                }
            }
        }
        
        // Cavalli
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
                if (target && target.color === byColor && target.type === PieceType.KNIGHT) {
                    return true;
                }
            }
        }
        
        // Re
        for (let dr = -1; dr <= 1; dr++) {
            for (let dc = -1; dc <= 1; dc++) {
                if (dr === 0 && dc === 0) continue;
                const newRow = row + dr;
                const newCol = col + dc;
                if (newRow >= 0 && newRow < 8 && newCol >= 0 && newCol < 8) {
                    const target = this.board[newRow][newCol];
                    if (target && target.color === byColor && target.type === PieceType.KING) {
                        return true;
                    }
                }
            }
        }
        
        // Pezzi scorrevoli
        const slidingDirections = [
            [-1, -1, [PieceType.BISHOP, PieceType.QUEEN]],
            [-1, 1, [PieceType.BISHOP, PieceType.QUEEN]],
            [1, -1, [PieceType.BISHOP, PieceType.QUEEN]],
            [1, 1, [PieceType.BISHOP, PieceType.QUEEN]],
            [-1, 0, [PieceType.ROOK, PieceType.QUEEN]],
            [1, 0, [PieceType.ROOK, PieceType.QUEEN]],
            [0, -1, [PieceType.ROOK, PieceType.QUEEN]],
            [0, 1, [PieceType.ROOK, PieceType.QUEEN]]
        ];
        
        for (const [dr, dc, pieceTypes] of slidingDirections) {
            let newRow = row + dr;
            let newCol = col + dc;
            
            while (newRow >= 0 && newRow < 8 && newCol >= 0 && newCol < 8) {
                const target = this.board[newRow][newCol];
                if (target) {
                    if (target.color === byColor && pieceTypes.includes(target.type)) {
                        return true;
                    }
                    break;
                }
                newRow += dr;
                newCol += dc;
            }
        }
        
        return false;
    }
    
    isInCheck(color) {
        const kingPos = this.findKing(color);
        if (!kingPos) return false;
        return this.isSquareAttacked(kingPos.row, kingPos.col, 1 - color);
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
        const piece = this.board[move.fromRow][move.fromCol];
        if (!piece || piece.color !== this.currentPlayer) {
            return false;
        }
        
        const savedState = this.saveState();
        
        if (!this.makeMoveInternal(move, false)) {
            this.restoreState(savedState);
            return false;
        }
        
        const inCheck = this.isInCheck(this.currentPlayer);
        
        this.restoreState(savedState);
        
        return !inCheck;
    }
    
    saveState() {
        return {
            board: this.copyBoard(),
            bitboard: this.copyBitboard(),
            currentPlayer: this.currentPlayer,
            castlingRights: JSON.parse(JSON.stringify(this.castlingRights)),
            enPassantTarget: this.enPassantTarget ? {...this.enPassantTarget} : null,
            halfMoveClock: this.halfMoveClock,
            fullMoveNumber: this.fullMoveNumber,
            moveHistory: [...this.moveHistory],
            moveSANHistory: [...this.moveSANHistory]
        };
    }
    
    restoreState(state) {
        this.board = state.board;
        this.bitboard = state.bitboard;
        this.currentPlayer = state.currentPlayer;
        this.castlingRights = state.castlingRights;
        this.enPassantTarget = state.enPassantTarget;
        this.halfMoveClock = state.halfMoveClock;
        this.fullMoveNumber = state.fullMoveNumber;
        this.moveHistory = state.moveHistory;
        this.moveSANHistory = state.moveSANHistory;
    }
    
    makeMoveInternal(move, updateHistory = true) {
        const piece = this.board[move.fromRow][move.fromCol];
        if (!piece) {
            console.log(`❌ Nessun pezzo in ${move.fromRow},${move.fromCol}`);
            return false;
        }
        
        if (updateHistory && piece.color !== this.currentPlayer) {
            console.log(`❌ Non è il turno di ${piece.color === Color.WHITE ? 'Bianco' : 'Nero'}`);
            return false;
        }
        
        const oldCastlingRights = JSON.parse(JSON.stringify(this.castlingRights));
        const oldEnPassant = this.enPassantTarget ? {...this.enPassantTarget} : null;
        const oldHalfMoveClock = this.halfMoveClock;
        const oldFullMoveNumber = this.fullMoveNumber;
        
        // Gestione cattura
        const capturedPiece = this.board[move.toRow][move.toCol];
        if (capturedPiece) {
            move.isCapture = true;
            move.capturedPiece = capturedPiece.copy();
            move.capturedPieceType = capturedPiece.type;
            
            this.board[move.toRow][move.toCol] = null;
            this.bitboard.clearSquare(move.toSquare);
        }
        
        // Gestione en passant
        if (move.isEnPassant) {
            const capturedPawnRow = move.fromRow;
            const capturedPawnCol = move.toCol;
            const capturedPawn = this.board[capturedPawnRow][capturedPawnCol];
            if (capturedPawn && capturedPawn.type === PieceType.PAWN) {
                move.isCapture = true;
                move.capturedPiece = capturedPawn.copy();
                move.capturedPieceType = PieceType.PAWN;
                
                this.board[capturedPawnRow][capturedPawnCol] = null;
                const capturedSquare = capturedPawnRow * 8 + capturedPawnCol;
                this.bitboard.clearSquare(capturedSquare);
            }
        }
        
        // Muovi il pezzo
        this.board[move.fromRow][move.fromCol] = null;
        this.bitboard.clearSquare(move.fromSquare);
        
        this.board[move.toRow][move.toCol] = piece;
        this.bitboard.setSquare(piece.color, piece.type, move.toSquare);
        
        // Promozione
        if (move.promotion !== PieceType.NONE) {
            const promotedPiece = new Piece(move.promotion, piece.color);
            this.board[move.toRow][move.toCol] = promotedPiece;
            this.bitboard.clearSquare(move.toSquare);
            this.bitboard.setSquare(piece.color, move.promotion, move.toSquare);
        }
        
        piece.hasMoved = true;
        
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
                
                const rookFromSquare = row * 8 + rookFromCol;
                const rookToSquare = row * 8 + rookToCol;
                this.bitboard.clearSquare(rookFromSquare);
                this.bitboard.setSquare(rook.color, PieceType.ROOK, rookToSquare);
            }
        }
        
        // Aggiorna diritti di arrocco
        if (piece.type === PieceType.KING) {
            this.castlingRights[piece.color] = { kingside: false, queenside: false };
        } else if (piece.type === PieceType.ROOK) {
            if (move.fromRow === 7) {
                if (move.fromCol === 0) this.castlingRights[Color.WHITE].queenside = false;
                if (move.fromCol === 7) this.castlingRights[Color.WHITE].kingside = false;
            } else if (move.fromRow === 0) {
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
        
        // Aggiorna contatore 50 mosse
        if (piece.type === PieceType.PAWN || move.isCapture) {
            this.halfMoveClock = 0;
        } else {
            this.halfMoveClock++;
        }
        
        if (updateHistory) {
            // CORREZIONE: Cambia il turno PRIMA di incrementare il numero di mosse
            this.currentPlayer = 1 - this.currentPlayer;
            
            // Incrementa fullMoveNumber dopo la mossa del nero
            if (this.currentPlayer === Color.WHITE) {
                this.fullMoveNumber++;
            }
            
            const isCheck = this.isInCheck(this.currentPlayer);
            const isMate = isCheck && this.getLegalMoves().length === 0;
            
            const san = move.getSAN(piece, this.board, isCheck, isMate);
            
            this.moveHistory.push({
                move: move.toString(),
                san: san,
                piece: piece.copy(),
                captured: move.capturedPiece,
                capturedType: move.capturedPieceType,
                castlingRights: oldCastlingRights,
                enPassantTarget: oldEnPassant,
                halfMoveClock: oldHalfMoveClock,
                fullMoveNumber: oldFullMoveNumber
            });
            
            this.moveSANHistory.push(san);
            
            const fen = this.getFEN().split(' ').slice(0, 4).join(' ');
            this.positionHistory.set(fen, (this.positionHistory.get(fen) || 0) + 1);
            
            this.updateGameState();
        }
        
        return true;
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
    
    copyBitboard() {
        const copy = new Bitboard();
        copy.pawns = [this.bitboard.pawns[0], this.bitboard.pawns[1]];
        copy.knights = [this.bitboard.knights[0], this.bitboard.knights[1]];
        copy.bishops = [this.bitboard.bishops[0], this.bitboard.bishops[1]];
        copy.rooks = [this.bitboard.rooks[0], this.bitboard.rooks[1]];
        copy.queens = [this.bitboard.queens[0], this.bitboard.queens[1]];
        copy.kings = [this.bitboard.kings[0], this.bitboard.kings[1]];
        copy.occupancy = [this.bitboard.occupancy[0], this.bitboard.occupancy[1]];
        copy.allPieces = this.bitboard.allPieces;
        return copy;
    }
    
    undoMove() {
        if (this.moveHistory.length === 0) return false;
        
        const lastMove = this.moveHistory.pop();
        this.moveSANHistory.pop();
        
        if (!lastMove || !lastMove.move) {
            return false;
        }
        
        const move = this.parseMove(lastMove.move);
        if (!move) {
            console.log(`❌ Impossibile parsare mossa: ${lastMove.move}`);
            return false;
        }
        
        // Ripristina il turno (corretto)
        this.currentPlayer = lastMove.piece.color;
        
        const piece = lastMove.piece;
        if (!piece) {
            console.log(`❌ Nessun pezzo nella cronologia`);
            return false;
        }
        
        const fen = this.getFEN().split(' ').slice(0, 4).join(' ');
        const count = this.positionHistory.get(fen);
        if (count && count > 1) {
            this.positionHistory.set(fen, count - 1);
        } else {
            this.positionHistory.delete(fen);
        }
        
        // Ripristina il pezzo mosso
        this.board[move.toRow][move.toCol] = null;
        this.bitboard.clearSquare(move.toSquare);
        
        this.board[move.fromRow][move.fromCol] = piece;
        this.bitboard.setSquare(piece.color, piece.type, move.fromSquare);
        
        // Ripristina il pezzo catturato
        if (lastMove.captured) {
            if (move.isEnPassant) {
                const capturedRow = move.fromRow;
                const capturedCol = move.toCol;
                this.board[capturedRow][capturedCol] = lastMove.captured;
                const capturedSquare = capturedRow * 8 + capturedCol;
                this.bitboard.setSquare(lastMove.captured.color, lastMove.captured.type, capturedSquare);
            } else {
                this.board[move.toRow][move.toCol] = lastMove.captured;
                const capturedSquare = move.toRow * 8 + move.toCol;
                this.bitboard.setSquare(lastMove.captured.color, lastMove.captured.type, capturedSquare);
            }
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
                
                const rookFromSquare = row * 8 + rookFromCol;
                const rookToSquare = row * 8 + rookToCol;
                this.bitboard.clearSquare(rookFromSquare);
                this.bitboard.setSquare(rook.color, PieceType.ROOK, rookToSquare);
            }
        }
        
        // Ripristina diritti di arrocco
        this.castlingRights = lastMove.castlingRights;
        
        // Ripristina en passant
        this.enPassantTarget = lastMove.enPassantTarget;
        
        // Ripristina contatori
        this.halfMoveClock = lastMove.halfMoveClock;
        this.fullMoveNumber = lastMove.fullMoveNumber;
        
        this.gameState = GameResult.ONGOING;
        return true;
    }
    
    evaluate() {
        if (this.gameState !== GameResult.ONGOING) {
            switch (this.gameState) {
                case GameResult.WHITE_WINS: return 1000000;
                case GameResult.BLACK_WINS: return -1000000;
                default: return 0;
            }
        }
        
        let score = 0;
        
        // Valutazione materiale
        for (let row = 0; row < 8; row++) {
            for (let col = 0; col < 8; col++) {
                const piece = this.board[row][col];
                if (piece) {
                    const value = piece.getValue();
                    const positional = this.getPositionalValue(piece, row, col);
                    
                    if (piece.color === Color.WHITE) {
                        score += value + positional;
                    } else {
                        score -= value + positional;
                    }
                }
            }
        }
        
        // Bonus per sviluppo
        if (this.fullMoveNumber < 12) {
            score += this.evaluateDevelopment();
        }
        
        // Bonus per struttura pedonale
        score += this.evaluatePawnStructure();
        
        // Bonus per sicurezza del re
        score += this.evaluateKingSafety();
        
        // Bonus per mobilità
        score += this.evaluateMobility();
        
        // Penalità per re in scacco
        if (this.isInCheck(Color.WHITE)) score -= 50;
        if (this.isInCheck(Color.BLACK)) score += 50;
        
        return Math.round(score);
    }
    
    getPositionalValue(piece, row, col) {
        const tables = this.pieceSquareTablesMG[piece.type];
        if (!tables) return 0;
        
        const actualRow = piece.color === Color.WHITE ? 7 - row : row;
        return tables[actualRow][col] || 0;
    }
    
    evaluateDevelopment() {
        let score = 0;
        
        for (let color = 0; color < 2; color++) {
            const sign = color === Color.WHITE ? 1 : -1;
            const backRow = color === Color.WHITE ? 7 : 0;
            
            if (this.board[backRow][1] && !this.board[backRow][1].hasMoved) score -= sign * 20;
            if (this.board[backRow][2] && !this.board[backRow][2].hasMoved) score -= sign * 15;
            if (this.board[backRow][5] && !this.board[backRow][5].hasMoved) score -= sign * 15;
            if (this.board[backRow][6] && !this.board[backRow][6].hasMoved) score -= sign * 20;
        }
        
        return score;
    }
    
    evaluatePawnStructure() {
        let score = 0;
        
        for (let color = 0; color < 2; color++) {
            const sign = color === Color.WHITE ? 1 : -1;
            const pawns = [];
            
            for (let row = 0; row < 8; row++) {
                for (let col = 0; col < 8; col++) {
                    const piece = this.board[row][col];
                    if (piece && piece.type === PieceType.PAWN && piece.color === color) {
                        pawns.push({row, col});
                    }
                }
            }
            
            for (const pawn of pawns) {
                let isolated = true;
                let doubled = false;
                let passed = true;
                
                for (const otherPawn of pawns) {
                    if (pawn === otherPawn) continue;
                    
                    if (Math.abs(pawn.col - otherPawn.col) === 1) {
                        isolated = false;
                    }
                    
                    if (pawn.col === otherPawn.col) {
                        doubled = true;
                    }
                    
                    const direction = color === Color.WHITE ? -1 : 1;
                    if (otherPawn.col >= pawn.col - 1 && otherPawn.col <= pawn.col + 1) {
                        if ((color === Color.WHITE && otherPawn.row < pawn.row) ||
                            (color === Color.BLACK && otherPawn.row > pawn.row)) {
                            passed = false;
                        }
                    }
                }
                
                if (isolated) score -= sign * 15;
                if (doubled) score -= sign * 10;
                if (passed) score += sign * 20;
            }
        }
        
        return score;
    }
    
    evaluateKingSafety() {
        let score = 0;
        
        for (let color = 0; color < 2; color++) {
            const sign = color === Color.WHITE ? 1 : -1;
            const kingPos = this.findKing(color);
            if (!kingPos) continue;
            
            let pawnShield = 0;
            for (let dr = -1; dr <= 1; dr++) {
                for (let dc = -1; dc <= 1; dc++) {
                    if (dr === 0 && dc === 0) continue;
                    const newRow = kingPos.row + dr;
                    const newCol = kingPos.col + dc;
                    if (newRow >= 0 && newRow < 8 && newCol >= 0 && newCol < 8) {
                        const piece = this.board[newRow][newCol];
                        if (piece && piece.type === PieceType.PAWN && piece.color === color) {
                            pawnShield++;
                        }
                    }
                }
            }
            
            score += sign * pawnShield * 10;
            
            if (this.fullMoveNumber < 20) {
                const centerDistance = Math.max(
                    Math.abs(kingPos.col - 3.5),
                    Math.abs(kingPos.row - (color === Color.WHITE ? 3.5 : 4.5))
                );
                score -= sign * centerDistance * 5;
            }
        }
        
        return score;
    }
    
    evaluateMobility() {
        const whiteMoves = this.getLegalMoves(Color.WHITE).length;
        const blackMoves = this.getLegalMoves(Color.BLACK).length;
        return (whiteMoves - blackMoves);
    }
    
    getBestMove(depth = 4) {
        const legalMoves = this.getLegalMoves();
        if (legalMoves.length === 0) {
            console.log("⚠️  Nessuna mossa legale disponibile!");
            return null;
        }
        
        const openingMove = this.getOpeningMove();
        if (openingMove) {
            const move = this.parseMove(openingMove);
            if (move && legalMoves.some(m => m.equals(move))) {
                console.log(`📖 Mossa di apertura: ${openingMove}`);
                return move;
            }
        }
        
        this.nodesSearched = 0;
        this.startSearchTime = Date.now();
        this.bestMoveSoFar = null;
        
        let bestMove = null;
        let bestValue = this.currentPlayer === Color.WHITE ? -Infinity : Infinity;
        
        let currentDepth = 1;
        const maxDepth = Math.min(depth, 6);
        
        try {
            while (currentDepth <= maxDepth && 
                   Date.now() - this.startSearchTime < this.searchTimeLimit) {
                
                console.log(`🔍 Ricerca profondità ${currentDepth}...`);
                
                const alpha = -Infinity;
                const beta = Infinity;
                
                const result = this.alphaBeta(currentDepth, alpha, beta, 0, true);
                
                if (Date.now() - this.startSearchTime < this.searchTimeLimit * 0.9) {
                    if (this.bestMoveSoFar) {
                        bestMove = this.bestMoveSoFar;
                        bestValue = result;
                        
                        console.log(`📊 Profondità ${currentDepth}: ${bestMove.toString()} (valore: ${bestValue})`);
                        
                        if (Math.abs(bestValue) > 900000) {
                            console.log(`🎯 Trovato scaccomatto!`);
                            break;
                        }
                    }
                    currentDepth++;
                }
            }
        } catch (error) {
            console.log(`⚠️  Errore nella ricerca: ${error.message}`);
        }
        
        console.log(`🔍 Profondità: ${currentDepth-1}, Nodi: ${this.nodesSearched}, Tempo: ${Date.now() - this.startSearchTime}ms`);
        
        // Fallback
        if (!bestMove || !legalMoves.some(m => m.equals(bestMove))) {
            console.log(`⚠️  La mossa trovata non è valida, usando mossa legale casuale`);
            
            const orderedMoves = this.orderMoves(legalMoves, 0);
            if (orderedMoves.length > 0) {
                bestMove = orderedMoves[0];
                console.log(`🎲 Mossa scelta: ${bestMove.toString()}`);
            } else {
                bestMove = legalMoves[Math.floor(Math.random() * legalMoves.length)];
                console.log(`🎲 Mossa casuale: ${bestMove?.toString() || 'null'}`);
            }
        }
        
        if (!bestMove) {
            console.log(`❌ ERRORE CRITICO: Nessuna mossa valida trovata!`);
            return legalMoves[0];
        }
        
        return bestMove;
    }
    
    alphaBeta(depth, alpha, beta, ply, isRoot = false) {
        this.nodesSearched++;
        
        if (Date.now() - this.startSearchTime > this.searchTimeLimit) {
            return this.evaluate();
        }
        
        if (depth <= 0) {
            return this.quiescence(alpha, beta);
        }
        
        const legalMoves = this.getLegalMoves();
        if (legalMoves.length === 0) {
            if (this.isInCheck(this.currentPlayer)) {
                return -1000000 + ply;
            }
            return 0;
        }
        
        const orderedMoves = this.orderMoves(legalMoves, ply);
        
        let bestMove = null;
        let bestScore = -Infinity;
        
        for (const move of orderedMoves) {
            const savedState = this.saveState();
            
            if (!this.makeMoveInternal(move, false)) {
                continue;
            }
            
            const score = -this.alphaBeta(depth - 1, -beta, -alpha, ply + 1);
            
            this.restoreState(savedState);
            
            if (score > bestScore) {
                bestScore = score;
                bestMove = move;
                
                if (isRoot) {
                    this.bestMoveSoFar = move;
                }
            }
            
            if (bestScore > alpha) {
                alpha = bestScore;
            }
            
            if (alpha >= beta) {
                if (!move.isCapture) {
                    if (!this.killerMoves[ply][0] || !move.equals(this.killerMoves[ply][0])) {
                        this.killerMoves[ply][1] = this.killerMoves[ply][0];
                        this.killerMoves[ply][0] = move;
                    }
                    
                    const piece = this.board[move.fromRow][move.fromCol];
                    if (piece) {
                        this.historyHeuristic[piece.color][piece.type][move.toSquare] += depth * depth;
                    }
                }
                break;
            }
        }
        
        return bestScore;
    }
    
    quiescence(alpha, beta) {
        this.nodesSearched++;
        
        const standPat = this.evaluate();
        
        if (this.currentPlayer === Color.WHITE) {
            if (standPat >= beta) return beta;
            if (standPat > alpha) alpha = standPat;
        } else {
            if (standPat <= alpha) return alpha;
            if (standPat < beta) beta = standPat;
        }
        
        const captureMoves = this.getAllPossibleMoves().filter(move => move.isCapture);
        const orderedMoves = this.orderMoves(captureMoves, 0);
        
        for (const move of orderedMoves) {
            const savedState = this.saveState();
            
            if (!this.makeMoveInternal(move, false)) continue;
            
            const score = -this.quiescence(-beta, -alpha);
            
            this.restoreState(savedState);
            
            if (this.currentPlayer === Color.WHITE) {
                if (score >= beta) return beta;
                if (score > alpha) alpha = score;
            } else {
                if (score <= alpha) return alpha;
                if (score < beta) beta = score;
            }
        }
        
        return this.currentPlayer === Color.WHITE ? alpha : beta;
    }
    
    getZobristKey() {
        let key = 0;
        
        for (let row = 0; row < 8; row++) {
            for (let col = 0; col < 8; col++) {
                const piece = this.board[row][col];
                if (piece) {
                    const pieceIndex = piece.type * 2 + piece.color;
                    const squareIndex = row * 8 + col;
                    key ^= (pieceIndex * 1000 + squareIndex) * 2654435761;
                }
            }
        }
        
        key ^= this.currentPlayer * 123456789;
        key ^= this.castlingRights[Color.WHITE].kingside ? 1 : 0;
        key ^= this.castlingRights[Color.WHITE].queenside ? 2 : 0;
        key ^= this.castlingRights[Color.BLACK].kingside ? 4 : 0;
        key ^= this.castlingRights[Color.BLACK].queenside ? 8 : 0;
        
        if (this.enPassantTarget) {
            key ^= (this.enPassantTarget.row * 8 + this.enPassantTarget.col) * 987654321;
        }
        
        return key.toString();
    }
    
    getOpeningMove() {
        const simplifiedFEN = this.getFEN().split(' ').slice(0, 4).join(' ');
        const moves = this.openingBook[simplifiedFEN];
        if (moves && moves.length > 0) {
            return moves[Math.floor(Math.random() * moves.length)];
        }
        return null;
    }
    
    updateGameState() {
        const legalMoves = this.getLegalMoves();
        
        const whiteKing = this.findKing(Color.WHITE);
        const blackKing = this.findKing(Color.BLACK);
        
        if (!whiteKing || !blackKing) {
            this.gameState = GameResult.DRAW;
            return;
        }
        
        if (this.isInCheck(this.currentPlayer) && legalMoves.length === 0) {
            this.gameState = this.currentPlayer === Color.WHITE ? 
                GameResult.BLACK_WINS : GameResult.WHITE_WINS;
            return;
        }
        
        if (!this.isInCheck(this.currentPlayer) && legalMoves.length === 0) {
            this.gameState = GameResult.STALEMATE;
            return;
        }
        
        if (this.halfMoveClock >= 100) {
            this.gameState = GameResult.DRAW;
            return;
        }
        
        if (this.isInsufficientMaterial()) {
            this.gameState = GameResult.DRAW;
            return;
        }
        
        for (const count of this.positionHistory.values()) {
            if (count >= 3) {
                this.gameState = GameResult.DRAW;
                return;
            }
        }
    }
    
    isInsufficientMaterial() {
        let whitePieces = 0, blackPieces = 0;
        let whiteMinors = 0, blackMinors = 0;
        let whiteBishops = [], blackBishops = [];
        
        for (let row = 0; row < 8; row++) {
            for (let col = 0; col < 8; col++) {
                const piece = this.board[row][col];
                if (piece) {
                    if (piece.color === Color.WHITE) {
                        whitePieces++;
                        if (piece.type === PieceType.BISHOP || piece.type === PieceType.KNIGHT) {
                            whiteMinors++;
                            if (piece.type === PieceType.BISHOP) {
                                whiteBishops.push((row + col) % 2);
                            }
                        } else if (piece.type !== PieceType.KING) {
                            return false;
                        }
                    } else {
                        blackPieces++;
                        if (piece.type === PieceType.BISHOP || piece.type === PieceType.KNIGHT) {
                            blackMinors++;
                            if (piece.type === PieceType.BISHOP) {
                                blackBishops.push((row + col) % 2);
                            }
                        } else if (piece.type !== PieceType.KING) {
                            return false;
                        }
                    }
                }
            }
        }
        
        if (whitePieces === 1 && blackPieces === 1) return true;
        
        if (whitePieces === 2 && whiteMinors === 1 && blackPieces === 1) return true;
        if (blackPieces === 2 && blackMinors === 1 && whitePieces === 1) return true;
        
        if (whitePieces === 2 && blackPieces === 2 && 
            whiteMinors === 1 && blackMinors === 1) {
            if (whiteBishops.length === 1 && blackBishops.length === 1) {
                return whiteBishops[0] === blackBishops[0];
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
    
    printBoard() {
        console.log('\n    a   b   c   d   e   f   g   h');
        console.log('  ┌───┬───┬───┬───┬───┬───┬───┬───┐');
        
        for (let row = 0; row < 8; row++) {
            let line = `${8 - row} │`;
            
            for (let col = 0; col < 8; col++) {
                const piece = this.board[row][col];
                if (piece) {
                    line += ` ${piece.getSymbol()} │`;
                } else {
                    const isDarkSquare = (row + col) % 2 === 1;
                    line += isDarkSquare ? '░░░│' : '   │';
                }
            }
            
            line += ` ${8 - row}`;
            console.log(line);
            
            if (row < 7) {
                console.log('  ├───┼───┼───┼───┼───┼───┼───┼───┤');
            }
        }
        
        console.log('  └───┴───┴───┴───┴───┴───┴───┴───┘');
        console.log('    a   b   c   d   e   f   g   h\n');
        
        const player = this.currentPlayer === Color.WHITE ? "Bianco ♔" : "Nero ♚";
        console.log(`🎮 Turno: ${player}`);
        console.log(`📊 Mossa: ${this.fullMoveNumber}`);
        
        if (this.isInCheck(this.currentPlayer)) {
            console.log("⚔️  SCACCO!");
        }
        
        const legalMoves = this.getLegalMoves();
        console.log(`📋 Mosse legali: ${legalMoves.length}`);
        
        if (this.isGameOver()) {
            const result = this.getResult();
            console.log(`\n🏁 ${this.getGameOverReason()}: ${result}`);
        } else {
            const score = this.evaluate();
            const evalText = score > 0 ? `+${(score/100).toFixed(2)} (Bianco in vantaggio)` :
                           score < 0 ? `${(score/100).toFixed(2)} (Nero in vantaggio)` : 
                           "0.00 (Pari)";
            console.log(`💰 Valutazione: ${evalText}`);
        }
        
        if (this.moveSANHistory.length > 0) {
            console.log(`\n📜 Ultime mosse:`);
            const lastMoves = this.moveSANHistory.slice(-3);
            let moveNum = this.fullMoveNumber - lastMoves.length;
            if (this.currentPlayer === Color.BLACK) moveNum--;
            
            for (let i = 0; i < lastMoves.length; i++) {
                console.log(`  ${moveNum + Math.ceil(i/2)}. ${lastMoves[i]}`);
            }
        }
    }
    
    analyzePosition() {
        console.log('\n🔍 Analisi posizione:');
        
        const fen = this.getFEN();
        console.log(`📝 FEN: ${fen}`);
        
        const score = this.evaluate();
        console.log(`💰 Valutazione: ${score > 0 ? '+' : ''}${score / 100} (${score > 0 ? 'Bianco' : 'Nero'} in vantaggio)`);
        
        let whiteMaterial = 0, blackMaterial = 0;
        let whitePieces = [], blackPieces = [];
        
        for (let row = 0; row < 8; row++) {
            for (let col = 0; col < 8; col++) {
                const piece = this.board[row][col];
                if (piece) {
                    if (piece.color === Color.WHITE) {
                        whiteMaterial += piece.getValue();
                        whitePieces.push(piece.getSymbol());
                    } else {
                        blackMaterial += piece.getValue();
                        blackPieces.push(piece.getSymbol());
                    }
                }
            }
        }
        
        console.log(`⚪ Bianco: ${whitePieces.join(' ')} (${whiteMaterial/100})`);
        console.log(`⚫ Nero: ${blackPieces.join(' ')} (${blackMaterial/100})`);
        console.log(`⚖️  Vantaggio: ${(whiteMaterial - blackMaterial)/100 > 0 ? '+' : ''}${(whiteMaterial - blackMaterial)/100}`);
        
        const whiteKing = this.findKing(Color.WHITE);
        const blackKing = this.findKing(Color.BLACK);
        if (whiteKing) console.log(`♔ Re bianco: ${String.fromCharCode(97 + whiteKing.col)}${8 - whiteKing.row}`);
        if (blackKing) console.log(`♚ Re nero: ${String.fromCharCode(97 + blackKing.col)}${8 - blackKing.row}`);
    }
}

// ============= GIOCO COMPLETO CORRETTO =============
class StrongChessGame {
    constructor() {
        this.engine = new StrongChessEngine();
        this.rl = readline.createInterface({
            input: process.stdin,
            output: process.stdout,
            terminal: true
        });
        this.isGameRunning = true;
        this.computerColor = Color.BLACK;
        this.difficulty = 3;
        this.gameMode = 'player-vs-computer';
    }

    start() {
        console.clear();
        console.log('╔══════════════════════════════════════════════════════════════════╗');
        console.log('║           ♔ STRONG CHESS ENGINE CORRETTO ♚                       ║');
        console.log('║        Motore scacchistico avanzato - Versione Corretta          ║');
        console.log('╚══════════════════════════════════════════════════════════════════╝\n');
        
        this.showMenu();
    }

    showMenu() {
        console.log('\n📋 MENU PRINCIPALE:');
        console.log('  1. Nuova partita (Bianco vs Computer)');
        console.log('  2. Nuova partita (Computer vs Nero)');
        console.log('  3. Partita tra umani');
        console.log('  4. Imposta difficoltà (attuale: ' + this.difficulty + ')');
        console.log('  5. Analizza posizione corrente');
        console.log('  6. Carica posizione da FEN');
        console.log('  7. Visualizza cronologia mosse');
        console.log('  0. Esci');
        
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
                    this.setDifficulty();
                    break;
                case '5':
                    this.analyzePosition();
                    break;
                case '6':
                    this.loadFENPosition();
                    break;
                case '7':
                    this.showMoveHistory();
                    break;
                case '0':
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
        this.engine = new StrongChessEngine();
        console.log('\n♔ Nuova partita iniziata! ♚');
        console.log(`Modalità: ${this.gameMode}`);
        console.log(`Difficoltà: ${this.difficulty}`);
        console.log(`🤖 Il computer gioca con il ${this.computerColor === Color.WHITE ? 'Bianco' : 'Nero'}`);
        
        this.gameLoop();
    }

    gameLoop() {
        if (!this.isGameRunning) {
            console.log('\n🎉 Grazie per aver giocato!');
            this.rl.close();
            return;
        }
        
        this.engine.printBoard();
        
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
        console.log('   ? = mostra mosse, undo = annulla, menu = ritorna al menu');
        console.log('   hint = suggerimento, fen = mostra FEN');
        
        this.rl.question('> ', (input) => {
            this.processInput(input.trim());
        });
    }

    computerTurn() {
        const depth = this.difficulty + 1;
        console.log(`\n🤖 Computer pensa (profondità ${depth})...`);
        
        const startTime = Date.now();
        try {
            const bestMove = this.engine.getBestMove(depth);
            const timeTaken = Date.now() - startTime;
            
            if (bestMove) {
                const piece = this.engine.board[bestMove.fromRow][bestMove.fromCol];
                const san = bestMove.getSAN(piece, this.engine.board, false, false);
                console.log(`🤖 Computer gioca: ${san} (${bestMove.toString()}) - ${timeTaken}ms`);
                
                const success = this.engine.makeMove(bestMove.toString());
                if (success) {
                    setTimeout(() => this.gameLoop(), 500);
                } else {
                    console.log('❌ ERRORE: Mossa del computer illegale! Riprovo...');
                    
                    const legalMoves = this.engine.getLegalMoves();
                    if (legalMoves.length > 0) {
                        const scoredMoves = this.engine.orderMoves(legalMoves, 0);
                        const fallbackMove = scoredMoves[0];
                        const fallbackPiece = this.engine.board[fallbackMove.fromRow][fallbackMove.fromCol];
                        const fallbackSAN = fallbackMove.getSAN(fallbackPiece, this.engine.board, false, false);
                        console.log(`🤖 Fallback: ${fallbackSAN} (${fallbackMove.toString()})`);
                        
                        if (this.engine.makeMove(fallbackMove.toString())) {
                            setTimeout(() => this.gameLoop(), 500);
                        } else {
                            console.log('❌ ERRORE CRITICO: Fallback fallito!');
                            this.gameLoop();
                        }
                    } else {
                        console.log('❌ Nessuna mossa legale disponibile!');
                        this.gameLoop();
                    }
                }
            } else {
                console.log('🤖 Computer non ha trovato mosse, controllando stato...');
                this.gameLoop();
            }
        } catch (error) {
            console.log(`❌ Errore nel computer: ${error.message}`);
            this.gameLoop();
        }
    }

    processInput(input) {
        if (!input) {
            this.gameLoop();
            return;
        }
        
        const lowerInput = input.toLowerCase();
        
        switch(lowerInput) {
            case 'menu':
                this.showMenu();
                return;
            case 'undo':
                if (this.engine.undoMove()) {
                    console.log('↩️  Mossa annullata');
                    this.gameLoop();
                } else {
                    console.log('❌ Nessuna mossa da annullare');
                    this.gameLoop();
                }
                return;
            case '?':
                this.showAvailableMoves();
                return;
            case 'hint':
                this.showHint();
                return;
            case 'analyze':
                this.engine.analyzePosition();
                setTimeout(() => {
                    this.rl.question('\n↵ Premi Invio per continuare... ', () => {
                        this.gameLoop();
                    });
                }, 100);
                return;
            case 'fen':
                console.log(`📝 FEN attuale: ${this.engine.getFEN()}`);
                this.gameLoop();
                return;
            case 'history':
                this.showMoveHistory();
                return;
            default:
                if (this.engine.makeMove(input)) {
                    console.log(`✅ Mossa eseguita: ${input}`);
                    setTimeout(() => this.gameLoop(), 500);
                } else {
                    console.log('❌ Mossa illegale!');
                    this.suggestMoves();
                }
        }
    }

    showAvailableMoves() {
        const legalMoves = this.engine.getLegalMoves();
        
        if (legalMoves.length === 0) {
            console.log('⚠️  Non ci sono mosse legali disponibili!');
        } else {
            console.log(`📋 ${legalMoves.length} mosse legali disponibili:`);
            
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
            
            let moveCount = 0;
            for (const [piece, moves] of Object.entries(movesByPiece)) {
                if (moveCount < 10) {
                    console.log(`  ${piece}: ${moves.slice(0, 8).join(', ')}${moves.length > 8 ? '...' : ''}`);
                    moveCount += moves.length;
                }
            }
            
            if (moveCount < legalMoves.length) {
                console.log(`  ... e altre ${legalMoves.length - moveCount} mosse`);
            }
            
            console.log(`\n💡 Usa notazione algebrica: e2e4, g1f3, O-O, e7e8q`);
        }
        
        this.gameLoop();
    }

    showHint() {
        console.log('\n💡 Suggerimento del computer...');
        const hintMove = this.engine.getBestMove(2);
        if (hintMove) {
            const piece = this.engine.board[hintMove.fromRow][hintMove.fromCol];
            const san = hintMove.getSAN(piece, this.engine.board, false, false);
            console.log(`💡 Suggerisco: ${san} (${hintMove.toString()})`);
        } else {
            console.log('⚠️  Nessun suggerimento disponibile');
        }
        this.gameLoop();
    }

    suggestMoves() {
        const legalMoves = this.engine.getLegalMoves();
        
        if (legalMoves.length > 0) {
            console.log('📋 Prova una di queste mosse:');
            const scoredMoves = this.engine.orderMoves(legalMoves, 0);
            for (let i = 0; i < Math.min(5, scoredMoves.length); i++) {
                const move = scoredMoves[i];
                const piece = this.engine.board[move.fromRow][move.fromCol];
                if (piece) {
                    const san = move.getSAN(piece, this.engine.board, false, false);
                    console.log(`  ${i+1}. ${san} (${move.toString()})`);
                }
            }
        }
        
        this.gameLoop();
    }

    analyzePosition() {
        this.engine.analyzePosition();
        
        setTimeout(() => {
            this.rl.question('\n🔍 Premere Invio per continuare... ', () => {
                this.showMenu();
            });
        }, 100);
    }

    showGameResult() {
        const result = this.engine.getResult();
        const reason = this.engine.getGameOverReason();
        
        console.log('\n' + '═'.repeat(60));
        console.log(`🏁 ${reason}`);
        console.log(`📊 Risultato finale: ${result}`);
        console.log('═'.repeat(60));
        
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
                this.gameLoop();
            } else {
                console.log('❌ FEN non valido!');
                this.loadFENPosition();
            }
        });
    }

    showMoveHistory() {
        console.log('\n📜 Cronologia mosse:');
        if (this.engine.moveSANHistory.length === 0) {
            console.log('Nessuna mossa ancora giocata');
        } else {
            for (let i = 0; i < this.engine.moveSANHistory.length; i += 2) {
                const whiteMove = this.engine.moveSANHistory[i];
                const blackMove = i + 1 < this.engine.moveSANHistory.length ? this.engine.moveSANHistory[i + 1] : '';
                const moveNumber = Math.floor(i / 2) + 1;
                console.log(`${moveNumber}. ${whiteMove} ${blackMove}`);
            }
        }
        
        setTimeout(() => {
            this.rl.question('\n↵ Premi Invio per continuare... ', () => {
                this.showMenu();
            });
        }, 100);
    }

    setDifficulty() {
        console.log('\n🎯 Imposta difficoltà del computer:');
        console.log('  1. Facile (profondità 2) - ELO ~1000');
        console.log('  2. Medio (profondità 3) - ELO ~1300');
        console.log('  3. Difficile (profondità 4) - ELO ~1500');
        console.log('  4. Esperto (profondità 5) - ELO ~1700');
        console.log('  5. Maestro (profondità 6) - ELO ~1900');
        
        this.rl.question('\nScegli livello (1-5): ', (level) => {
            const newLevel = parseInt(level);
            
            if (newLevel >= 1 && newLevel <= 5) {
                this.difficulty = newLevel;
                this.engine.searchTimeLimit = 1000 * (newLevel + 2);
                console.log(`✅ Difficoltà impostata a ${newLevel}`);
                console.log(`⏱️  Tempo di ricerca: ${this.engine.searchTimeLimit/1000} secondi`);
            } else {
                console.log('❌ Livello non valido');
            }
            
            this.showMenu();
        });
    }
}

// ============= AVVIA GIOCO =============
if (require.main === module) {
    const game = new StrongChessGame();
    game.start();
}

module.exports = { StrongChessEngine, StrongChessGame, PieceType, Color };