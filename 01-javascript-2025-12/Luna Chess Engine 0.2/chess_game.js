const readline = require('readline');

// ============= COSTANTI =============
const PieceType = {
    PAWN: 0, KNIGHT: 1, BISHOP: 2, ROOK: 3, QUEEN: 4, KING: 5, NONE: 6
};

const Color = {
    WHITE: 0, BLACK: 1, NONE: 2
};

const GameState = {
    NORMAL: 0,
    CHECK: 1,
    CHECKMATE: 2,
    STALEMATE: 3,
    DRAW: 4
};

const PIECE_VALUES = {
    [PieceType.PAWN]: 100,
    [PieceType.KNIGHT]: 320,
    [PieceType.BISHOP]: 330,
    [PieceType.ROOK]: 500,
    [PieceType.QUEEN]: 900,
    [PieceType.KING]: 20000
};

// Tabelle dei pezzi per posizionamento
const PAWN_TABLE = [
    [0, 0, 0, 0, 0, 0, 0, 0],
    [50, 50, 50, 50, 50, 50, 50, 50],
    [10, 10, 20, 30, 30, 20, 10, 10],
    [5, 5, 10, 25, 25, 10, 5, 5],
    [0, 0, 0, 20, 20, 0, 0, 0],
    [5, -5, -10, 0, 0, -10, -5, 5],
    [5, 10, 10, -20, -20, 10, 10, 5],
    [0, 0, 0, 0, 0, 0, 0, 0]
];

const KNIGHT_TABLE = [
    [-50, -40, -30, -30, -30, -30, -40, -50],
    [-40, -20, 0, 0, 0, 0, -20, -40],
    [-30, 0, 10, 15, 15, 10, 0, -30],
    [-30, 5, 15, 20, 20, 15, 5, -30],
    [-30, 0, 15, 20, 20, 15, 0, -30],
    [-30, 5, 10, 15, 15, 10, 5, -30],
    [-40, -20, 0, 5, 5, 0, -20, -40],
    [-50, -40, -30, -30, -30, -30, -40, -50]
];

const BISHOP_TABLE = [
    [-20, -10, -10, -10, -10, -10, -10, -20],
    [-10, 0, 0, 0, 0, 0, 0, -10],
    [-10, 0, 5, 10, 10, 5, 0, -10],
    [-10, 5, 5, 10, 10, 5, 5, -10],
    [-10, 0, 10, 10, 10, 10, 0, -10],
    [-10, 10, 10, 10, 10, 10, 10, -10],
    [-10, 5, 0, 0, 0, 0, 5, -10],
    [-20, -10, -10, -10, -10, -10, -10, -20]
];

const ROOK_TABLE = [
    [0, 0, 0, 0, 0, 0, 0, 0],
    [5, 10, 10, 10, 10, 10, 10, 5],
    [-5, 0, 0, 0, 0, 0, 0, -5],
    [-5, 0, 0, 0, 0, 0, 0, -5],
    [-5, 0, 0, 0, 0, 0, 0, -5],
    [-5, 0, 0, 0, 0, 0, 0, -5],
    [-5, 0, 0, 0, 0, 0, 0, -5],
    [0, 0, 0, 5, 5, 0, 0, 0]
];

const QUEEN_TABLE = [
    [-20, -10, -10, -5, -5, -10, -10, -20],
    [-10, 0, 0, 0, 0, 0, 0, -10],
    [-10, 0, 5, 5, 5, 5, 0, -10],
    [-5, 0, 5, 5, 5, 5, 0, -5],
    [0, 0, 5, 5, 5, 5, 0, -5],
    [-10, 5, 5, 5, 5, 5, 0, -10],
    [-10, 0, 5, 0, 0, 0, 0, -10],
    [-20, -10, -10, -5, -5, -10, -10, -20]
];

const KING_TABLE_MIDDLE = [
    [-30, -40, -40, -50, -50, -40, -40, -30],
    [-30, -40, -40, -50, -50, -40, -40, -30],
    [-30, -40, -40, -50, -50, -40, -40, -30],
    [-30, -40, -40, -50, -50, -40, -40, -30],
    [-20, -30, -30, -40, -40, -30, -30, -20],
    [-10, -20, -20, -20, -20, -20, -20, -10],
    [20, 20, 0, 0, 0, 0, 20, 20],
    [20, 30, 10, 0, 0, 10, 30, 20]
];

const KING_TABLE_END = [
    [-50, -40, -30, -20, -20, -30, -40, -50],
    [-30, -20, -10, 0, 0, -10, -20, -30],
    [-30, -10, 20, 30, 30, 20, -10, -30],
    [-30, -10, 30, 40, 40, 30, -10, -30],
    [-30, -10, 30, 40, 40, 30, -10, -30],
    [-30, -10, 20, 30, 30, 20, -10, -30],
    [-30, -30, 0, 0, 0, 0, -30, -30],
    [-50, -30, -30, -30, -30, -30, -30, -50]
];

// ============= CLASSI =============
class Move {
    constructor(fromRow, fromCol, toRow, toCol, promotion = PieceType.NONE, special = null) {
        this.fromRow = fromRow;
        this.fromCol = fromCol;
        this.toRow = toRow;
        this.toCol = toCol;
        this.promotion = promotion;
        this.special = special; // 'castle', 'enpassant', 'pawn2'
        this.score = 0;
    }

    toString() {
        const cols = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h'];
        const from = cols[this.fromCol] + (8 - this.fromRow);
        const to = cols[this.toCol] + (8 - this.toRow);
        
        if (this.promotion !== PieceType.NONE) {
            const promoChar = { [PieceType.QUEEN]: 'q', [PieceType.ROOK]: 'r', 
                              [PieceType.BISHOP]: 'b', [PieceType.KNIGHT]: 'n' };
            return from + to + (promoChar[this.promotion] || '');
        }
        return from + to;
    }

    static fromString(moveStr, board) {
        if (!moveStr || moveStr.length < 4) return null;
        
        const cols = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h'];
        const fromCol = cols.indexOf(moveStr[0]);
        const fromRow = 8 - parseInt(moveStr[1]);
        const toCol = cols.indexOf(moveStr[2]);
        const toRow = 8 - parseInt(moveStr[3]);
        
        if (fromCol < 0 || fromRow < 0 || toCol < 0 || toRow < 0 || 
            fromRow > 7 || toRow > 7) return null;
        
        let promotion = PieceType.NONE;
        if (moveStr.length > 4) {
            const promoMap = { 'q': PieceType.QUEEN, 'r': PieceType.ROOK, 
                              'b': PieceType.BISHOP, 'n': PieceType.KNIGHT };
            promotion = promoMap[moveStr[4].toLowerCase()] || PieceType.NONE;
        }
        
        return new Move(fromRow, fromCol, toRow, toCol, promotion);
    }
}

class Piece {
    constructor(type, color) {
        this.type = type;
        this.color = color;
        this.hasMoved = false;
    }

    getValue() { return PIECE_VALUES[this.type] || 0; }
    
    getPositionValue(row, col, gamePhase = 'middle') {
        if (row < 0 || row > 7 || col < 0 || col > 7) return 0;
        
        let table;
        const r = this.color === Color.WHITE ? row : 7 - row;
        const c = this.color === Color.WHITE ? col : 7 - col;
        
        switch(this.type) {
            case PieceType.PAWN: table = PAWN_TABLE; break;
            case PieceType.KNIGHT: table = KNIGHT_TABLE; break;
            case PieceType.BISHOP: table = BISHOP_TABLE; break;
            case PieceType.ROOK: table = ROOK_TABLE; break;
            case PieceType.QUEEN: table = QUEEN_TABLE; break;
            case PieceType.KING: 
                table = gamePhase === 'end' ? KING_TABLE_END : KING_TABLE_MIDDLE;
                break;
            default: return 0;
        }
        
        return table[r]?.[c] || 0;
    }

    copy() {
        const newPiece = new Piece(this.type, this.color);
        newPiece.hasMoved = this.hasMoved;
        return newPiece;
    }

    getSymbol() {
        const symbols = {
            [PieceType.PAWN]: { [Color.WHITE]: '♙', [Color.BLACK]: '♟' },
            [PieceType.KNIGHT]: { [Color.WHITE]: '♘', [Color.BLACK]: '♞' },
            [PieceType.BISHOP]: { [Color.WHITE]: '♗', [Color.BLACK]: '♝' },
            [PieceType.ROOK]: { [Color.WHITE]: '♖', [Color.BLACK]: '♜' },
            [PieceType.QUEEN]: { [Color.WHITE]: '♕', [Color.BLACK]: '♛' },
            [PieceType.KING]: { [Color.WHITE]: '♔', [Color.BLACK]: '♚' }
        };
        return symbols[this.type]?.[this.color] || '?';
    }
}

class TranspositionTable {
    constructor() {
        this.table = new Map();
        this.size = 0;
        this.MAX_SIZE = 1000000;
    }

    init() {
        this.zobrist = [];
        for (let i = 0; i < 768; i++) {
            this.zobrist.push(Math.floor(Math.random() * 2**32));
        }
    }

    hash(board) {
        let hash = 0;
        for (let row = 0; row < 8; row++) {
            for (let col = 0; col < 8; col++) {
                const piece = board[row][col];
                if (piece) {
                    const index = row * 64 + col * 8 + piece.type * 2 + piece.color;
                    hash ^= this.zobrist[index % this.zobrist.length];
                }
            }
        }
        return hash;
    }

    store(key, depth, score, flag, move) {
        if (this.size >= this.MAX_SIZE) {
            this.table.clear();
            this.size = 0;
        }
        this.table.set(key, { depth, score, flag, move });
        this.size++;
    }

    lookup(key, depth, alpha, beta) {
        const entry = this.table.get(key);
        if (!entry) return null;
        
        if (entry.depth >= depth) {
            if (entry.flag === 'EXACT') return { score: entry.score, move: entry.move };
            if (entry.flag === 'LOWER' && entry.score >= beta) return { score: beta, move: entry.move };
            if (entry.flag === 'UPPER' && entry.score <= alpha) return { score: alpha, move: entry.move };
        }
        return null;
    }
}

class ChessEngine {
    constructor() {
        this.board = this.createBoard();
        this.currentPlayer = Color.WHITE;
        this.moveHistory = [];
        this.enPassantSquare = null;
        this.castlingRights = { 
            whiteKingside: true, 
            whiteQueenside: true, 
            blackKingside: true, 
            blackQueenside: true 
        };
        this.halfMoveClock = 0;
        this.fullMoveNumber = 1;
        this.gameState = GameState.NORMAL;
        this.result = null;
        this.difficulty = 2;
        this.transpositionTable = new TranspositionTable();
        this.transpositionTable.init();
        this.killerMoves = Array(100).fill().map(() => [null, null]);
        this.historyHeuristic = Array(2).fill().map(() => 
            Array(8).fill().map(() => Array(8).fill().map(() => Array(8).fill().map(() => Array(8).fill(0))))
        );
    }

    createBoard() {
        const board = Array(8).fill().map(() => Array(8).fill(null));
        
        // Pedoni
        for (let i = 0; i < 8; i++) {
            board[1][i] = new Piece(PieceType.PAWN, Color.BLACK);
            board[6][i] = new Piece(PieceType.PAWN, Color.WHITE);
        }
        
        // Pezzi
        const pieces = [
            PieceType.ROOK, PieceType.KNIGHT, PieceType.BISHOP,
            PieceType.QUEEN, PieceType.KING,
            PieceType.BISHOP, PieceType.KNIGHT, PieceType.ROOK
        ];
        
        for (let i = 0; i < 8; i++) {
            board[0][i] = new Piece(pieces[i], Color.BLACK);
            board[7][i] = new Piece(pieces[i], Color.WHITE);
        }
        
        return board;
    }

    getPiece(row, col) {
        return (row >= 0 && row < 8 && col >= 0 && col < 8) ? this.board[row][col] : null;
    }

    setPiece(row, col, piece) {
        if (row >= 0 && row < 8 && col >= 0 && col < 8) {
            this.board[row][col] = piece;
        }
    }

    isSquareAttacked(row, col, byColor) {
        // Controlla attacchi da pedoni
        const pawnDir = byColor === Color.WHITE ? -1 : 1;
        for (const dc of [-1, 1]) {
            const r = row - pawnDir;
            const c = col + dc;
            const piece = this.getPiece(r, c);
            if (piece && piece.type === PieceType.PAWN && piece.color === byColor) {
                return true;
            }
        }

        // Controlla attacchi da cavalli
        const knightMoves = [[2,1],[2,-1],[-2,1],[-2,-1],[1,2],[1,-2],[-1,2],[-1,-2]];
        for (const [dr, dc] of knightMoves) {
            const r = row + dr;
            const c = col + dc;
            const piece = this.getPiece(r, c);
            if (piece && piece.type === PieceType.KNIGHT && piece.color === byColor) {
                return true;
            }
        }

        // Controlla attacchi da direzioni (alfiere, torre, regina, re)
        const directions = [[1,0],[-1,0],[0,1],[0,-1],[1,1],[1,-1],[-1,1],[-1,-1]];
        for (const [dr, dc] of directions) {
            let r = row + dr;
            let c = col + dc;
            let step = 1;
            while (r >= 0 && r < 8 && c >= 0 && c < 8) {
                const piece = this.getPiece(r, c);
                if (piece) {
                    if (piece.color === byColor) {
                        if (step === 1) { // Re (solo un passo)
                            if (piece.type === PieceType.KING) {
                                return true;
                            }
                        }
                        
                        if (dr === 0 || dc === 0) { // Direzioni rettilinee
                            if (piece.type === PieceType.ROOK || piece.type === PieceType.QUEEN) {
                                return true;
                            }
                        } else { // Direzioni diagonali
                            if (piece.type === PieceType.BISHOP || piece.type === PieceType.QUEEN) {
                                return true;
                            }
                        }
                    }
                    break;
                }
                r += dr;
                c += dc;
                step++;
            }
        }

        return false;
    }

    isInCheck(color) {
        // Trova il re
        let kingRow, kingCol;
        for (let row = 0; row < 8; row++) {
            for (let col = 0; col < 8; col++) {
                const piece = this.getPiece(row, col);
                if (piece && piece.type === PieceType.KING && piece.color === color) {
                    kingRow = row;
                    kingCol = col;
                    break;
                }
            }
            if (kingRow !== undefined) break;
        }
        
        if (kingRow === undefined) return false;
        
        return this.isSquareAttacked(kingRow, kingCol, color === Color.WHITE ? Color.BLACK : Color.WHITE);
    }

    // ============= GENERAZIONE MOSSE =============
    getAllMoves(color, onlyCaptures = false) {
        const moves = [];
        for (let row = 0; row < 8; row++) {
            for (let col = 0; col < 8; col++) {
                const piece = this.getPiece(row, col);
                if (piece && piece.color === color) {
                    this.addMoves(row, col, piece, moves, onlyCaptures);
                }
            }
        }
        return this.filterLegalMoves(moves, color);
    }

    addMoves(row, col, piece, moves, onlyCaptures = false) {
        switch (piece.type) {
            case PieceType.PAWN: 
                this.addPawnMoves(row, col, piece, moves, onlyCaptures); 
                break;
            case PieceType.KNIGHT: 
                this.addKnightMoves(row, col, piece, moves, onlyCaptures); 
                break;
            case PieceType.BISHOP: 
                this.addBishopMoves(row, col, piece, moves, onlyCaptures); 
                break;
            case PieceType.ROOK: 
                this.addRookMoves(row, col, piece, moves, onlyCaptures); 
                break;
            case PieceType.QUEEN: 
                this.addQueenMoves(row, col, piece, moves, onlyCaptures); 
                break;
            case PieceType.KING: 
                this.addKingMoves(row, col, piece, moves, onlyCaptures); 
                break;
        }
    }

    addPawnMoves(row, col, piece, moves, onlyCaptures) {
        const dir = piece.color === Color.WHITE ? -1 : 1;
        const startRow = piece.color === Color.WHITE ? 6 : 1;
        const promoRow = piece.color === Color.WHITE ? 0 : 7;
        
        if (!onlyCaptures) {
            // Avanti
            const newRow = row + dir;
            if (newRow >= 0 && newRow < 8 && !this.getPiece(newRow, col)) {
                if (newRow === promoRow) {
                    [PieceType.QUEEN, PieceType.ROOK, PieceType.BISHOP, PieceType.KNIGHT]
                        .forEach(p => moves.push(new Move(row, col, newRow, col, p)));
                } else {
                    moves.push(new Move(row, col, newRow, col));
                    
                    // Doppio passo iniziale
                    if (row === startRow) {
                        const doubleRow = newRow + dir;
                        if (!this.getPiece(doubleRow, col)) {
                            moves.push(new Move(row, col, doubleRow, col, PieceType.NONE, 'pawn2'));
                        }
                    }
                }
            }
        }
        
        // Catture
        for (const dc of [-1, 1]) {
            const newRow = row + dir;
            const newCol = col + dc;
            if (newRow >= 0 && newRow < 8 && newCol >= 0 && newCol < 8) {
                const target = this.getPiece(newRow, newCol);
                if (target && target.color !== piece.color) {
                    if (newRow === promoRow) {
                        [PieceType.QUEEN, PieceType.ROOK, PieceType.BISHOP, PieceType.KNIGHT]
                            .forEach(p => moves.push(new Move(row, col, newRow, newCol, p)));
                    } else {
                        moves.push(new Move(row, col, newRow, newCol));
                    }
                }
                // En passant
                else if (this.enPassantSquare && 
                         this.enPassantSquare[0] === newRow && 
                         this.enPassantSquare[1] === newCol) {
                    moves.push(new Move(row, col, newRow, newCol, PieceType.NONE, 'enpassant'));
                }
            }
        }
    }

    addKnightMoves(row, col, piece, moves, onlyCaptures) {
        const movesList = [[2,1],[2,-1],[-2,1],[-2,-1],[1,2],[1,-2],[-1,2],[-1,-2]];
        for (const [dr, dc] of movesList) {
            const newRow = row + dr, newCol = col + dc;
            if (newRow >= 0 && newRow < 8 && newCol >= 0 && newCol < 8) {
                const target = this.getPiece(newRow, newCol);
                if (!target) {
                    if (!onlyCaptures) moves.push(new Move(row, col, newRow, newCol));
                } else if (target.color !== piece.color) {
                    moves.push(new Move(row, col, newRow, newCol));
                }
            }
        }
    }

    addSliderMoves(row, col, piece, directions, moves, onlyCaptures) {
        for (const [dr, dc] of directions) {
            let r = row + dr, c = col + dc;
            while (r >= 0 && r < 8 && c >= 0 && c < 8) {
                const target = this.getPiece(r, c);
                if (!target) {
                    if (!onlyCaptures) moves.push(new Move(row, col, r, c));
                } else {
                    if (target.color !== piece.color) {
                        moves.push(new Move(row, col, r, c));
                    }
                    break;
                }
                r += dr; c += dc;
            }
        }
    }

    addBishopMoves(row, col, piece, moves, onlyCaptures) {
        this.addSliderMoves(row, col, piece, [[1,1],[1,-1],[-1,1],[-1,-1]], moves, onlyCaptures);
    }

    addRookMoves(row, col, piece, moves, onlyCaptures) {
        this.addSliderMoves(row, col, piece, [[1,0],[-1,0],[0,1],[0,-1]], moves, onlyCaptures);
    }

    addQueenMoves(row, col, piece, moves, onlyCaptures) {
        this.addSliderMoves(row, col, piece, 
            [[1,0],[-1,0],[0,1],[0,-1],[1,1],[1,-1],[-1,1],[-1,-1]], 
            moves, onlyCaptures);
    }

    addKingMoves(row, col, piece, moves, onlyCaptures) {
        for (let dr = -1; dr <= 1; dr++) {
            for (let dc = -1; dc <= 1; dc++) {
                if (dr === 0 && dc === 0) continue;
                const newRow = row + dr, newCol = col + dc;
                if (newRow >= 0 && newRow < 8 && newCol >= 0 && newCol < 8) {
                    const target = this.getPiece(newRow, newCol);
                    if (!target) {
                        if (!onlyCaptures) moves.push(new Move(row, col, newRow, newCol));
                    } else if (target.color !== piece.color) {
                        moves.push(new Move(row, col, newRow, newCol));
                    }
                }
            }
        }
        
        // Arrocco (solo se non stiamo cercando solo catture)
        if (!onlyCaptures && !piece.hasMoved && !this.isInCheck(piece.color)) {
            const rowPos = piece.color === Color.WHITE ? 7 : 0;
            
            // Arrocco corto (kingside)
            if ((piece.color === Color.WHITE && this.castlingRights.whiteKingside) ||
                (piece.color === Color.BLACK && this.castlingRights.blackKingside)) {
                const rook = this.getPiece(rowPos, 7);
                if (rook && rook.type === PieceType.ROOK && !rook.hasMoved) {
                    if (!this.getPiece(rowPos, 5) && !this.getPiece(rowPos, 6) &&
                        !this.isSquareAttacked(rowPos, 4, piece.color === Color.WHITE ? Color.BLACK : Color.WHITE) &&
                        !this.isSquareAttacked(rowPos, 5, piece.color === Color.WHITE ? Color.BLACK : Color.WHITE) &&
                        !this.isSquareAttacked(rowPos, 6, piece.color === Color.WHITE ? Color.BLACK : Color.WHITE)) {
                        moves.push(new Move(rowPos, 4, rowPos, 6, PieceType.NONE, 'castle'));
                    }
                }
            }
            
            // Arrocco lungo (queenside)
            if ((piece.color === Color.WHITE && this.castlingRights.whiteQueenside) ||
                (piece.color === Color.BLACK && this.castlingRights.blackQueenside)) {
                const rook = this.getPiece(rowPos, 0);
                if (rook && rook.type === PieceType.ROOK && !rook.hasMoved) {
                    if (!this.getPiece(rowPos, 1) && !this.getPiece(rowPos, 2) && !this.getPiece(rowPos, 3) &&
                        !this.isSquareAttacked(rowPos, 2, piece.color === Color.WHITE ? Color.BLACK : Color.WHITE) &&
                        !this.isSquareAttacked(rowPos, 3, piece.color === Color.WHITE ? Color.BLACK : Color.WHITE) &&
                        !this.isSquareAttacked(rowPos, 4, piece.color === Color.WHITE ? Color.BLACK : Color.WHITE)) {
                        moves.push(new Move(rowPos, 4, rowPos, 2, PieceType.NONE, 'castle'));
                    }
                }
            }
        }
    }

    filterLegalMoves(moves, color) {
        const legalMoves = [];
        
        for (const move of moves) {
            // Esegui mossa temporanea
            const piece = this.getPiece(move.fromRow, move.fromCol);
            const captured = this.getPiece(move.toRow, move.toCol);
            const oldEnPassant = this.enPassantSquare;
            const oldCastling = {...this.castlingRights};
            const oldHalfMove = this.halfMoveClock;
            
            this.makeMoveInternal(move, false);
            
            // Controlla se il re è sotto scacco dopo la mossa
            if (!this.isInCheck(color)) {
                legalMoves.push(move);
            }
            
            // Ripristina
            this.undoMoveInternal(move, piece, captured, oldEnPassant, oldCastling, oldHalfMove, false);
        }
        
        return legalMoves;
    }

    // ============= ESECUZIONE MOSSE =============
    makeMoveInternal(move, updateHistory = true) {
        const piece = this.getPiece(move.fromRow, move.fromCol);
        if (!piece) return null;
        
        let captured = this.getPiece(move.toRow, move.toCol);
        
        if (updateHistory) {
            // Salva stato per undo
            this.moveHistory.push({
                move: new Move(move.fromRow, move.fromCol, move.toRow, move.toCol, move.promotion, move.special),
                piece: piece.copy(),
                captured: captured ? captured.copy() : null,
                enPassantSquare: this.enPassantSquare ? [...this.enPassantSquare] : null,
                castlingRights: {...this.castlingRights},
                halfMoveClock: this.halfMoveClock
            });
        }
        
        // Gestione catture en passant
        if (move.special === 'enpassant') {
            captured = this.getPiece(move.fromRow, move.toCol);
            this.setPiece(move.fromRow, move.toCol, null);
        }
        
        // Gestione arrocco
        if (move.special === 'castle') {
            const row = piece.color === Color.WHITE ? 7 : 0;
            if (move.toCol === 6) { // Arrocco corto
                const rook = this.getPiece(row, 7);
                this.setPiece(row, 5, rook);
                this.setPiece(row, 7, null);
                if (rook) rook.hasMoved = true;
            } else if (move.toCol === 2) { // Arrocco lungo
                const rook = this.getPiece(row, 0);
                this.setPiece(row, 3, rook);
                this.setPiece(row, 0, null);
                if (rook) rook.hasMoved = true;
            }
        }
        
        // Aggiorna diritti di arrocco
        if (piece.type === PieceType.KING) {
            if (piece.color === Color.WHITE) {
                this.castlingRights.whiteKingside = false;
                this.castlingRights.whiteQueenside = false;
            } else {
                this.castlingRights.blackKingside = false;
                this.castlingRights.blackQueenside = false;
            }
        } else if (piece.type === PieceType.ROOK) {
            if (move.fromCol === 0) { // Torre di regina
                if (piece.color === Color.WHITE) this.castlingRights.whiteQueenside = false;
                else this.castlingRights.blackQueenside = false;
            } else if (move.fromCol === 7) { // Torre di re
                if (piece.color === Color.WHITE) this.castlingRights.whiteKingside = false;
                else this.castlingRights.blackKingside = false;
            }
        }
        
        // Aggiorna en passant
        this.enPassantSquare = null;
        if (move.special === 'pawn2') {
            this.enPassantSquare = [move.fromRow + (piece.color === Color.WHITE ? -1 : 1), move.fromCol];
        }
        
        // Aggiorna contatori
        if (piece.type === PieceType.PAWN || captured) {
            this.halfMoveClock = 0;
        } else {
            this.halfMoveClock++;
        }
        
        if (piece.color === Color.BLACK) {
            this.fullMoveNumber++;
        }
        
        // Esegui mossa
        this.setPiece(move.toRow, move.toCol, piece);
        this.setPiece(move.fromRow, move.fromCol, null);
        piece.hasMoved = true;
        
        if (move.promotion !== PieceType.NONE) {
            this.setPiece(move.toRow, move.toCol, new Piece(move.promotion, piece.color));
        }
        
        // Cambia turno
        this.currentPlayer = this.currentPlayer === Color.WHITE ? Color.BLACK : Color.WHITE;
        
        return captured;
    }

    undoMoveInternal(move, piece, captured, oldEnPassant, oldCastling, oldHalfMove, updateHistory = true) {
        // Ripristina turno
        this.currentPlayer = this.currentPlayer === Color.WHITE ? Color.BLACK : Color.WHITE;
        
        // Ripristina pezzo
        this.setPiece(move.fromRow, move.fromCol, piece);
        
        // Gestione en passant
        if (move.special === 'enpassant') {
            this.setPiece(move.fromRow, move.toCol, captured);
            this.setPiece(move.toRow, move.toCol, null);
        } 
        // Gestione arrocco
        else if (move.special === 'castle') {
            const row = piece.color === Color.WHITE ? 7 : 0;
            if (move.toCol === 6) { // Arrocco corto
                const rook = this.getPiece(row, 5);
                this.setPiece(row, 7, rook);
                this.setPiece(row, 5, null);
                if (rook) rook.hasMoved = false;
            } else if (move.toCol === 2) { // Arrocco lungo
                const rook = this.getPiece(row, 3);
                this.setPiece(row, 0, rook);
                this.setPiece(row, 3, null);
                if (rook) rook.hasMoved = false;
            }
            this.setPiece(move.toRow, move.toCol, null);
        }
        else {
            this.setPiece(move.toRow, move.toCol, captured);
        }
        
        // Ripristina stato
        this.enPassantSquare = oldEnPassant;
        this.castlingRights = oldCastling;
        this.halfMoveClock = oldHalfMove;
        
        if (updateHistory && this.moveHistory.length > 0) {
            if (piece.color === Color.BLACK) {
                this.fullMoveNumber--;
            }
        }
        
        piece.hasMoved = move.fromRow !== (piece.color === Color.WHITE ? 7 : 0) || 
                         (piece.type === PieceType.KING && move.fromCol !== 4) ||
                         (piece.type === PieceType.ROOK && (move.fromCol !== 0 && move.fromCol !== 7));
    }

    makeMove(moveStr) {
        const move = Move.fromString(moveStr, this.board);
        if (!move) return false;
        
        const moves = this.getAllMoves(this.currentPlayer);
        const legalMove = moves.find(m => 
            m.fromRow === move.fromRow && m.fromCol === move.fromCol &&
            m.toRow === move.toRow && m.toCol === move.toCol &&
            m.promotion === move.promotion
        );
        
        if (!legalMove) return false;
        
        this.makeMoveInternal(legalMove, true);
        this.updateGameState();
        
        return true;
    }

    undoMove() {
        if (this.moveHistory.length === 0) return false;
        
        const lastMove = this.moveHistory.pop();
        const move = lastMove.move;
        const piece = lastMove.piece;
        const captured = lastMove.captured;
        
        this.undoMoveInternal(move, piece, captured, lastMove.enPassantSquare, 
                              lastMove.castlingRights, lastMove.halfMoveClock, false);
        this.updateGameState();
        
        return true;
    }

    // ============= VALUTAZIONE =============
    evaluate() {
        let material = 0;
        let positional = 0;
        let mobility = 0;
        let kingSafety = 0;
        let pawnStructure = 0;
        
        let whitePieceCount = 0;
        let blackPieceCount = 0;
        
        // Conta materiale e posizioni
        for (let row = 0; row < 8; row++) {
            for (let col = 0; col < 8; col++) {
                const piece = this.getPiece(row, col);
                if (piece) {
                    const value = piece.getValue();
                    if (piece.color === Color.WHITE) {
                        material += value;
                        whitePieceCount++;
                        const gamePhase = (whitePieceCount + blackPieceCount) <= 10 ? 'end' : 'middle';
                        positional += piece.getPositionValue(row, col, gamePhase);
                    } else {
                        material -= value;
                        blackPieceCount++;
                        const gamePhase = (whitePieceCount + blackPieceCount) <= 10 ? 'end' : 'middle';
                        positional -= piece.getPositionValue(row, col, gamePhase);
                    }
                }
            }
        }
        
        // Mobilità
        const whiteMoves = this.getAllMoves(Color.WHITE, false).length;
        const blackMoves = this.getAllMoves(Color.BLACK, false).length;
        mobility = (whiteMoves - blackMoves) * 5;
        
        // Struttura pedoni
        for (let col = 0; col < 8; col++) {
            let whitePawnsInFile = 0;
            let blackPawnsInFile = 0;
            
            for (let row = 0; row < 8; row++) {
                const piece = this.getPiece(row, col);
                if (piece && piece.type === PieceType.PAWN) {
                    if (piece.color === Color.WHITE) whitePawnsInFile++;
                    else blackPawnsInFile++;
                }
            }
            
            // Penalizza pedoni doppiati
            if (whitePawnsInFile > 1) pawnStructure -= 20 * (whitePawnsInFile - 1);
            if (blackPawnsInFile > 1) pawnStructure += 20 * (blackPawnsInFile - 1);
        }
        
        // Sicurezza del re
        if (this.isInCheck(Color.WHITE)) kingSafety -= 50;
        if (this.isInCheck(Color.BLACK)) kingSafety += 50;
        
        const totalScore = material + positional + mobility + pawnStructure + kingSafety;
        return this.currentPlayer === Color.WHITE ? totalScore : -totalScore;
    }

    quiescenceSearch(alpha, beta, depth) {
        const standPat = this.evaluate();
        if (standPat >= beta) return beta;
        if (alpha < standPat) alpha = standPat;
        
        const captures = this.getAllMoves(this.currentPlayer, true);
        this.orderMoves(captures, depth);
        
        for (const move of captures) {
            const piece = this.getPiece(move.fromRow, move.fromCol);
            if (!piece) continue;
            
            const captured = this.getPiece(move.toRow, move.toCol);
            if (!captured) continue;
            
            if (captured.getValue() < piece.getValue() / 2) {
                continue; // LVA pruning
            }
            
            const oldEnPassant = this.enPassantSquare;
            const oldCastling = {...this.castlingRights};
            const oldHalfMove = this.halfMoveClock;
            
            this.makeMoveInternal(move, false);
            const score = -this.quiescenceSearch(-beta, -alpha, depth + 1);
            this.undoMoveInternal(move, piece, captured, oldEnPassant, oldCastling, oldHalfMove, false);
            
            if (score >= beta) return beta;
            if (score > alpha) alpha = score;
        }
        
        return alpha;
    }

    // ============= ORDINAMENTO MOSSE =============
    orderMoves(moves, depth) {
        for (const move of moves) {
            let score = 0;
            const piece = this.getPiece(move.fromRow, move.fromCol);
            const target = this.getPiece(move.toRow, move.toCol);
            
            if (!piece) {
                move.score = 0;
                continue;
            }
            
            // MVV-LVA
            if (target) {
                score = 10000 + target.getValue() * 10 - piece.getValue();
            }
            
            // Killer moves
            if (depth < this.killerMoves.length) {
                const [killer1, killer2] = this.killerMoves[depth];
                if (killer1 && this.movesEqual(move, killer1)) score += 9000;
                if (killer2 && this.movesEqual(move, killer2)) score += 8000;
            }
            
            // History heuristic
            score += this.historyHeuristic[this.currentPlayer]?.[move.fromRow]?.[move.fromCol]?.[move.toRow]?.[move.toCol] || 0;
            
            // Promozioni
            if (move.promotion === PieceType.QUEEN) score += 8000;
            
            move.score = score;
        }
        
        moves.sort((a, b) => b.score - a.score);
    }

    movesEqual(move1, move2) {
        if (!move1 || !move2) return false;
        return move1.fromRow === move2.fromRow && move1.fromCol === move2.fromCol &&
               move1.toRow === move2.toRow && move1.toCol === move2.toCol &&
               move1.promotion === move2.promotion;
    }

    // ============= ALPHA-BETA + ITERATIVE DEEPENING =============
    alphaBeta(depth, alpha, beta, maximizingPlayer, startDepth = 0) {
        if (depth === 0) {
            return this.quiescenceSearch(alpha, beta, startDepth);
        }
        
        const moves = this.getAllMoves(this.currentPlayer);
        if (moves.length === 0) {
            if (this.isInCheck(this.currentPlayer)) {
                return maximizingPlayer ? -20000 + startDepth : 20000 - startDepth;
            }
            return 0; // Stallo
        }
        
        this.orderMoves(moves, startDepth - depth);
        
        let bestMove = moves[0];
        let bestScore = -Infinity;
        
        for (const move of moves) {
            const piece = this.getPiece(move.fromRow, move.fromCol);
            if (!piece) continue;
            
            const captured = this.makeMoveInternal(move, false);
            
            const score = -this.alphaBeta(depth - 1, -beta, -alpha, !maximizingPlayer, startDepth);
            
            const oldEnPassant = this.enPassantSquare;
            const oldCastling = {...this.castlingRights};
            const oldHalfMove = this.halfMoveClock;
            this.undoMoveInternal(move, piece, captured, oldEnPassant, oldCastling, oldHalfMove, false);
            
            if (score >= beta) {
                // Killer move
                if (!captured) {
                    if (depth < this.killerMoves.length) {
                        const [killer1, killer2] = this.killerMoves[depth];
                        if (!killer1 || !this.movesEqual(move, killer1)) {
                            this.killerMoves[depth][1] = killer1;
                            this.killerMoves[depth][0] = move;
                        }
                    }
                }
                
                // History heuristic update
                if (this.historyHeuristic[this.currentPlayer]) {
                    this.historyHeuristic[this.currentPlayer][move.fromRow][move.fromCol][move.toRow][move.toCol] += depth * depth;
                }
                
                return beta;
            }
            
            if (score > bestScore) {
                bestScore = score;
                bestMove = move;
                if (score > alpha) {
                    alpha = score;
                }
            }
        }
        
        return alpha;
    }

    findBestMove(timeLimit) {
        const startTime = Date.now();
        const moves = this.getAllMoves(this.currentPlayer);
        if (moves.length === 0) return null;
        
        let bestMove = moves[0];
        let bestScore = -Infinity;
        let depth = 1;
        const maxDepth = this.difficulty * 2 + 2; // 4, 6, o 8
        
        // Iterative deepening
        while (Date.now() - startTime < timeLimit * 0.9 && depth <= maxDepth) {
            let currentBest = moves[0];
            let currentScore = -Infinity;
            
            for (const move of moves) {
                const piece = this.getPiece(move.fromRow, move.fromCol);
                if (!piece) continue;
                
                const captured = this.makeMoveInternal(move, false);
                
                const score = -this.alphaBeta(depth - 1, -Infinity, Infinity, false, depth);
                
                const oldEnPassant = this.enPassantSquare;
                const oldCastling = {...this.castlingRights};
                const oldHalfMove = this.halfMoveClock;
                this.undoMoveInternal(move, piece, captured, oldEnPassant, oldCastling, oldHalfMove, false);
                
                if (score > currentScore) {
                    currentScore = score;
                    currentBest = move;
                }
                
                if (Date.now() - startTime > timeLimit * 0.9) break;
            }
            
            if (Date.now() - startTime < timeLimit * 0.9) {
                bestMove = currentBest;
                bestScore = currentScore;
                console.log(`Profondità ${depth}: Migliore mossa ${bestMove.toString()} con punteggio ${bestScore.toFixed(2)}`);
                depth++;
            }
        }
        
        console.log(`Profondità finale: ${depth-1}, Valutazione: ${bestScore.toFixed(2)}`);
        return bestMove;
    }

    getComputerMove() {
        let searchTime;
        switch (this.difficulty) {
            case 1: searchTime = 2000; break;
            case 2: searchTime = 5000; break;
            case 3: searchTime = 10000; break;
            default: searchTime = 3000;
        }
        
        const move = this.findBestMove(searchTime);
        return move;
    }

    // ============= GAME STATE =============
    updateGameState() {
        const moves = this.getAllMoves(this.currentPlayer);
        
        if (moves.length === 0) {
            if (this.isInCheck(this.currentPlayer)) {
                this.gameState = GameState.CHECKMATE;
                this.result = this.currentPlayer === Color.WHITE ? "0-1" : "1-0";
            } else {
                this.gameState = GameState.STALEMATE;
                this.result = "1/2-1/2";
            }
        } else if (this.isInCheck(this.currentPlayer)) {
            this.gameState = GameState.CHECK;
        } else {
            this.gameState = GameState.NORMAL;
        }
        
        // Controlla patta per 50 mosse
        if (this.halfMoveClock >= 100) {
            this.gameState = GameState.DRAW;
            this.result = "1/2-1/2";
        }
    }

    // ============= VISUALIZZAZIONE =============
    displayBoard() {
        console.clear();
        console.log("=== LUNA CHESS ENGINE (Advanced) ===\n");
        
        console.log("  a b c d e f g h");
        console.log("  ----------------");
        
        for (let row = 0; row < 8; row++) {
            let line = (8 - row) + '|';
            for (let col = 0; col < 8; col++) {
                const piece = this.getPiece(row, col);
                if (piece) {
                    line += piece.getSymbol() + ' ';
                } else {
                    // Evidenzia la casella en passant
                    if (this.enPassantSquare && 
                        this.enPassantSquare[0] === row && 
                        this.enPassantSquare[1] === col) {
                        line += '* ';
                    } else {
                        line += '. ';
                    }
                }
            }
            line += '|' + (8 - row);
            console.log(line);
        }
        
        console.log("  ----------------");
        console.log("  a b c d e f g h\n");
        
        console.log(`Turno: ${this.currentPlayer === Color.WHITE ? 'BIANCO' : 'NERO'}`);
        console.log(`Difficoltà: ${this.difficulty === 1 ? 'Facile' : this.difficulty === 2 ? 'Medio' : 'Difficile'}`);
        console.log(`Mossa n°: ${Math.floor((this.fullMoveNumber + 1) / 2)}`);
        
        // Stato del gioco
        switch (this.gameState) {
            case GameState.CHECK:
                console.log("⚠️  SCACCO!");
                break;
            case GameState.CHECKMATE:
                console.log("♔ SCACCO MATTO!");
                break;
            case GameState.STALEMATE:
                console.log("🤝 STALLO");
                break;
            case GameState.DRAW:
                console.log("🤝 PATTA (50 mosse)");
                break;
        }
        
        if (this.gameState >= GameState.CHECKMATE) {
            console.log(`\n=== PARTITA TERMINATA ===`);
            console.log(`Risultato: ${this.result}`);
        } else {
            const moves = this.getAllMoves(this.currentPlayer);
            console.log(`Mosse disponibili: ${moves.length}`);
            
            // Mostra arrocco disponibile
            if (this.currentPlayer === Color.WHITE) {
                if (this.castlingRights.whiteKingside) console.log("Arrocco corto disponibile (O-O)");
                if (this.castlingRights.whiteQueenside) console.log("Arrocco lungo disponibile (O-O-O)");
            } else {
                if (this.castlingRights.blackKingside) console.log("Arrocco corto disponibile (O-O)");
                if (this.castlingRights.blackQueenside) console.log("Arrocco lungo disponibile (O-O-O)");
            }
        }
        
        console.log("\n" + "=".repeat(50));
    }
}

// ============= INTERFACCIA GIOCO =============
class ChessGame {
    constructor() {
        this.engine = new ChessEngine();
        this.rl = readline.createInterface({
            input: process.stdin,
            output: process.stdout
        });
        this.playerColor = Color.WHITE;
        this.isPlayerTurn = true;
    }

    start() {
        console.clear();
        console.log("=== BENVENUTO IN LUNA CHESS (Advanced) ===");
        console.log("Scegli il colore:");
        console.log("1. Bianco (primo a muovere)");
        console.log("2. Nero (il computer muove per primo)");
        
        this.rl.question("Scelta (1-2): ", (choice) => {
            if (choice === '2') {
                this.playerColor = Color.BLACK;
                this.isPlayerTurn = false;
            }
            
            this.chooseDifficulty();
        });
    }

    chooseDifficulty() {
        console.log("\nScegli la difficoltà:");
        console.log("1. Facile (profondità 3-4)");
        console.log("2. Medio (profondità 5-6)");
        console.log("3. Difficile (profondità 7-8)");
        
        this.rl.question("Scelta (1-3): ", (choice) => {
            const diff = parseInt(choice);
            if (diff >= 1 && diff <= 3) {
                this.engine.difficulty = diff;
            } else {
                this.engine.difficulty = 2;
            }
            
            console.log(`\nDifficoltà impostata: ${this.engine.difficulty === 1 ? 'Facile' : 
                        this.engine.difficulty === 2 ? 'Medio' : 'Difficile'}`);
            console.log("\nPremi INVIO per iniziare...");
            
            this.rl.question("", () => {
                this.gameLoop();
            });
        });
    }

    gameLoop() {
        if (this.engine.gameState >= GameState.CHECKMATE) {
            this.showGameResult();
            return;
        }
        
        this.engine.displayBoard();
        
        if (this.isPlayerTurn) {
            this.playerTurn();
        } else {
            this.computerTurn();
        }
    }

    playerTurn() {
        console.log("\n--- TUO TURNO ---");
        console.log("Comandi:");
        console.log("  <mossa>      es: e2e4, g1f3, e7e8q (promozione)");
        console.log("  O-O          arrocco corto");
        console.log("  O-O-O        arrocco lungo");
        console.log("  undo         annulla l'ultima mossa");
        console.log("  quit         esci dal gioco");
        console.log("  help         mostra mosse disponibili");
        
        this.rl.question("\nLa tua mossa: ", (input) => {
            input = input.trim().toLowerCase();
            
            if (input === 'quit' || input === 'exit') {
                console.log("\nGrazie per aver giocato!");
                this.rl.close();
                return;
            }
            
            if (input === 'undo') {
                if (this.engine.undoMove()) {
                    this.isPlayerTurn = !this.isPlayerTurn;
                    console.log("Mossa annullata.");
                } else {
                    console.log("Nessuna mossa da annullare.");
                }
                setTimeout(() => this.gameLoop(), 1000);
                return;
            }
            
            if (input === 'help') {
                const moves = this.engine.getAllMoves(this.engine.currentPlayer);
                console.log(`\nMosse disponibili (${moves.length}):`);
                const moveStrs = moves.map(m => m.toString());
                for (let i = 0; i < moveStrs.length; i += 8) {
                    console.log(moveStrs.slice(i, i + 8).join(', '));
                }
                setTimeout(() => this.gameLoop(), 3000);
                return;
            }
            
            // Converti notazione arrocco
            if (input === 'o-o' || input === '0-0') {
                input = this.engine.currentPlayer === Color.WHITE ? 'e1g1' : 'e8g8';
            } else if (input === 'o-o-o' || input === '0-0-0') {
                input = this.engine.currentPlayer === Color.WHITE ? 'e1c1' : 'e8c8';
            }
            
            // Verifica formato mossa
            if (!/^[a-h][1-8][a-h][1-8][qrnb]?$/i.test(input)) {
                console.log("Formato mossa non valido! Usa formato come: e2e4");
                setTimeout(() => this.gameLoop(), 1500);
                return;
            }
            
            // Esegui mossa
            if (this.engine.makeMove(input)) {
                console.log(`Mossa eseguita: ${input}`);
                this.isPlayerTurn = false;
                setTimeout(() => this.gameLoop(), 1000);
            } else {
                console.log("Mossa non valida! Prova ancora.");
                setTimeout(() => this.gameLoop(), 1500);
            }
        });
    }

    computerTurn() {
        console.log("\n--- TURNO DEL COMPUTER ---");
        console.log("Il computer sta pensando...");
        
        try {
            const startTime = Date.now();
            const move = this.engine.getComputerMove();
            const thinkTime = Date.now() - startTime;
            
            if (move) {
                const moveStr = move.toString();
                console.log(`Computer gioca: ${moveStr} (${thinkTime}ms)`);
                this.engine.makeMove(moveStr);
                this.isPlayerTurn = true;
            } else {
                console.log("Il computer non ha mosse disponibili!");
                this.isPlayerTurn = true; // Passa il turno
            }
        } catch (error) {
            console.error(`Errore nel turno del computer: ${error.message}`);
            console.log("Passo il turno...");
            this.isPlayerTurn = true;
        }
        
        setTimeout(() => this.gameLoop(), 2000);
    }

    showGameResult() {
        this.engine.displayBoard();
        
        console.log("\n" + "=".repeat(50));
        console.log("GAME OVER");
        console.log("=".repeat(50));
        
        const result = this.engine.result;
        if (result === "1-0") {
            console.log("Vittoria del BIANCO!");
            console.log(this.playerColor === Color.WHITE ? "HAI VINTO! 🎉" : "Hai perso...");
        } else if (result === "0-1") {
            console.log("Vittoria del NERO!");
            console.log(this.playerColor === Color.BLACK ? "HAI VINTO! 🎉" : "Hai perso...");
        } else {
            console.log("PAREGGIO! 1/2-1/2");
        }
        
        console.log(`\nMosse totali: ${Math.floor(this.engine.fullMoveNumber / 2)}`);
        console.log("\nGrazie per aver giocato!");
        this.rl.close();
    }
}

// ============= AVVIO GIOCO =============
function main() {
    console.log("Avvio Luna Chess Engine (Advanced)...");
    
    const game = new ChessGame();
    game.start();
}

// Controlla se siamo in ambiente Node.js
if (typeof require !== 'undefined' && require.main === module) {
    main();
} else {
    console.log("Questo gioco deve essere eseguito con Node.js");
    console.log("Comando: node chess_game.js");
}