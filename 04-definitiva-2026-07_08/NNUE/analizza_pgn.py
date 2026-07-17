"""
Analizza un PGN per capire se e' adatto ad allenare la NNUE.

Scansiona (in streaming, senza caricare tutto in RAM) le prime N partite e riporta:
  * quante partite / quante mosse (posizioni di training stimate);
  * distribuzione dei risultati (1-0 / 0-1 / 1/2-1/2);
  * quante partite hanno le valutazioni per-posizione [%eval ...] (fondamentali!);
  * lunghezza media delle partite e copertura dei finali.

Uso:
  python analizza_pgn.py partite.pgn
  python analizza_pgn.py partite.pgn --games 20000
"""
import argparse
import chess
import chess.pgn


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("pgn")
    ap.add_argument("--games", type=int, default=10000,
                    help="numero di partite da campionare (default 10000)")
    args = ap.parse_args()

    n_games = 0
    n_positions = 0
    results = {"1-0": 0, "0-1": 0, "1/2-1/2": 0, "*": 0}
    games_with_eval = 0
    positions_with_eval = 0
    endgame_positions = 0  # posizioni con <= 6 pezzi totali

    with open(args.pgn, "r", errors="ignore") as fh:
        while n_games < args.games:
            try:
                game = chess.pgn.read_game(fh)
            except Exception:
                continue
            if game is None:
                break
            n_games += 1
            results[game.headers.get("Result", "*")] = \
                results.get(game.headers.get("Result", "*"), 0) + 1

            board = game.board()
            node = game
            game_has_eval = False
            while node.variations:
                node = node.variation(0)
                board.push(node.move)
                n_positions += 1
                try:
                    has_eval = node.eval() is not None
                except Exception:
                    has_eval = False
                if has_eval:
                    positions_with_eval += 1
                    game_has_eval = True
                if chess.popcount(board.occupied) <= 6:
                    endgame_positions += 1
            if game_has_eval:
                games_with_eval += 1

    if n_games == 0:
        print("❌ Nessuna partita letta: il file non e' un PGN valido?")
        return

    print(f"\n=== ANALISI: {args.pgn} (campione: {n_games} partite) ===")
    print(f"Posizioni di training stimate: {n_positions:,} (~{n_positions // n_games} per partita)")
    print("\nRisultati:")
    for k in ("1-0", "0-1", "1/2-1/2", "*"):
        pct = 100 * results.get(k, 0) / n_games
        print(f"  {k:8s}: {results.get(k,0):7d}  ({pct:5.1f}%)")

    eval_game_pct = 100 * games_with_eval / n_games
    eval_pos_pct = 100 * positions_with_eval / max(n_positions, 1)
    end_pct = 100 * endgame_positions / max(n_positions, 1)
    print(f"\nValutazioni [%eval]: {eval_game_pct:.1f}% delle partite, "
          f"{eval_pos_pct:.1f}% delle posizioni")
    print(f"Finali (<=6 pezzi): {end_pct:.1f}% delle posizioni")

    print("\n=== GIUDIZIO ===")
    ok = True
    if n_positions < 1_000_000 and n_games >= args.games:
        print("• Dataset probabilmente PICCOLO: punta ad almeno alcuni milioni di posizioni.")
        ok = False
    if eval_pos_pct < 50:
        print("• POCHE/NESSUNA [%eval]: le etichette saranno solo WDL (segnale debole).")
        print("  -> preferisci un PGN annotato con le valutazioni del motore.")
        ok = False
    else:
        print("• Buona copertura di [%eval]: puoi usare etichette per-posizione (molto meglio).")
    draw_pct = 100 * results.get("1/2-1/2", 0) / n_games
    if draw_pct > 70:
        print(f"• Moltissime patte ({draw_pct:.0f}%): rischio segnale poco informativo.")
    if end_pct < 3:
        print("• Pochi finali: la rete rischia di restare debole nei finali (come ora).")
    if ok:
        print("• Il file sembra adatto. Procedi con l'allenamento e valida con verifica_nnue.py.")


if __name__ == "__main__":
    main()
