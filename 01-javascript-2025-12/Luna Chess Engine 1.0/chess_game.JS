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

    getValue() {
        const pieceValues = [100, 320, 330, 500, 900, 20000];
        return pieceValues[this.type] || 0;
    }
}

// ============= MOTORE SCACCHI SEMPLICE E SICURO =============
class SimpleChessEngine {
    constructor() {
        this.board = Array(8).fill().map(() => Array(8).fill(null));
        this.currentPlayer = Color.WHITE;
        this.moveHistory = [];
        this.initializeBoard();
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

        // Avanti 1
        let newRow = row + direction;
        if (newRow >= 0 && newRow < 8) {
            if (!this.board[newRow][col]) {
                moves.push(new Move(row, col, newRow, col));
                
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
                        moves.push(new Move(row, col, newRow, newCol));
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
    }

    isSquareAttacked(row, col, byColor) {
        // Controlla tutti i pezzi del colore attaccante
        for (let r = 0; r < 8; r++) {
            for (let c = 0; c < 8; c++) {
                const piece = this.board[r][c];
                if (piece && piece.color === byColor) {
                    // Genera tutte le mosse per questo pezzo
                    const moves = [];
                    this.getMovesForPiece(r, c, moves);
                    
                    // Controlla se qualche mossa arriva alla casella target
                    for (const move of moves) {
                        if (move.toRow === row && move.toCol === col) {
                            // Per i pedoni, solo le diagonali sono attacchi
                            if (piece.type === PieceType.PAWN) {
                                if (Math.abs(move.fromCol - col) === 1) {
                                    return true;
                                }
                            } else {
                                return true;
                            }
                        }
                    }
                }
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
            // Crea una copia del board
            const tempBoard = this.copyBoard();
            
            // Esegui la mossa sulla copia
            const piece = tempBoard[move.fromRow][move.fromCol];
            tempBoard[move.toRow][move.toCol] = piece;
            tempBoard[move.fromRow][move.fromCol] = null;
            
            // Crea un engine temporaneo con la copia
            const tempEngine = new SimpleChessEngine();
            tempEngine.board = tempBoard;
            tempEngine.currentPlayer = color === Color.WHITE ? Color.BLACK : Color.WHITE;
            
            // Controlla se il re è sotto scacco dopo la mossa
            if (!tempEngine.isInCheck(color)) {
                legalMoves.push(move);
            }
        }

        return legalMoves;
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

    // ============= ESECUZIONE MOSSE =============
    makeMove(moveStr) {
        if (!moveStr || moveStr.length < 4) return false;
        
        const cols = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h'];
        const fromCol = cols.indexOf(moveStr[0].toLowerCase());
        const fromRow = 8 - parseInt(moveStr[1]);
        const toCol = cols.indexOf(moveStr[2].toLowerCase());
        const toRow = 8 - parseInt(moveStr[3]);
        
        if (fromCol < 0 || fromRow < 0 || toCol < 0 || toRow < 0) return false;
        
        const move = new Move(fromRow, fromCol, toRow, toCol);
        
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
        const piece = this.board[move.fromRow][move.fromCol];
        this.board[move.toRow][move.toCol] = piece;
        this.board[move.fromRow][move.fromCol] = null;
        piece.hasMoved = true;
        
        // Cambia giocatore
        this.currentPlayer = this.currentPlayer === Color.WHITE ? Color.BLACK : Color.WHITE;
        
        // Salva nella history
        this.moveHistory.push({
            move: moveStr,
            piece: piece,
            captured: null
        });
        
        return true;
    }

    undoMove() {
        if (this.moveHistory.length === 0) return false;
        
        const lastMove = this.moveHistory.pop();
        const moveStr = lastMove.move;
        
        const cols = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h'];
        const fromCol = cols.indexOf(moveStr[0].toLowerCase());
        const fromRow = 8 - parseInt(moveStr[1]);
        const toCol = cols.indexOf(moveStr[2].toLowerCase());
        const toRow = 8 - parseInt(moveStr[3]);
        
        // Ripristina il pezzo
        const piece = lastMove.piece;
        this.board[fromRow][fromCol] = piece;
        this.board[toRow][toCol] = null;
        
        // Ripristina giocatore
        this.currentPlayer = this.currentPlayer === Color.WHITE ? Color.BLACK : Color.WHITE;
        
        return true;
    }

    // ============= VALUTAZIONE =============
    evaluate() {
        let score = 0;
        
        // Valore materiale
        for (let row = 0; row < 8; row++) {
            for (let col = 0; col < 8; col++) {
                const piece = this.board[row][col];
                if (piece) {
                    const value = piece.getValue();
                    score += piece.color === Color.WHITE ? value : -value;
                }
            }
        }
        
        // Bonus per mobilità
        const whiteMoves = this.getLegalMoves(Color.WHITE).length;
        const blackMoves = this.getLegalMoves(Color.BLACK).length;
        score += (whiteMoves - blackMoves) * 0.5;
        
        // Penalità per re sotto scacco
        if (this.isInCheck(Color.WHITE)) score -= 50;
        if (this.isInCheck(Color.BLACK)) score += 50;
        
        // Assicurati che il valore non sia Infinity
        if (!isFinite(score)) {
            console.error("ERRORE: Valutazione infinita rilevata!");
            return 0;
        }
        
        return Math.round(score);
    }

    // Controlla condizioni di fine partita
    isGameOver() {
        // Scaccomatto
        if (this.isInCheck(this.currentPlayer)) {
            return this.getLegalMoves().length === 0;
        }
        
        // Stallo
        return this.getLegalMoves().length === 0;
    }

    getResult() {
        if (this.isInCheck(this.currentPlayer) && this.getLegalMoves().length === 0) {
            return this.currentPlayer === Color.WHITE ? "0-1" : "1-0";
        } else if (this.getLegalMoves().length === 0) {
            return "1/2-1/2";
        }
        return null;
    }

    // ============= ALGORITMO DI RICERCA SICURO =============
    search(depth = 3) {
        console.log(`🤖 Computer pensa a profondità ${depth}...`);
        
        const legalMoves = this.getLegalMoves();
        if (legalMoves.length === 0) return null;
        
        let bestMove = null;
        let bestScore = this.currentPlayer === Color.WHITE ? -Infinity : Infinity;
        
        for (const move of legalMoves) {
            // Esegui la mossa
            const piece = this.board[move.fromRow][move.fromCol];
            const captured = this.board[move.toRow][move.toCol];
            
            this.board[move.toRow][move.toCol] = piece;
            this.board[move.fromRow][move.fromCol] = null;
            
            const oldPlayer = this.currentPlayer;
            this.currentPlayer = this.currentPlayer === Color.WHITE ? Color.BLACK : Color.WHITE;
            
            // Valuta
            const score = this.minimax(depth - 1, -Infinity, Infinity, false);
            
            // Annulla mossa
            this.board[move.fromRow][move.fromCol] = piece;
            this.board[move.toRow][move.toCol] = captured;
            this.currentPlayer = oldPlayer;
            
            // Controlla se il punteggio è valido
            if (!isFinite(score)) {
                console.error(`ERRORE: Punteggio infinito per mossa ${move.toString()}`);
                continue;
            }
            
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
        }
        
        console.log(`🤖 Valutazione: ${bestScore}`);
        return bestMove;
    }

    minimax(depth, alpha, beta, maximizingPlayer) {
        if (depth === 0 || this.isGameOver()) {
            const score = this.evaluate();
            return isFinite(score) ? score : 0;
        }
        
        const legalMoves = this.getLegalMoves();
        
        if (maximizingPlayer) {
            let maxEval = -Infinity;
            for (const move of legalMoves) {
                // Esegui mossa temporanea
                const piece = this.board[move.fromRow][move.fromCol];
                const captured = this.board[move.toRow][move.toCol];
                
                this.board[move.toRow][move.toCol] = piece;
                this.board[move.fromRow][move.fromCol] = null;
                
                const oldPlayer = this.currentPlayer;
                this.currentPlayer = this.currentPlayer === Color.WHITE ? Color.BLACK : Color.WHITE;
                
                const evalScore = this.minimax(depth - 1, alpha, beta, false);
                
                // Annulla mossa
                this.board[move.fromRow][move.fromCol] = piece;
                this.board[move.toRow][move.toCol] = captured;
                this.currentPlayer = oldPlayer;
                
                maxEval = Math.max(maxEval, evalScore);
                alpha = Math.max(alpha, evalScore);
                if (beta <= alpha) break;
            }
            return maxEval;
        } else {
            let minEval = Infinity;
            for (const move of legalMoves) {
                // Esegui mossa temporanea
                const piece = this.board[move.fromRow][move.fromCol];
                const captured = this.board[move.toRow][move.toCol];
                
                this.board[move.toRow][move.toCol] = piece;
                this.board[move.fromRow][move.fromCol] = null;
                
                const oldPlayer = this.currentPlayer;
                this.currentPlayer = this.currentPlayer === Color.WHITE ? Color.BLACK : Color.WHITE;
                
                const evalScore = this.minimax(depth - 1, alpha, beta, true);
                
                // Annulla mossa
                this.board[move.fromRow][move.fromCol] = piece;
                this.board[move.toRow][move.toCol] = captured;
                this.currentPlayer = oldPlayer;
                
                minEval = Math.min(minEval, evalScore);
                beta = Math.min(beta, evalScore);
                if (beta <= alpha) break;
            }
            return minEval;
        }
    }

    // ============= VISUALIZZAZIONE =============
    printBoard() {
        console.log('\n  a b c d e f g h');
        console.log('  ────────────────');
        
        for (let row = 0; row < 8; row++) {
            let line = `${8 - row} `;
            
            for (let col = 0; col < 8; col++) {
                const piece = this.board[row][col];
                if (piece) {
                    line += piece.getSymbol() + ' ';
                } else {
                    line += ((row + col) % 2 === 1 ? '· ' : '  ');
                }
            }
            
            line += ` ${8 - row}`;
            console.log(line);
        }
        
        console.log('  ────────────────');
        console.log('  a b c d e f g h\n');
        
        const player = this.currentPlayer === Color.WHITE ? "Bianco ♔" : "Nero ♚";
        console.log(`🎮 Turno: ${player}`);
        
        const legalMoves = this.getLegalMoves();
        console.log(`📋 Mosse legali: ${legalMoves.length}`);
        
        if (this.isInCheck(this.currentPlayer)) {
            console.log("⚔️  SCACCO!");
        }
        
        if (this.isGameOver()) {
            const result = this.getResult();
            console.log(`🏁 Partita terminata: ${result}`);
        } else {
            const score = this.evaluate();
            console.log(`💰 Valutazione: ${score > 0 ? '+' : ''}${score}`);
        }
    }
}

// ============= GIOCO SEMPLICE E STABILE =============
class SimpleChessGame {
    constructor() {
        this.engine = new SimpleChessEngine();
        this.rl = readline.createInterface({
            input: process.stdin,
            output: process.stdout
        });
        this.isGameRunning = true;
        this.computerColor = Color.BLACK; // Computer gioca nero di default
    }

    start() {
        console.log('╔═══════════════════════════════════════╗');
        console.log('║        ♔ CHESS ENGINE SEMPLICE ♚     ║');
        console.log('║        Versione ultra-stabile        ║');
        console.log('╚═══════════════════════════════════════╝\n');
        
        console.log('🎮 Comandi:');
        console.log('  <mossa>  - es. e2e4, e7e5');
        console.log('  undo     - annulla ultima mossa');
        console.log('  new      - nuova partita');
        console.log('  menu     - torna al menu');
        console.log('  quit     - esci');
        console.log('');
        
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
            const result = this.engine.getResult();
            console.log(`\n🏁 RISULTATO FINALE: ${result}\n`);
            
            this.rl.question('🔄 Nuova partita? (s/n): ', (answer) => {
                if (answer.toLowerCase() === 's') {
                    this.engine = new SimpleChessEngine();
                    this.gameLoop();
                } else {
                    this.isGameRunning = false;
                    this.rl.close();
                }
            });
            return;
        }
        
        if (this.engine.currentPlayer === this.computerColor) {
            this.computerTurn();
        } else {
            this.humanTurn();
        }
    }

    humanTurn() {
        console.log('\n🎮 La tua mossa (es: e2e4):');
        this.rl.question('> ', (input) => {
            this.processInput(input.trim());
        });
    }

    computerTurn() {
        console.log('\n🤖 Computer pensa...');
        
        // Usa una profondità ridotta per stabilità
        const depth = 2;
        
        setTimeout(() => {
            try {
                const bestMove = this.engine.search(depth);
                
                if (bestMove) {
                    const moveStr = bestMove.toString();
                    console.log(`🤖 Computer gioca: ${moveStr}`);
                    
                    // Usa makeMove per garantire che sia legale
                    if (this.engine.makeMove(moveStr)) {
                        setTimeout(() => {
                            this.gameLoop();
                        }, 500);
                    } else {
                        console.log('❌ ERRORE: Mossa del computer illegale!');
                        // Se la mossa è illegale, prova una mossa casuale
                        const legalMoves = this.engine.getLegalMoves();
                        if (legalMoves.length > 0) {
                            const randomMove = legalMoves[Math.floor(Math.random() * legalMoves.length)];
                            console.log(`🤖 Provo mossa casuale: ${randomMove.toString()}`);
                            this.engine.makeMove(randomMove.toString());
                            setTimeout(() => {
                                this.gameLoop();
                            }, 500);
                        } else {
                            this.gameLoop();
                        }
                    }
                } else {
                    console.log('🤖 Computer non ha mosse disponibili');
                    this.gameLoop();
                }
            } catch (error) {
                console.error('❌ ERRORE durante la ricerca del computer:', error.message);
                // In caso di errore, prova una mossa casuale
                const legalMoves = this.engine.getLegalMoves();
                if (legalMoves.length > 0) {
                    const randomMove = legalMoves[Math.floor(Math.random() * legalMoves.length)];
                    console.log(`🤖 Provo mossa casuale: ${randomMove.toString()}`);
                    this.engine.makeMove(randomMove.toString());
                    setTimeout(() => {
                        this.gameLoop();
                    }, 500);
                } else {
                    this.gameLoop();
                }
            }
        }, 1000);
    }

    processInput(input) {
        if (!input) {
            this.gameLoop();
            return;
        }
        
        const lowerInput = input.toLowerCase();
        
        switch(lowerInput) {
            case 'quit':
                this.isGameRunning = false;
                this.gameLoop();
                return;
                
            case 'menu':
            case 'new':
                this.engine = new SimpleChessEngine();
                console.log('♔ Nuova partita iniziata! ♚');
                this.gameLoop();
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
                
            default:
                // Controlla formato mossa
                if (input.length < 4) {
                    console.log('❌ Formato mossa non valido. Esempio: e2e4');
                    this.gameLoop();
                    return;
                }
                
                // Esegui mossa
                if (this.engine.makeMove(input)) {
                    console.log(`✅ Mossa eseguita: ${input}`);
                    
                    setTimeout(() => {
                        this.gameLoop();
                    }, 500);
                } else {
                    console.log('❌ Mossa illegale!');
                    
                    // Mostra mosse legali per aiutare
                    const legalMoves = this.engine.getLegalMoves();
                    if (legalMoves.length > 0) {
                        console.log('📋 Mosse legali disponibili:');
                        const moveStrs = legalMoves.map(m => m.toString());
                        console.log(moveStrs.slice(0, 10).join(', '));
                        if (moveStrs.length > 10) {
                            console.log(`... e altre ${moveStrs.length - 10} mosse`);
                        }
                    }
                    
                    this.gameLoop();
                }
        }
    }
}

// ============= AVVIA GIOCO =============
if (require.main === module) {
    const game = new SimpleChessGame();
    game.start();
}

module.exports = { SimpleChessEngine, SimpleChessGame };