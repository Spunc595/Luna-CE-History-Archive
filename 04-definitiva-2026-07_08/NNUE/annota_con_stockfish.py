"""
Annota un PGN con le valutazioni di Stockfish (commenti [%eval ...]),
producendo un file pronto per allena_luna.py (etichette per-posizione).

Non serve annotare tutto: bastano 1-3 milioni di posizioni per una rete
768->256->32->1. Usa --max-positions per fissare il budget e --step per
sottocampionare (annota 1 posizione ogni N mezze-mosse).

Uso (Windows):
  python annota_con_stockfish.py --pgn training_data.pgn --out training_eval.pgn ^
      --stockfish stockfish-windows-x86-64-avx2.exe --depth 8 --max-positions 2000000

Dipendenze: python-chess. Serve l'eseguibile di Stockfish.
"""
import argparse
import os
import time

import chess
import chess.pgn
import chess.engine


def main():
    ap = argparse.ArgumentParser(description="Annota un PGN con [%eval] di Stockfish")
    ap.add_argument("--pgn", required=True, help="PGN di input")
    ap.add_argument("--out", default="training_eval.pgn", help="PGN annotato in uscita")
    ap.add_argument("--stockfish", required=True, help="percorso dell'eseguibile Stockfish")
    ap.add_argument("--depth", type=int, default=8, help="profondità di analisi per posizione")
    ap.add_argument("--movetime", type=float, default=None,
                    help="tempo per posizione in secondi (alternativa a --depth)")
    ap.add_argument("--max-positions", type=int, default=2_000_000,
                    help="budget totale di posizioni da annotare")
    ap.add_argument("--step", type=int, default=1,
                    help="annota 1 posizione ogni STEP mezze-mosse (sottocampionamento)")
    ap.add_argument("--skip-opening", type=int, default=4,
                    help="salta le prime N mezze-mosse (aperture poco informative)")
    ap.add_argument("--threads", type=int, default=1, help="Thread di Stockfish")
    ap.add_argument("--hash", type=int, default=128, help="Hash di Stockfish (MB)")
    args = ap.parse_args()

    if not os.path.exists(args.pgn):
        print(f"❌ {args.pgn} non trovato")
        return
    if not os.path.exists(args.stockfish):
        print(f"❌ Stockfish non trovato: {args.stockfish}")
        return

    limit = (chess.engine.Limit(time=args.movetime) if args.movetime
             else chess.engine.Limit(depth=args.depth))

    engine = chess.engine.SimpleEngine.popen_uci(args.stockfish)
    try:
        engine.configure({"Threads": args.threads, "Hash": args.hash})
    except Exception as e:
        print(f"⚠️  configure ignorato: {e}")

    annotated = 0
    n_games = 0
    t0 = time.time()

    with open(args.pgn, "r", errors="ignore") as fin, \
         open(args.out, "w", encoding="utf-8") as fout:
        while annotated < args.max_positions:
            try:
                game = chess.pgn.read_game(fin)
            except Exception:
                continue
            if game is None:
                break
            n_games += 1

            board = game.board()
            node = game
            ply = 0
            while node.variations:
                node = node.variation(0)
                board.push(node.move)
                ply += 1
                if board.is_game_over():
                    break
                if ply <= args.skip_opening or (ply % args.step) != 0:
                    continue
                if annotated >= args.max_positions:
                    break
                try:
                    info = engine.analyse(board, limit)
                    node.set_eval(info["score"], info.get("depth"))
                    annotated += 1
                except Exception as e:
                    print(f"⚠️  analisi saltata: {e}")

            print(game, file=fout, end="\n\n")

            if n_games % 100 == 0:
                dt = time.time() - t0
                rate = annotated / dt if dt > 0 else 0
                print(f"  partite={n_games}  posizioni annotate={annotated:,}  "
                      f"({rate:.0f}/s)")
    engine.quit()

    dt = time.time() - t0
    print(f"\n✅ Fatto: {annotated:,} posizioni annotate da {n_games} partite in {dt/60:.1f} min")
    print(f"   Output: {args.out}")
    print("   Ora allena con:  python allena_luna.py --pgn " + args.out + " --eval-lambda 0.7")


if __name__ == "__main__":
    main()
