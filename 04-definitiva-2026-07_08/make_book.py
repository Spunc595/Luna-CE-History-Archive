import chess
import chess.pgn
import chess.polyglot
import struct

def encode_move(move):
    # Formato Polyglot standard per la mossa (16-bit)
    # 0-5: from_square, 6-11: to_square, 12-14: promotion
    from_sq = move.from_square
    to_sq = move.to_square
    prom = 0
    if move.promotion == chess.KNIGHT: prom = 1
    elif move.promotion == chess.BISHOP: prom = 2
    elif move.promotion == chess.ROOK: prom = 3
    elif move.promotion == chess.QUEEN: prom = 4
    
    return (from_sq) | (to_sq << 6) | (prom << 12)

pgn_path = "italiana.pgn"
bin_path = "book.bin"

with open(pgn_path) as pgn_file, open(bin_path, "wb") as bin_file:
    while True:
        game = chess.pgn.read_game(pgn_file)
        if game is None: break
        
        board = game.board()
        for move in game.mainline_moves():
            # Calcolo chiave Polyglot (Zobrist hash)
            key = chess.polyglot.zobrist_hash(board)
            move_code = encode_move(move)
            
            # Scrittura: Key (8 bytes), Move (2), Weight (2), Learn (4)
            # Big-endian (>)
            bin_file.write(struct.pack(">QHH I", key, move_code, 100, 0))
            
            board.push(move)

print("File book.bin creato con successo!")