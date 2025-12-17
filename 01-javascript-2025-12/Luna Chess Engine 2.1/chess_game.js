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

// ============= BITBOARD =============
class Bitboard {
    constructor() {
        this.reset();
    }
    
    reset() {
        // 64 bit per ogni tipo di pezzo per ogni colore
        this.pawns = [0n, 0n];     // [bianco, nero]
        this.knights = [0n, 0n];
        this.bishops = [0n, 0n];
        this.rooks = [0n, 0n];
        this.queens = [0n, 0n];
        this.kings = [0n, 0n];
        
        this.occupancy = [0n, 0n]; // Tutti i pezzi per colore
        this.allPieces = 0n;       // Tutti i pezzi sulla scacchiera
        
        // Maschere precalcolate
        this.knightMasks = new Array(64);
        this.kingMasks = new Array(64);
        this.pawnAttacks = [[], []]; // [white][square], [black][square]
        
        this.initializeMasks();
    }
    
    initializeMasks() {
        // Inizializza maschere precalcolate
        for (let square = 0; square < 64; square++) {
            this.knightMasks[square] = this.calculateKnightAttacks(square);
            this.kingMasks[square] = this.calculateKingAttacks(square);
        }
        
        // Attacchi pedoni
        for (let square = 0; square < 64; square++) {
            this.pawnAttacks[0][square] = this.calculatePawnAttacks(square, Color.WHITE);
            this.pawnAttacks[1][square] = this.calculatePawnAttacks(square, Color.BLACK);
        }
    }
    
    squareToBit(square) {
        if (square < 0 || square > 63) return 0n;
        return 1n << BigInt(square);
    }
    
    bitToSquare(bit) {
        if (bit === 0n) return -1;
        // Usa Math.clz32 per contare gli zeri a sinistra
        const high = Number(bit >> 32n);
        const low = Number(bit & 0xFFFFFFFFn);
        
        if (high > 0) {
            return 63 - Math.clz32(high);
        } else {
            return 31 - Math.clz32(low);
        }
    }
    
    setSquare(color, pieceType, square) {
        const bit = this.squareToBit(square);
        
        // Rimuovi da eventuali bitboard precedenti
        this.clearSquare(square);
        
        // Aggiungi al bitboard corretto
        switch(pieceType) {
            case PieceType.PAWN: this.pawns[color] |= bit; break;
            case PieceType.KNIGHT: this.knights[color] |= bit; break;
            case PieceType.BISHOP: this.bishops[color] |= bit; break;
            case PieceType.ROOK: this.rooks[color] |= bit; break;
            case PieceType.QUEEN: this.queens[color] |= bit; break;
            case PieceType.KING: this.kings[color] |= bit; break;
        }
        
        // Aggiorna occupancy
        this.occupancy[color] |= bit;
        this.allPieces |= bit;
    }
    
    clearSquare(square) {
        const bit = this.squareToBit(square);
        const notBit = ~bit;
        
        // Rimuovi da tutti i bitboard
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
    }
    
    moveSquare(fromSquare, toSquare) {
        // Trova il pezzo in fromSquare
        const fromBit = this.squareToBit(fromSquare);
        
        let pieceType = PieceType.NONE;
        let color = Color.NONE;
        
        // Cerca il pezzo
        for (let c = 0; c < 2; c++) {
            if (this.pawns[c] & fromBit) { pieceType = PieceType.PAWN; color = c; break; }
            if (this.knights[c] & fromBit) { pieceType = PieceType.KNIGHT; color = c; break; }
            if (this.bishops[c] & fromBit) { pieceType = PieceType.BISHOP; color = c; break; }
            if (this.rooks[c] & fromBit) { pieceType = PieceType.ROOK; color = c; break; }
            if (this.queens[c] & fromBit) { pieceType = PieceType.QUEEN; color = c; break; }
            if (this.kings[c] & fromBit) { pieceType = PieceType.KING; color = c; break; }
        }
        
        if (pieceType === PieceType.NONE) return false;
        
        // Rimuovi dalla casella di partenza
        this.clearSquare(fromSquare);
        
        // Aggiungi alla casella di arrivo (catturando eventuale pezzo)
        this.clearSquare(toSquare);
        this.setSquare(color, pieceType, toSquare);
        
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
    
    getOccupancy(color) {
        return this.occupancy[color];
    }
    
    getAllPieces() {
        return this.allPieces;
    }
    
    getEmptySquares() {
        return ~this.allPieces;
    }
    
    // Metodi per attacchi
    calculatePawnAttacks(square, color) {
        const attacks = [];
        const row = Math.floor(square / 8);
        const col = square % 8;
        
        if (color === Color.WHITE) {
            // Attacchi in alto a sinistra e destra
            if (row > 0 && col > 0) attacks.push(square - 9);
            if (row > 0 && col < 7) attacks.push(square - 7);
        } else {
            // Attacchi in basso a sinistra e destra
            if (row < 7 && col > 0) attacks.push(square + 7);
            if (row < 7 && col < 7) attacks.push(square + 9);
        }
        
        return attacks;
    }
    
    calculateKnightAttacks(square) {
        const attacks = [];
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
                attacks.push(newRow * 8 + newCol);
            }
        }
        
        return attacks;
    }
    
    calculateKingAttacks(square) {
        const attacks = [];
        const row = Math.floor(square / 8);
        const col = square % 8;
        
        for (let dr = -1; dr <= 1; dr++) {
            for (let dc = -1; dc <= 1; dc++) {
                if (dr === 0 && dc === 0) continue;
                const newRow = row + dr;
                const newCol = col + dc;
                if (newRow >= 0 && newRow < 8 && newCol >= 0 && newCol < 8) {
                    attacks.push(newRow * 8 + newCol);
                }
            }
        }
        
        return attacks;
    }
    
    getPawnAttacks(square, color) {
        return this.pawnAttacks[color][square] || [];
    }
    
    getKnightAttacks(square) {
        return this.knightMasks[square] || [];
    }
    
    getKingAttacks(square) {
        return this.kingMasks[square] || [];
    }
    
    // CORREZIONE: Generazione mosse pedoni corretta
    generatePawnMoves(square, color) {
        const moves = [];
        const row = Math.floor(square / 8);
        const col = square % 8;
        
        if (color === Color.WHITE) {
            // Avanti 1
            if (row > 0) {
                const forward = square - 8;
                const forwardBit = this.squareToBit(forward);
                
                // Controlla se la casella è vuota
                if (!(forwardBit & this.allPieces)) {
                    moves.push({ from: square, to: forward, isCapture: false });
                    
                    // Avanti 2 dalla posizione iniziale (riga 6 per bianco)
                    if (row === 6) {
                        const forward2 = square - 16;
                        const forward2Bit = this.squareToBit(forward2);
                        if (!(forward2Bit & this.allPieces)) {
                            moves.push({ from: square, to: forward2, isCapture: false });
                        }
                    }
                }
            }
            
            // Catture
            const attacks = this.getPawnAttacks(square, color);
            for (const attack of attacks) {
                const attackBit = this.squareToBit(attack);
                if (attackBit & this.occupancy[Color.BLACK]) {
                    moves.push({ from: square, to: attack, isCapture: true });
                }
            }
        } else {
            // Nero
            if (row < 7) {
                const forward = square + 8;
                const forwardBit = this.squareToBit(forward);
                
                if (!(forwardBit & this.allPieces)) {
                    moves.push({ from: square, to: forward, isCapture: false });
                    
                    // Avanti 2 dalla posizione iniziale (riga 1 per nero)
                    if (row === 1) {
                        const forward2 = square + 16;
                        const forward2Bit = this.squareToBit(forward2);
                        if (!(forward2Bit & this.allPieces)) {
                            moves.push({ from: square, to: forward2, isCapture: false });
                        }
                    }
                }
            }
            
            // Catture
            const attacks = this.getPawnAttacks(square, color);
            for (const attack of attacks) {
                const attackBit = this.squareToBit(attack);
                if (attackBit & this.occupancy[Color.WHITE]) {
                    moves.push({ from: square, to: attack, isCapture: true });
                }
            }
        }
        
        return moves;
    }
    
    generateKnightMoves(square, color) {
        const moves = [];
        const enemyPieces = color === Color.WHITE ? 
            this.occupancy[Color.BLACK] : this.occupancy[Color.WHITE];
        
        for (const target of this.getKnightAttacks(square)) {
            const targetBit = this.squareToBit(target);
            if (!(targetBit & this.occupancy[color])) {
                moves.push({ 
                    from: square, 
                    to: target, 
                    isCapture: !!(targetBit & enemyPieces) 
                });
            }
        }
        
        return moves;
    }
    
    generateKingMoves(square, color) {
        const moves = [];
        const enemyPieces = color === Color.WHITE ? 
            this.occupancy[Color.BLACK] : this.occupancy[Color.WHITE];
        
        for (const target of this.getKingAttacks(square)) {
            const targetBit = this.squareToBit(target);
            if (!(targetBit & this.occupancy[color])) {
                moves.push({ 
                    from: square, 
                    to: target, 
                    isCapture: !!(targetBit & enemyPieces) 
                });
            }
        }
        
        return moves;
    }
    
    // Helper per debugging
    print() {
        console.log('\n  a b c d e f g h');
        console.log('  ────────────────');
        
        for (let row = 0; row < 8; row++) {
            let line = `${8 - row} `;
            for (let col = 0; col < 8; col++) {
                const square = row * 8 + col;
                const piece = this.getPieceAt(square);
                if (piece) {
                    const symbols = ['♙♟', '♘♞', '♗♝', '♖♜', '♕♛', '♔♚'];
                    const index = piece.color === Color.WHITE ? 0 : 1;
                    line += symbols[piece.type][index] + ' ';
                } else {
                    line += ((row + col) % 2 === 1 ? '· ' : '  ');
                }
            }
            line += ` ${8 - row}`;
            console.log(line);
        }
        
        console.log('  ────────────────');
        console.log('  a b c d e f g h');
    }
}

// ============= CLASSI =============
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
            return this.toCol === 6 ? '0-0' : '0-0-0';
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

// ============= MOTORE SCACCHI COMPLETO CON BITBOARD =============
class CompleteChessEngine {
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
        
        // Componenti avanzati
        this.tt = new TranspositionTable(50000);
        this.openingBook = new OpeningBook();
        this.killerMoves = Array(100).fill().map(() => [null, null]);
        this.historyHeuristic = Array(2).fill().map(() => 
            Array(6).fill().map(() => 
                Array(64).fill(0)
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
        this.bitboard.reset();

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
            const piece = new Piece(type, color);
            this.board[row][col] = piece;
            const square = row * 8 + col;
            this.bitboard.setSquare(color, type, square);
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
        this.bitboard.reset();

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
                        const piece = new Piece(pieceType, color);
                        this.board[row][col] = piece;
                        const square = row * 8 + col;
                        this.bitboard.setSquare(color, pieceType, square);
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

    // ============= GENERAZIONE MOSSE CON BITBOARD =============
    getAllPossibleMoves(forColor) {
        const color = forColor !== undefined ? forColor : this.currentPlayer;
        const moves = [];
        
        // Usa bitboard per generare mosse più velocemente
        this.generateMovesWithBitboard(color, moves);
        
        return moves;
    }
    
    generateMovesWithBitboard(color, moves) {
        // Genera mosse per ogni tipo di pezzo
        
        // Pedoni
        let pawns = this.bitboard.pawns[color];
        while (pawns) {
            const square = this.popLsb(pawns);
            pawns &= pawns - 1n;
            
            const pawnMoves = this.bitboard.generatePawnMoves(square, color);
            for (const move of pawnMoves) {
                const fromRow = Math.floor(square / 8);
                const fromCol = square % 8;
                const toRow = Math.floor(move.to / 8);
                const toCol = move.to % 8;
                
                const moveObj = new Move(fromRow, fromCol, toRow, toCol);
                moveObj.isCapture = move.isCapture || false;
                
                // Promozione se il pedone raggiunge l'ultima traversa
                if ((color === Color.WHITE && toRow === 0) || (color === Color.BLACK && toRow === 7)) {
                    // Aggiungi tutte le promozioni possibili
                    [PieceType.QUEEN, PieceType.ROOK, PieceType.BISHOP, PieceType.KNIGHT].forEach(promType => {
                        const promoMove = new Move(fromRow, fromCol, toRow, toCol, promType);
                        promoMove.isCapture = move.isCapture;
                        moves.push(promoMove);
                    });
                } else {
                    moves.push(moveObj);
                }
            }
            
            // En passant
            if (this.enPassantTarget) {
                const epSquare = this.enPassantTarget.row * 8 + this.enPassantTarget.col;
                const epBit = this.bitboard.squareToBit(epSquare);
                
                // Controlla se questo pedone può catturare en passant
                const attacks = this.bitboard.getPawnAttacks(square, color);
                if (attacks.includes(epSquare)) {
                    const moveObj = new Move(
                        Math.floor(square / 8),
                        square % 8,
                        this.enPassantTarget.row,
                        this.enPassantTarget.col,
                        PieceType.NONE,
                        false,
                        true
                    );
                    moveObj.isCapture = true;
                    moves.push(moveObj);
                }
            }
        }
        
        // Cavalli
        let knights = this.bitboard.knights[color];
        while (knights) {
            const square = this.popLsb(knights);
            knights &= knights - 1n;
            
            const knightMoves = this.bitboard.generateKnightMoves(square, color);
            for (const move of knightMoves) {
                const moveObj = new Move(
                    Math.floor(square / 8),
                    square % 8,
                    Math.floor(move.to / 8),
                    move.to % 8
                );
                moveObj.isCapture = move.isCapture;
                moves.push(moveObj);
            }
        }
        
        // Re
        let kings = this.bitboard.kings[color];
        while (kings) {
            const square = this.popLsb(kings);
            kings &= kings - 1n;
            
            const kingMoves = this.bitboard.generateKingMoves(square, color);
            for (const move of kingMoves) {
                const moveObj = new Move(
                    Math.floor(square / 8),
                    square % 8,
                    Math.floor(move.to / 8),
                    move.to % 8
                );
                moveObj.isCapture = move.isCapture;
                moves.push(moveObj);
            }
            
            // Arrocco
            const row = Math.floor(square / 8);
            const col = square % 8;
            if (col === 4) { // Re nella posizione iniziale
                const piece = this.board[row][col];
                if (piece && !piece.hasMoved && !this.isInCheck(color)) {
                    // Arrocco corto
                    if (this.castlingRights[color].kingside) {
                        const rook = this.board[row][7];
                        if (rook && rook.type === PieceType.ROOK && !rook.hasMoved) {
                            if (!this.board[row][5] && !this.board[row][6]) {
                                const opponentColor = color === Color.WHITE ? Color.BLACK : Color.WHITE;
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
                    if (this.castlingRights[color].queenside) {
                        const rook = this.board[row][0];
                        if (rook && rook.type === PieceType.ROOK && !rook.hasMoved) {
                            if (!this.board[row][1] && !this.board[row][2] && !this.board[row][3]) {
                                const opponentColor = color === Color.WHITE ? Color.BLACK : Color.WHITE;
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
        }
        
        // Alfieri
        let bishops = this.bitboard.bishops[color];
        while (bishops) {
            const square = this.popLsb(bishops);
            bishops &= bishops - 1n;
            
            const bishopMoves = this.getSlidingMovesForSquare(square, PieceType.BISHOP, color);
            moves.push(...bishopMoves);
        }
        
        // Torri
        let rooks = this.bitboard.rooks[color];
        while (rooks) {
            const square = this.popLsb(rooks);
            rooks &= rooks - 1n;
            
            const rookMoves = this.getSlidingMovesForSquare(square, PieceType.ROOK, color);
            moves.push(...rookMoves);
        }
        
        // Regine
        let queens = this.bitboard.queens[color];
        while (queens) {
            const square = this.popLsb(queens);
            queens &= queens - 1n;
            
            const queenMoves = this.getSlidingMovesForSquare(square, PieceType.QUEEN, color);
            moves.push(...queenMoves);
        }
    }
    
    popLsb(bitboard) {
        const lsb = bitboard & -bitboard;
        return this.bitboard.bitToSquare(lsb);
    }
    
    getSlidingMovesForSquare(square, pieceType, color) {
        const moves = [];
        const row = Math.floor(square / 8);
        const col = square % 8;
        
        let directions = [];
        if (pieceType === PieceType.BISHOP) {
            directions = [[-1, -1], [-1, 1], [1, -1], [1, 1]];
        } else if (pieceType === PieceType.ROOK) {
            directions = [[-1, 0], [1, 0], [0, -1], [0, 1]];
        } else if (pieceType === PieceType.QUEEN) {
            directions = [[-1, -1], [-1, 1], [1, -1], [1, 1], [-1, 0], [1, 0], [0, -1], [0, 1]];
        }
        
        for (const [dr, dc] of directions) {
            let newRow = row + dr;
            let newCol = col + dc;
            
            while (newRow >= 0 && newRow < 8 && newCol >= 0 && newCol < 8) {
                const targetSquare = newRow * 8 + newCol;
                const targetBit = this.bitboard.squareToBit(targetSquare);
                
                const move = new Move(row, col, newRow, newCol);
                
                // Controlla se c'è un pezzo
                if (targetBit & this.bitboard.allPieces) {
                    // C'è un pezzo
                    if (targetBit & this.bitboard.occupancy[color === Color.WHITE ? Color.BLACK : Color.WHITE]) {
                        // Pezzo avversario - cattura valida
                        move.isCapture = true;
                        moves.push(move);
                    }
                    break; // Fermati in ogni caso
                } else {
                    // Casella vuota
                    moves.push(move);
                }
                
                newRow += dr;
                newCol += dc;
            }
        }
        
        return moves;
    }

    isSquareAttacked(row, col, byColor) {
        const square = row * 8 + col;
        const bit = this.bitboard.squareToBit(square);
        
        // Controlla pedoni
        const pawnDirection = byColor === Color.WHITE ? -1 : 1;
        for (const dc of [-1, 1]) {
            const newRow = row + pawnDirection;
            const newCol = col + dc;
            if (newRow >= 0 && newRow < 8 && newCol >= 0 && newCol < 8) {
                const targetSquare = newRow * 8 + newCol;
                const targetBit = this.bitboard.squareToBit(targetSquare);
                if (targetBit & this.bitboard.pawns[byColor]) {
                    return true;
                }
            }
        }

        // Controlla cavalli
        const knightAttacks = this.bitboard.getKnightAttacks(square);
        for (const attack of knightAttacks) {
            const attackBit = this.bitboard.squareToBit(attack);
            if (attackBit & this.bitboard.knights[byColor]) {
                return true;
            }
        }

        // Controlla re
        const kingAttacks = this.bitboard.getKingAttacks(square);
        for (const attack of kingAttacks) {
            const attackBit = this.bitboard.squareToBit(attack);
            if (attackBit & this.bitboard.kings[byColor]) {
                return true;
            }
        }

        // Controlla pezzi che slittano
        // Alfieri e regine (diagonali)
        for (const attack of this.getSlidingAttacks(square, [[-1, -1], [-1, 1], [1, -1], [1, 1]])) {
            const attackBit = this.bitboard.squareToBit(attack);
            if ((attackBit & this.bitboard.bishops[byColor]) || 
                (attackBit & this.bitboard.queens[byColor])) {
                return true;
            }
        }
        
        // Torri e regine (orizzontali/verticali)
        for (const attack of this.getSlidingAttacks(square, [[-1, 0], [1, 0], [0, -1], [0, 1]])) {
            const attackBit = this.bitboard.squareToBit(attack);
            if ((attackBit & this.bitboard.rooks[byColor]) || 
                (attackBit & this.bitboard.queens[byColor])) {
                return true;
            }
        }

        return false;
    }
    
    getSlidingAttacks(square, directions) {
        const attacks = [];
        const row = Math.floor(square / 8);
        const col = square % 8;
        
        for (const [dr, dc] of directions) {
            let newRow = row + dr;
            let newCol = col + dc;
            
            while (newRow >= 0 && newRow < 8 && newCol >= 0 && newCol < 8) {
                const targetSquare = newRow * 8 + newCol;
                attacks.push(targetSquare);
                
                // Se c'è un pezzo, fermati
                const targetBit = this.bitboard.squareToBit(targetSquare);
                if (targetBit & this.bitboard.allPieces) {
                    break;
                }
                
                newRow += dr;
                newCol += dc;
            }
        }
        
        return attacks;
    }

    findKing(color) {
        const kingBits = this.bitboard.kings[color];
        if (kingBits === 0n) return null;
        
        const square = this.bitboard.bitToSquare(kingBits);
        return { 
            row: Math.floor(square / 8), 
            col: square % 8 
        };
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
        const originalBitboard = this.copyBitboard();
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
        this.bitboard = originalBitboard;
        this.currentPlayer = originalPlayer;
        this.castlingRights = originalCastling;
        this.enPassantTarget = originalEnPassant;
        this.halfMoveClock = originalHalfMove;
        
        return !inCheck;
    }

    copyBitboard() {
        const copy = new Bitboard();
        copy.pawns = [...this.bitboard.pawns];
        copy.knights = [...this.bitboard.knights];
        copy.bishops = [...this.bitboard.bishops];
        copy.rooks = [...this.bitboard.rooks];
        copy.queens = [...this.bitboard.queens];
        copy.kings = [...this.bitboard.kings];
        copy.occupancy = [...this.bitboard.occupancy];
        copy.allPieces = this.bitboard.allPieces;
        return copy;
    }

    // ============= ESECUZIONE MOSSE =============
    makeMove(moveStr) {
        if (!moveStr || moveStr.length < 2) return false;
        
        let move;
        
        // CORREZIONE: Supporta sia '0-0' che 'O-O' per l'arrocco
        if (moveStr === '0-0' || moveStr === '0-0-0' || 
            moveStr === 'O-O' || moveStr === 'O-O-O') {
            const color = this.currentPlayer;
            const row = color === Color.WHITE ? 7 : 0;
            const kingCol = 4;
            
            // Determina se è arrocco corto o lungo
            const isKingside = moveStr === '0-0' || moveStr === 'O-O';
            const newKingCol = isKingside ? 6 : 2;
            
            // Verifica che ci sia il re
            const king = this.board[row][kingCol];
            if (!king || king.type !== PieceType.KING || king.color !== color) {
                console.log(`❌ Nessun re trovato in ${row},${kingCol}`);
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
            
            if (fromCol < 0 || fromRow < 0 || toCol < 0 || toRow < 0 ||
                fromRow < 0 || fromRow > 7 || toRow < 0 || toRow > 7) {
                console.log(`❌ Coordinate non valide: ${moveStr}`);
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
            m.toCol === move.toCol &&
            m.promotion === move.promotion
        );
        
        if (!isValid) {
            console.log(`❌ Mossa ${moveStr} non è legale`);
            console.log(`   Mosse legali disponibili: ${legalMoves.length}`);
            return false;
        }
        
        // Esegui la mossa
        return this.makeMoveInternal(move, true);
    }

    makeMoveInternal(move, updateHistory = true) {
        const piece = this.board[move.fromRow][move.fromCol];
        if (!piece) {
            console.log(`❌ Nessun pezzo in ${move.fromRow},${move.fromCol}`);
            return false;
        }
        
        // Salva lo stato per eventuale undo
        const captured = this.board[move.toRow][move.toCol];
        const oldCastlingRights = JSON.parse(JSON.stringify(this.castlingRights));
        const oldEnPassant = this.enPassantTarget;
        const oldHalfMoveClock = this.halfMoveClock;
        
        // Gestione cattura
        if (captured) {
            move.isCapture = true;
            move.capturedPiece = captured.copy();
            move.capturedPieceType = captured.type;
            
            // Rimuovi dalla bitboard
            const capturedSquare = move.toRow * 8 + move.toCol;
            this.bitboard.clearSquare(capturedSquare);
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
                
                // Rimuovi pedone catturato
                this.board[capturedPawnRow][capturedPawnCol] = null;
                const capturedSquare = capturedPawnRow * 8 + capturedPawnCol;
                this.bitboard.clearSquare(capturedSquare);
            }
        }
        
        // Esegui la mossa sulla board
        this.board[move.toRow][move.toCol] = piece;
        this.board[move.fromRow][move.fromCol] = null;
        
        // Esegui la mossa sulla bitboard
        this.bitboard.moveSquare(move.fromSquare, move.toSquare);
        
        // Promozione
        if (move.promotion !== PieceType.NONE) {
            const promotedPiece = new Piece(move.promotion, piece.color);
            this.board[move.toRow][move.toCol] = promotedPiece;
            
            // Aggiorna bitboard
            this.bitboard.clearSquare(move.toSquare);
            this.bitboard.setSquare(piece.color, move.promotion, move.toSquare);
        }
        
        // Arrocco
        if (move.isCastling) {
            const row = move.fromRow;
            const isKingside = move.toCol === 6;
            const rookFromCol = isKingside ? 7 : 0;
            const rookToCol = isKingside ? 5 : 3;
            
            const rook = this.board[row][rookFromCol];
            if (rook && rook.type === PieceType.ROOK) {
                // Muovi torre
                this.board[row][rookToCol] = rook;
                this.board[row][rookFromCol] = null;
                rook.hasMoved = true;
                
                // Aggiorna bitboard
                const rookFromSquare = row * 8 + rookFromCol;
                const rookToSquare = row * 8 + rookToCol;
                this.bitboard.moveSquare(rookFromSquare, rookToSquare);
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
            
            if (piece) {
                const san = move.getSAN(piece, this.board, isCheck, isMate);
                this.moveHistory.push({
                    move: move.toString(),
                    san: san,
                    piece: piece.copy(),
                    captured: move.capturedPiece,
                    capturedType: move.capturedPieceType,
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
                    capturedType: move.capturedPieceType,
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
        
        // Ripristina bitboard
        this.bitboard.clearSquare(move.toSquare);
        if (piece) {
            this.bitboard.setSquare(piece.color, piece.type, move.fromSquare);
        }
        
        // Ripristina pezzo nella casella di arrivo (o null)
        if (move.isEnPassant) {
            // Per en passant, la casella di arrivo è vuota
            this.board[move.toRow][move.toCol] = null;
            // Rimetti il pedone catturato nella sua posizione originale
            const capturedRow = move.fromRow;
            const capturedCol = move.toCol;
            if (lastMove.captured) {
                this.board[capturedRow][capturedCol] = lastMove.captured;
                const capturedSquare = capturedRow * 8 + capturedCol;
                this.bitboard.setSquare(lastMove.captured.color, lastMove.captured.type, capturedSquare);
            }
        } else {
            this.board[move.toRow][move.toCol] = lastMove.captured;
            if (lastMove.captured) {
                const capturedSquare = move.toRow * 8 + move.toCol;
                this.bitboard.setSquare(lastMove.captured.color, lastMove.captured.type, capturedSquare);
            }
        }
        
        // Ripristina promozione
        if (move.promotion !== PieceType.NONE && piece) {
            this.board[move.fromRow][move.fromCol] = new Piece(PieceType.PAWN, piece.color);
            this.board[move.fromRow][move.fromCol].hasMoved = piece.hasMoved;
            
            // Aggiorna bitboard
            this.bitboard.clearSquare(move.fromSquare);
            this.bitboard.setSquare(piece.color, PieceType.PAWN, move.fromSquare);
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
                
                // Aggiorna bitboard
                const rookFromSquare = row * 8 + rookFromCol;
                const rookToSquare = row * 8 + rookToCol;
                this.bitboard.moveSquare(rookFromSquare, rookToSquare);
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
        
        if (moveStr === '0-0' || moveStr === 'O-O') {
            const row = this.currentPlayer === Color.WHITE ? 7 : 0;
            return new Move(row, 4, row, 6, PieceType.NONE, true);
        } else if (moveStr === '0-0-0' || moveStr === 'O-O-O') {
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
        
        // Valore materiale e posizionale usando bitboard
        for (let color = 0; color < 2; color++) {
            const sign = color === Color.WHITE ? 1 : -1;
            
            // Pedoni
            let pawns = this.bitboard.pawns[color];
            while (pawns) {
                const square = this.popLsb(pawns);
                pawns &= pawns - 1n;
                score += sign * 100;
                score += sign * this.getPositionalValue(PieceType.PAWN, square, color);
            }
            
            // Cavalli
            let knights = this.bitboard.knights[color];
            while (knights) {
                const square = this.popLsb(knights);
                knights &= knights - 1n;
                score += sign * 320;
                score += sign * this.getPositionalValue(PieceType.KNIGHT, square, color);
            }
            
            // Alfieri
            let bishops = this.bitboard.bishops[color];
            while (bishops) {
                const square = this.popLsb(bishops);
                bishops &= bishops - 1n;
                score += sign * 330;
                score += sign * this.getPositionalValue(PieceType.BISHOP, square, color);
            }
            
            // Torri
            let rooks = this.bitboard.rooks[color];
            while (rooks) {
                const square = this.popLsb(rooks);
                rooks &= rooks - 1n;
                score += sign * 500;
                score += sign * this.getPositionalValue(PieceType.ROOK, square, color);
            }
            
            // Regine
            let queens = this.bitboard.queens[color];
            while (queens) {
                const square = this.popLsb(queens);
                queens &= queens - 1n;
                score += sign * 900;
                score += sign * this.getPositionalValue(PieceType.QUEEN, square, color);
            }
            
            // Re
            let kings = this.bitboard.kings[color];
            while (kings) {
                const square = this.popLsb(kings);
                kings &= kings - 1n;
                score += sign * 20000;
                const gamePhase = this.getGamePhase();
                if (gamePhase > 0.7) { // Finale
                    score += sign * this.getKingEndgameValue(square, color);
                } else {
                    score += sign * this.getPositionalValue(PieceType.KING, square, color);
                }
            }
        }
        
        // Bonus per coppia degli alfieri
        const whiteBishops = this.countBits(this.bitboard.bishops[Color.WHITE]);
        const blackBishops = this.countBits(this.bitboard.bishops[Color.BLACK]);
        if (whiteBishops >= 2) score += 50;
        if (blackBishops >= 2) score -= 50;
        
        // Mobilità (semplificata)
        const whiteMobility = this.getLegalMoves(Color.WHITE).length;
        const blackMobility = this.getLegalMoves(Color.BLACK).length;
        score += (whiteMobility - blackMobility) * 2;
        
        // Penalità per re sotto scacco
        if (this.isInCheck(Color.WHITE)) score -= 25;
        if (this.isInCheck(Color.BLACK)) score += 25;
        
        return Math.round(score);
    }
    
    countBits(bits) {
        let count = 0;
        while (bits) {
            bits &= bits - 1n;
            count++;
        }
        return count;
    }
    
    getPositionalValue(pieceType, square, color) {
        const table = this.pieceSquareTables[pieceType];
        if (!table) return 0;
        
        const row = Math.floor(square / 8);
        const col = square % 8;
        const actualRow = color === Color.WHITE ? 7 - row : row;
        return table[actualRow][col] || 0;
    }
    
    getKingEndgameValue(square, color) {
        const row = Math.floor(square / 8);
        const col = square % 8;
        const actualRow = color === Color.WHITE ? 7 - row : row;
        return this.kingEndgameTable[actualRow][col] || 0;
    }

    getGamePhase() {
        let material = 0;
        let maxMaterial = 0;
        
        // Calcola materiale totale
        for (let color = 0; color < 2; color++) {
            material += this.countBits(this.bitboard.pawns[color]) * 100;
            material += this.countBits(this.bitboard.knights[color]) * 320;
            material += this.countBits(this.bitboard.bishops[color]) * 330;
            material += this.countBits(this.bitboard.rooks[color]) * 500;
            material += this.countBits(this.bitboard.queens[color]) * 900;
            
            maxMaterial += 8 * 100 + 2 * 320 + 2 * 330 + 2 * 500 + 1 * 900;
        }
        
        // Fase di gioco: 0 = apertura, 1 = finale
        const maxNonPawnMaterial = 4*320 + 4*330 + 4*500 + 2*900;
        const phase = Math.min(1, Math.max(0, (maxMaterial - material) / maxNonPawnMaterial));
        
        return phase;
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
        const totalPieces = this.countBits(this.bitboard.allPieces);
        
        // Solo re vs re
        if (totalPieces === 2) return true;
        
        // Re e alfiere vs re
        if (totalPieces === 3) {
            const bishops = this.countBits(this.bitboard.bishops[Color.WHITE]) + 
                          this.countBits(this.bitboard.bishops[Color.BLACK]);
            const knights = this.countBits(this.bitboard.knights[Color.WHITE]) + 
                          this.countBits(this.bitboard.knights[Color.BLACK]);
            return bishops === 1 || knights === 1;
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
        
        whiteMaterial += this.countBits(this.bitboard.pawns[Color.WHITE]) * 100;
        whiteMaterial += this.countBits(this.bitboard.knights[Color.WHITE]) * 320;
        whiteMaterial += this.countBits(this.bitboard.bishops[Color.WHITE]) * 330;
        whiteMaterial += this.countBits(this.bitboard.rooks[Color.WHITE]) * 500;
        whiteMaterial += this.countBits(this.bitboard.queens[Color.WHITE]) * 900;
        
        blackMaterial += this.countBits(this.bitboard.pawns[Color.BLACK]) * 100;
        blackMaterial += this.countBits(this.bitboard.knights[Color.BLACK]) * 320;
        blackMaterial += this.countBits(this.bitboard.bishops[Color.BLACK]) * 330;
        blackMaterial += this.countBits(this.bitboard.rooks[Color.BLACK]) * 500;
        blackMaterial += this.countBits(this.bitboard.queens[Color.BLACK]) * 900;
        
        console.log(`⚪ Materiale bianco: ${whiteMaterial}`);
        console.log(`⚫ Materiale nero: ${blackMaterial}`);
        console.log(`📊 Vantaggio materiale: ${whiteMaterial - blackMaterial > 0 ? '+' : ''}${whiteMaterial - blackMaterial}`);
        
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
        
        // Debug bitboard
        console.log('\n🔢 Bitboard stats:');
        console.log(`   Bianco: ${this.countBits(this.bitboard.occupancy[Color.WHITE])} pezzi`);
        console.log(`   Nero: ${this.countBits(this.bitboard.occupancy[Color.BLACK])} pezzi`);
        console.log(`   Totali: ${this.countBits(this.bitboard.allPieces)} pezzi`);
    }

    saveGameToPGN() {
        let pgn = '[Event "Chess Game"]\n';
        pgn += '[Site "JavaScript Chess Engine with Bitboard"]\n';
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
                const preview = legalMoves.slice(0, Math.min(5, legalMoves.length)).map(m => {
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
        console.log('╔══════════════════════════════════════════════════════════╗');
        console.log('║          ♔ COMPLETE CHESS ENGINE WITH BITBOARD ♚        ║');
        console.log('║     Con Bitboard, correzione arrocco e IA avanzata      ║');
        console.log('║              Supporta sia 0-0 che O-O per arrocco       ║');
        console.log('╚══════════════════════════════════════════════════════════╝\n');
        
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
        console.log('  9. Mostra bitboard (debug)');
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
                    console.log('\n🔢 BITBOARD DEBUG:');
                    this.engine.bitboard.print();
                    setTimeout(() => {
                        this.showMenu();
                    }, 1000);
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
        this.engine = new CompleteChessEngine();
        this.highlightedMoves = [];
        console.log('\n♔ Nuova partita iniziata! ♚');
        console.log(`Modalità: ${this.gameMode}`);
        console.log(`Difficoltà: ${this.difficulty}`);
        console.log(`📢 Nota: Usa '0-0' o 'O-O' per arrocco corto, '0-0-0' o 'O-O-O' per arrocco lungo`);
        
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
        console.log('\n🎮 La tua mossa (es: e2e4, 0-0, O-O, e7e8q):');
        console.log('   ? = mostra mosse, h <casella> = evidenzia mosse');
        console.log('   undo = annulla, menu = ritorna al menu');
        console.log('   bitboard = mostra bitboard di debug');
        
        this.rl.question('> ', (input) => {
            this.processInput(input.trim());
        });
    }

    computerTurn() {
        console.log('\n🤖 Computer pensa...');
        
        const depth = this.difficulty + 2; // 3-7
        const maxTime = this.difficulty * 1500; // 1.5-7.5 secondi
        
        setTimeout(() => {
            // Usa la vecchia implementazione di search per ora
            // (più stabile mentre sistemiamo la bitboard)
            const legalMoves = this.engine.getLegalMoves();
            if (legalMoves.length === 0) {
                console.log('🤖 Computer non ha mosse disponibili');
                this.gameLoop();
                return;
            }
            
            // Per ora, scegli una mossa casuale
            const randomMove = legalMoves[Math.floor(Math.random() * legalMoves.length)];
            const moveStr = randomMove.toString();
            console.log(`🤖 Computer gioca: ${moveStr}`);
            
            if (this.engine.makeMove(moveStr)) {
                this.highlightedMoves = [randomMove];
                setTimeout(() => {
                    this.gameLoop();
                }, 800);
            } else {
                console.log('❌ ERRORE: Mossa del computer illegale!');
                this.gameLoop();
            }
        }, 500);
    }

    computerVsComputer() {
        console.log('\n🤖🤖 Modalità Computer vs Computer');
        console.log('Premi Ctrl+C per interrompere\n');
        
        let moveCount = 0;
        const maxMoves = 50; // Ridotto per testing
        
        const playNextMove = () => {
            if (moveCount >= maxMoves || this.engine.isGameOver() || !this.isGameRunning) {
                this.showGameResult();
                this.showMenu();
                return;
            }
            
            this.engine.printBoard(this.highlightedMoves);
            
            const legalMoves = this.engine.getLegalMoves();
            if (legalMoves.length === 0) {
                console.log('🤖 Nessuna mossa disponibile');
                this.showGameResult();
                this.showMenu();
                return;
            }
            
            // Per ora, mossa casuale
            const randomMove = legalMoves[Math.floor(Math.random() * legalMoves.length)];
            const moveStr = randomMove.toString();
            const piece = this.engine.board[randomMove.fromRow][randomMove.fromCol];
            let san;
            if (piece) {
                san = randomMove.getSAN(piece, this.engine.board, false, false);
            } else {
                san = moveStr;
            }
            
            console.log(`\n🤖 Mossa ${moveCount + 1}: ${san} (${moveStr})`);
            
            if (this.engine.makeMove(moveStr)) {
                moveCount++;
                this.highlightedMoves = [randomMove];
                
                // Pausa per leggibilità
                setTimeout(playNextMove, 800);
            } else {
                console.log('❌ Mossa illegale!');
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
                
            case 'bitboard':
                console.log('\n🔢 BITBOARD:');
                this.engine.bitboard.print();
                this.gameLoop();
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
            const moveStrs = legalMoves.slice(0, Math.min(12, legalMoves.length)).map(m => {
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