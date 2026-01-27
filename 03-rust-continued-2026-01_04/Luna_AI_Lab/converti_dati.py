import chess.pgn

# --- CONFIGURAZIONE ---
INPUT_PGN = "dati_maestro_stockfish.pgn"      # Il file generato da Cutechess
OUTPUT_DATASET = "dataset_2500_elo.txt"   # File di output pulito

def converti_pgn_in_dataset():
    print(f"Leggo le partite da {INPUT_PGN}...")
    count = 0
    games_processed = 0
    
    with open(OUTPUT_DATASET, "w") as out_file:
        with open(INPUT_PGN) as pgn_file:
            while True:
                game = chess.pgn.read_game(pgn_file)
                if game is None: break
                
                games_processed += 1

                # 1. PRENDIAMO IL RISULTATO REALE (La "Verità")
                res_str = game.headers.get("Result", "*")
                if res_str == "1-0": score = 1.0
                elif res_str == "0-1": score = 0.0
                elif res_str == "1/2-1/2": score = 0.5
                else: continue # Partita interrotta o nulla, saltiamo

                board = game.board()
                
                # 2. RIPERCORRIAMO LA PARTITA
                for move in game.mainline_moves():
                    board.push(move)
                    
                    # 3. FILTRI DI QUALITÀ
                    # Saltiamo l'apertura (prime 8 mosse) e le posizioni di scacco (troppo instabili)
                    if board.fullmove_number > 8 and not board.is_check():
                        
                        # FORMATO PERFETTO: FEN | SCORE REALE
                        # La rete imparerà: "Questa posizione porta alla vittoria (1.0)"
                        out_file.write(f"{board.fen()} | {score}\n")
                        count += 1
                        
                if games_processed % 100 == 0:
                    print(f"Processate {games_processed} partite... ({count} posizioni salvate)")

    print(f"Finito! Salvate {count} posizioni pronte per il training.")

if __name__ == "__main__":
    converti_pgn_in_dataset()