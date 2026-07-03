import chess
import chess.pgn
import random
import os

# Cambia il percorso con quello assoluto per sicurezza
PGN_FILE = r"C:\Users\danie\Desktop\DEFINITIVA\tuner\lichess_db_standard_rated_2013-11.pgn"
OUTPUT_FILE = "training_data.txt"

def process_pgn():
    if not os.path.exists(PGN_FILE):
        print(f"ERRORE: File {PGN_FILE} non trovato!")
        return

    with open(PGN_FILE, encoding="utf-8") as pgn, open(OUTPUT_FILE, "w") as f:
        print("Inizio estrazione...")
        game_count = 0
        written_count = 0
        
        while True:
            game = chess.pgn.read_game(pgn)
            if game is None: break
            
            game_count += 1
            if game_count % 5000 == 0:
                print(f"Letta partita {game_count}, scritte finora: {written_count}")

            # Prendi il risultato
            res = game.headers.get("Result")
            if res not in ["1-0", "0-1", "1/2-1/2"]: continue
            score = 1.0 if res == "1-0" else (0.0 if res == "0-1" else 0.5)

            board = game.board()
            # Riduciamo il filtro: prendiamo posizioni dopo la mossa 10 (non 30)
            for i, move in enumerate(game.mainline_moves()):
                board.push(move)
                
                if i > 10: 
                    # Togliamo il filtro is_quiet per ora per vedere se scrive qualcosa
                    fen = board.fen()
                    f.write(f"{fen} | {score}\n")
                    written_count += 1
                    
                    if written_count > 450000: # Limite per prova
                        print("Raggiunte 450.000 posizioni, mi fermo.")
                        return

    print(f"Completato! Scritte {written_count} posizioni.")

if __name__ == "__main__":
    process_pgn()