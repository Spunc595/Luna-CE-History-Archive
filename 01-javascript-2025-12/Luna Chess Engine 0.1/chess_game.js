const readline = require('readline');
const { Console } = require('console');

// ============= COSTANTI =============
const PieceType = {
    PAWN: 0, KNIGHT: 1, BISHOP: 2, ROOK: 3, QUEEN: 4, KING: 5, NONE: 6
};

const Color = {
    WHITE: 0, BLACK: 1, NONE: 2
};

const PIECE_VALUES = {
    [PieceType.PAWN]: 100,
    [PieceType.KNIGHT]: 320,
    [PieceType.BISHOP]: 330,
    [PieceType.ROOK]: 500,
    [PieceType.QUEEN]: 900,
    [PieceType.KING]: 20000
};

// ============= CLASSI =============
class Move {
    constructor(fromRow, fromCol, toRow, toCol, promotion = PieceType.NONE) {
        this.fromRow = fromRow;
        this.fromCol = fromCol;
        this.toRow = toRow;
        this.toCol = toCol;
        this.promotion = promotion;
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
        if (moveStr.length < 4) return null;
        
        const cols = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h'];
        const fromCol = cols.indexOf(moveStr[0]);
        const fromRow = 8 - parseInt(moveStr[1]);
        const toCol = cols.indexOf(moveStr[2]);
        const toRow = 8 - parseInt(moveStr[3]);
        
        if (fromCol < 0 || fromRow < 0 || toCol < 0 || toRow < 0) return null;
        
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

class ChessEngine {
    constructor() {
        this.board = this.createBoard();
        this.currentPlayer = Color.WHITE;
        this.moveHistory = [];
        this.gameOver = false;
        this.result = null;
        this.difficulty = 2; // 1=facile, 2=medio, 3=difficile
    }

    createBoard() {
        const board = Array(8).fill().map(() => Array(8).fill(null));
        
        // Pedoni
        for (let i = 0; i < 8; i++) {
            board[1][i] = new Piece(PieceType.PAWN, Color.BLACK);
            board[6][i] = new Piece(PieceType.PAWN, Color.WHITE);
        }
        
        // Pezzi
        const pieces = [PieceType.ROOK, PieceType.KNIGHT, PieceType.BISHOP,
                       PieceType.QUEEN, PieceType.KING,
                       PieceType.BISHOP, PieceType.KNIGHT, PieceType.ROOK];
        
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

    // ============= GENERAZIONE MOSSE =============
    getAllMoves(color) {
        const moves = [];
        for (let row = 0; row < 8; row++) {
            for (let col = 0; col < 8; col++) {
                const piece = this.getPiece(row, col);
                if (piece && piece.color === color) {
                    this.addMoves(row, col, piece, moves);
                }
            }
        }
        return moves;
    }

    addMoves(row, col, piece, moves) {
        switch (piece.type) {
            case PieceType.PAWN: this.addPawnMoves(row, col, piece, moves); break;
            case PieceType.KNIGHT: this.addKnightMoves(row, col, piece, moves); break;
            case PieceType.BISHOP: this.addBishopMoves(row, col, piece, moves); break;
            case PieceType.ROOK: this.addRookMoves(row, col, piece, moves); break;
            case PieceType.QUEEN: this.addQueenMoves(row, col, piece, moves); break;
            case PieceType.KING: this.addKingMoves(row, col, piece, moves); break;
        }
    }

    addPawnMoves(row, col, piece, moves) {
        const dir = piece.color === Color.WHITE ? -1 : 1;
        const startRow = piece.color === Color.WHITE ? 6 : 1;
        const promoRow = piece.color === Color.WHITE ? 0 : 7;
        
        // Avanti
        const newRow = row + dir;
        if (newRow >= 0 && newRow < 8 && !this.getPiece(newRow, col)) {
            if (newRow === promoRow) {
                [PieceType.QUEEN, PieceType.ROOK, PieceType.BISHOP, PieceType.KNIGHT]
                    .forEach(p => moves.push(new Move(row, col, newRow, col, p)));
            } else {
                moves.push(new Move(row, col, newRow, col));
                
                // Doppio passo iniziale
                if (row === startRow && !this.getPiece(newRow + dir, col)) {
                    moves.push(new Move(row, col, newRow + dir, col));
                }
            }
        }
        
        // Catture
        for (const dc of [-1, 1]) {
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
            }
        }
    }

    addKnightMoves(row, col, piece, moves) {
        const movesList = [[2,1],[2,-1],[-2,1],[-2,-1],[1,2],[1,-2],[-1,2],[-1,-2]];
        for (const [dr, dc] of movesList) {
            const newRow = row + dr, newCol = col + dc;
            if (newRow >= 0 && newRow < 8 && newCol >= 0 && newCol < 8) {
                const target = this.getPiece(newRow, newCol);
                if (!target || target.color !== piece.color) {
                    moves.push(new Move(row, col, newRow, newCol));
                }
            }
        }
    }

    addSliderMoves(row, col, piece, directions, moves) {
        for (const [dr, dc] of directions) {
            let r = row + dr, c = col + dc;
            while (r >= 0 && r < 8 && c >= 0 && c < 8) {
                const target = this.getPiece(r, c);
                if (!target) {
                    moves.push(new Move(row, col, r, c));
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

    addBishopMoves(row, col, piece, moves) {
        this.addSliderMoves(row, col, piece, [[1,1],[1,-1],[-1,1],[-1,-1]], moves);
    }

    addRookMoves(row, col, piece, moves) {
        this.addSliderMoves(row, col, piece, [[1,0],[-1,0],[0,1],[0,-1]], moves);
    }

    addQueenMoves(row, col, piece, moves) {
        this.addSliderMoves(row, col, piece, 
            [[1,0],[-1,0],[0,1],[0,-1],[1,1],[1,-1],[-1,1],[-1,-1]], moves);
    }

    addKingMoves(row, col, piece, moves) {
        for (let dr = -1; dr <= 1; dr++) {
            for (let dc = -1; dc <= 1; dc++) {
                if (dr === 0 && dc === 0) continue;
                const newRow = row + dr, newCol = col + dc;
                if (newRow >= 0 && newRow < 8 && newCol >= 0 && newCol < 8) {
                    const target = this.getPiece(newRow, newCol);
                    if (!target || target.color !== piece.color) {
                        moves.push(new Move(row, col, newRow, newCol));
                    }
                }
            }
        }
    }

    // ============= ESECUZIONE MOSSE =============
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
        
        const piece = this.getPiece(move.fromRow, move.fromCol);
        const captured = this.getPiece(move.toRow, move.toCol);
        
        // Salva per undo
        this.moveHistory.push({
            move: legalMove,
            piece: piece.copy(),
            captured: captured ? captured.copy() : null
        });
        
        // Esegui
        this.setPiece(move.toRow, move.toCol, piece);
        this.setPiece(move.fromRow, move.fromCol, null);
        piece.hasMoved = true;
        
        if (move.promotion !== PieceType.NONE) {
            this.setPiece(move.toRow, move.toCol, new Piece(move.promotion, piece.color));
        }
        
        // Cambia turno
        this.currentPlayer = this.currentPlayer === Color.WHITE ? Color.BLACK : Color.WHITE;
        
        // Controlla fine partita
        this.checkGameOver();
        
        return true;
    }

    undoMove() {
        if (this.moveHistory.length === 0) return false;
        
        const lastMove = this.moveHistory.pop();
        const move = lastMove.move;
        
        // Ripristina
        this.setPiece(move.fromRow, move.fromCol, lastMove.piece);
        if (lastMove.captured) {
            this.setPiece(move.toRow, move.toCol, lastMove.captured);
        } else {
            this.setPiece(move.toRow, move.toCol, null);
        }
        
        this.currentPlayer = lastMove.piece.color;
        this.gameOver = false;
        this.result = null;
        
        return true;
    }

    // ============= VALUTAZIONE =============
    evaluate() {
        let score = 0;
        for (let row = 0; row < 8; row++) {
            for (let col = 0; col < 8; col++) {
                const piece = this.getPiece(row, col);
                if (piece) {
                    const value = piece.getValue();
                    score += piece.color === Color.WHITE ? value : -value;
                }
            }
        }
        return score;
    }

    // ============= MOTORE IA =============
    getComputerMove() {
        const moves = this.getAllMoves(this.currentPlayer);
        if (moves.length === 0) return null;
        
        // In base alla difficoltà
        let searchTime;
        switch (this.difficulty) {
            case 1: searchTime = 1000; break; // Facile: 1 secondo
            case 2: searchTime = 3000; break; // Medio: 3 secondi
            case 3: searchTime = 5000; break; // Difficile: 5 secondi
            default: searchTime = 2000;
        }
        
        return this.findBestMove(searchTime);
    }

    findBestMove(timeLimit) {
        const startTime = Date.now();
        const moves = this.getAllMoves(this.currentPlayer);
        if (moves.length === 0) return null;
        
        // Ordina mosse (migliora la ricerca)
        moves.sort((a, b) => {
            const pieceA = this.getPiece(a.fromRow, a.fromCol);
            const pieceB = this.getPiece(b.fromRow, b.fromCol);
            const targetA = this.getPiece(a.toRow, a.toCol);
            const targetB = this.getPiece(b.toRow, b.toCol);
            
            // Priorità: catture > mosse centrali > altre
            let scoreA = 0, scoreB = 0;
            if (targetA) scoreA += targetA.getValue();
            if (targetB) scoreB += targetB.getValue();
            
            // Bonus per centro
            const centerBonus = (r, c) => {
                const distFromCenter = Math.abs(3.5 - r) + Math.abs(3.5 - c);
                return 10 - distFromCenter * 2;
            };
            scoreA += centerBonus(a.toRow, a.toCol);
            scoreB += centerBonus(b.toRow, b.toCol);
            
            return scoreB - scoreA;
        });
        
        let bestMove = moves[0];
        let bestScore = -Infinity;
        
        // Ricerca semplice (per ogni mossa valuta la posizione risultante)
        for (const move of moves) {
            if (Date.now() - startTime > timeLimit) break;
            
            // Esegui mossa temporanea
            const piece = this.getPiece(move.fromRow, move.fromCol);
            const captured = this.getPiece(move.toRow, move.toCol);
            
            this.setPiece(move.toRow, move.toCol, piece);
            this.setPiece(move.fromRow, move.fromCol, null);
            this.currentPlayer = this.currentPlayer === Color.WHITE ? Color.BLACK : Color.WHITE;
            
            // Valuta posizione
            const score = -this.evaluate();
            
            // Ripristina
            this.currentPlayer = this.currentPlayer === Color.WHITE ? Color.BLACK : Color.WHITE;
            this.setPiece(move.fromRow, move.fromCol, piece);
            this.setPiece(move.toRow, move.toCol, captured);
            
            if (score > bestScore) {
                bestScore = score;
                bestMove = move;
            }
        }
        
        return bestMove;
    }

    // ============= CONTROLLO FINE PARTITA =============
    checkGameOver() {
        const moves = this.getAllMoves(this.currentPlayer);
        
        if (moves.length === 0) {
            // Verifica se è scacco
            let kingFound = false;
            for (let row = 0; row < 8 && !kingFound; row++) {
                for (let col = 0; col < 8 && !kingFound; col++) {
                    const piece = this.getPiece(row, col);
                    if (piece && piece.type === PieceType.KING && piece.color === this.currentPlayer) {
                        kingFound = true;
                    }
                }
            }
            
            if (kingFound) {
                // Controlla se il re è sotto scacco
                const opponentColor = this.currentPlayer === Color.WHITE ? Color.BLACK : Color.WHITE;
                const opponentMoves = this.getAllMoves(opponentColor);
                let kingInCheck = false;
                
                for (const move of opponentMoves) {
                    const target = this.getPiece(move.toRow, move.toCol);
                    if (target && target.type === PieceType.KING && target.color === this.currentPlayer) {
                        kingInCheck = true;
                        break;
                    }
                }
                
                if (kingInCheck) {
                    this.gameOver = true;
                    this.result = this.currentPlayer === Color.WHITE ? "0-1" : "1-0";
                } else {
                    this.gameOver = true;
                    this.result = "1/2-1/2"; // Stallo
                }
            }
        }
    }

    // ============= VISUALIZZAZIONE =============
    displayBoard() {
        console.clear();
        console.log("=== LUNA CHESS ENGINE ===\n");
        
        console.log("  a b c d e f g h");
        console.log("  ----------------");
        
        for (let row = 0; row < 8; row++) {
            let line = (8 - row) + '|';
            for (let col = 0; col < 8; col++) {
                const piece = this.getPiece(row, col);
                if (piece) {
                    line += piece.getSymbol() + ' ';
                } else {
                    line += '. ';
                }
            }
            line += '|' + (8 - row);
            console.log(line);
        }
        
        console.log("  ----------------");
        console.log("  a b c d e f g h\n");
        
        console.log(`Turno: ${this.currentPlayer === Color.WHITE ? 'BIANCO' : 'NERO'}`);
        console.log(`Difficoltà: ${this.difficulty === 1 ? 'Facile' : this.difficulty === 2 ? 'Medio' : 'Difficile'}`);
        
        if (this.gameOver) {
            console.log(`\n=== PARTITA TERMINATA ===`);
            console.log(`Risultato: ${this.result}`);
        } else {
            const moves = this.getAllMoves(this.currentPlayer);
            console.log(`Mosse disponibili: ${moves.length}`);
            
            if (moves.length > 0 && moves.length <= 10) {
                console.log(`Mosse: ${moves.map(m => m.toString()).join(', ')}`);
            }
        }
        
        console.log("\n" + "=".repeat(40));
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
        this.playerColor = Color.WHITE; // Giocatore gioca con il bianco
        this.isPlayerTurn = true;
    }

    start() {
        console.clear();
        console.log("=== BENVENUTO IN LUNA CHESS ===");
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
        console.log("1. Facile (pensiero veloce)");
        console.log("2. Medio (pensiero moderato)");
        console.log("3. Difficile (pensiero profondo)");
        
        this.rl.question("Scelta (1-3): ", (choice) => {
            this.engine.difficulty = parseInt(choice) || 2;
            console.log(`\nDifficoltà impostata: ${this.engine.difficulty === 1 ? 'Facile' : 
                        this.engine.difficulty === 2 ? 'Medio' : 'Difficile'}`);
            console.log("\nPremi INVIO per iniziare...");
            
            this.rl.question("", () => {
                this.gameLoop();
            });
        });
    }

    gameLoop() {
        if (this.engine.gameOver) {
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
                    // Se annulliamo, dobbiamo invertire il turno
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
                console.log(moves.map(m => m.toString()).join(', '));
                setTimeout(() => this.gameLoop(), 2000);
                return;
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
        
        setTimeout(() => {
            const move = this.engine.getComputerMove();
            if (move) {
                const moveStr = move.toString();
                console.log(`Computer gioca: ${moveStr}`);
                this.engine.makeMove(moveStr);
                this.isPlayerTurn = true;
            } else {
                console.log("Il computer non ha mosse disponibili!");
            }
            
            setTimeout(() => this.gameLoop(), 2000);
        }, 1000);
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
            console.log("PAREGGIO!");
        }
        
        console.log("\nGrazie per aver giocato!");
        this.rl.close();
    }
}

// ============= AVVIO GIOCO =============
function main() {
    console.log("Avvio Luna Chess Engine...");
    
    const game = new ChessGame();
    game.start();
}

// Controlla se siamo in Node.js
if (typeof require !== 'undefined' && require.main === module) {
    main();
} else {
    console.log("Questo gioco deve essere eseguito con Node.js");
    console.log("Comando: node chess_game.js");
}