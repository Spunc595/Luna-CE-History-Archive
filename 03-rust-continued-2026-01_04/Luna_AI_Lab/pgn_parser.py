import chess
import chess.pgn
import sys

# --- CONFIGURAZIONE ---
INPUT_PGN = "partite_generate.pgn"   # Metti qui il nome del file che sta creando lo script
OUTPUT_TXT = "dataset_new.txt"       # Il file pronto per il training

def parse_pgn():
    print(f"Apro il file PGN: {INPUT_PGN}...")
    pgn = open(INPUT_PGN)
    
    output = open(OUTPUT_TXT, "w")
    games_count = 0
    positions_count = 0

    while True:
        try:
            game = chess.pgn.read_game(pgn)
            if game is None:
                break # Fine del file
        except Exception as e:
            print(f"Errore lettura partita: {e}")
            continue

        games_count += 1
        
        # Gestione Risultato
        result = game.headers.get("Result", "*")
        if result == "1-0":
            score = 1.0
        elif result == "0-1":
            score = 0.0
        elif result == "1/2-1/2":
            score = 0.5
        else:
            continue # Salta partite non finite o interrotte

        board = game.board()
        
        # Scorriamo tutte le mosse della partita
        for move in game.mainline_moves():
            board.push(move)
            
            # Non salviamo posizioni di scacco matto (non c'è mossa successiva)
            if board.is_game_over():
                break

            # Scriviamo: FEN | SCORE
            # Esempio: rnbqkbnr/pppppppp... | 1.0
            output.write(f"{board.fen()} | {score}\n")
            positions_count += 1

        if games_count % 100 == 0:
            print(f"Processate {games_count} partite... ({positions_count} posizioni estratte)")

    print(f"\n--- FINITO ---")
    print(f"Totale Partite: {games_count}")
    print(f"Totale Posizioni: {positions_count}")
    print(f"File salvato in: {OUTPUT_TXT}")
    
    pgn.close()
    output.close()

if __name__ == "__main__":
    parse_pgn()