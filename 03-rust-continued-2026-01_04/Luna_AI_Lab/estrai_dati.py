import chess.pgn

# CONFIGURAZIONE
INPUT_PGN = "dati_training_luna.pgn"      # Il file appena creato da Cutechess
OUTPUT_DATASET = "dataset_completo.txt"   # Il tuo file con le 26.000 righe (cambia nome se serve)

def process_pgn():
    print(f"Apro {INPUT_PGN} e aggiungo i dati a {OUTPUT_DATASET}...")
    
    games_count = 0
    positions_count = 0
    
    # Apre il file di output in modalità 'a' (APPEND) per non cancellare i vecchi dati
    with open(OUTPUT_DATASET, "a") as out_file:
        with open(INPUT_PGN) as pgn_file:
            while True:
                game = chess.pgn.read_game(pgn_file)
                if game is None:
                    break # Fine del file

                games_count += 1
                board = game.board()
                result = game.headers["Result"]
                
                # Convertiamo il risultato in valore per la rete: 1.0 (Vince B), 0.0 (Vince N), 0.5 (Patta)
                if result == "1-0": score = 1.0
                elif result == "0-1": score = 0.0
                elif result == "1/2-1/2": score = 0.5
                else: continue # Ignora partite non finite

                # Ripercorriamo la partita mossa per mossa
                for move in game.mainline_moves():
                    board.push(move)
                    
                    # FILTRI DI QUALITÀ (Importante per una rete forte!)
                    # 1. Saltiamo le prime 8 mosse (apertura troppo standard)
                    if board.fullmove_number < 9: continue
                    
                    # 2. Saltiamo se c'è scacco (posizioni troppo instabili/tattiche)
                    if board.is_check(): continue
                    
                    # Scriviamo la riga: FEN | SCORE
                    # Esempio formato: rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1 | 1.0
                    out_file.write(f"{board.fen()} | {score}\n")
                    positions_count += 1

    print(f"Finito! Processate {games_count} partite.")
    print(f"Aggiunte {positions_count} nuove posizioni al dataset.")

if __name__ == "__main__":
    process_pgn()