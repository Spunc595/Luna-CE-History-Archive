"""
Verifica di una rete Luna NNUE.

Fa due cose:
  1) Riproduce ESATTAMENTE l'inferenza intera del motore (src/nnue.rs) leggendo
     il file `luna.nnue`, e stampa la valutazione in centipedine su un set di FEN.
  2) (Opzionale, se passi --ckpt) confronta l'inferenza intera con il modello
     PyTorch float dequantizzato, per validare che export + quantizzazione siano
     coerenti col training (differenza attesa < ~0.01).

Dipendenze: numpy, python-chess (obbligatorie); torch + simple_model (solo per --ckpt).

Esempi:
  python verifica_nnue.py --nnue luna.nnue
  python verifica_nnue.py --nnue luna.nnue --ckpt checkpoints/luna-simple-step=200000.ckpt
"""
import argparse
import os

import numpy as np
import chess

try:
    import torch
    import simple_model as M
    _HAS_TORCH = True
except Exception:
    _HAS_TORCH = False

# --- Costanti: DEVONO combaciare con src/nnue.rs ---
INPUT_SIZE = 768
L1_SIZE = 256
L2_SIZE = 32
QA = 255
QB = 64
QAB = QA * QB
EVAL_SCALE = 2400  # stessa costante di nnue.rs (tunabile)

PIECE_MAP = {
    chess.PAWN: 0, chess.KNIGHT: 1, chess.BISHOP: 2,
    chess.ROOK: 3, chess.QUEEN: 4, chess.KING: 5,
}

# Set di FEN con asimmetrie di materiale note (prospettiva Bianco al tratto).
DEFAULT_FENS = [
    ("start",   chess.STARTING_FEN),
    ("W+Donna", "4k3/8/8/8/8/8/8/Q3K3 w - - 0 1"),
    ("N+Donna", "q3k3/8/8/8/8/8/8/4K3 w - - 0 1"),
    ("W+Torre", "4k3/8/8/8/8/8/8/R3K3 w - - 0 1"),
    ("W+2Donne","4k3/8/8/8/8/8/8/Q2QK3 w - - 0 1"),
    ("W-Cavallo","rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/1NBQKBNR w - - 0 1"),
]


def active_features(board):
    """Indici feature attive (prospettiva Bianco): pt*64+sq, Nero += 6*64."""
    idx = []
    for sq, piece in board.piece_map().items():
        pt = PIECE_MAP[piece.piece_type]
        if piece.color == chess.BLACK:
            pt += 6
        idx.append(pt * 64 + sq)
    return idx


class IntNet:
    """Replica dell'inferenza intera di src/nnue.rs a partire dal file .nnue."""

    def __init__(self, path):
        raw = np.fromfile(path, dtype="<i2")
        expected = INPUT_SIZE * L1_SIZE + L1_SIZE + L1_SIZE * L2_SIZE + L2_SIZE + L2_SIZE + 1
        if raw.size != expected:
            raise ValueError(f"{path}: {raw.size} i16 letti, attesi {expected} (file corrotto/troncato)")
        a = raw.astype(np.int64)
        o = 0

        def take(n):
            nonlocal o
            v = a[o:o + n]
            o += n
            return v

        # Layout del file (come da esporta_muscolosa.py): pesi in [out, in].
        self.l1w = take(INPUT_SIZE * L1_SIZE).reshape(L1_SIZE, INPUT_SIZE)  # [out=256][in=768]
        self.l1b = take(L1_SIZE)
        self.l2w = take(L1_SIZE * L2_SIZE).reshape(L2_SIZE, L1_SIZE)        # [out=32][in=256]
        self.l2b = take(L2_SIZE)
        self.l3w = take(L2_SIZE)
        self.l3b = take(1)

    def raw_output(self, board):
        """out_full a scala QA*QB, prospettiva del Bianco (prima del segno lato-al-tratto)."""
        acc = self.l1b.copy()
        for f in active_features(board):
            acc += self.l1w[:, f]
        h1 = np.clip(acc, 0, QA)                         # clipped ReLU [0, QA]
        acc2 = self.l2b * QA + self.l2w @ h1            # scala QA*QB
        h2 = np.maximum(acc2 // QB, 0)                  # nn.ReLU() (nessun tetto)
        return int(self.l3b[0] * QA + int(self.l3w @ h2))

    def eval_cp(self, board):
        out_full = self.raw_output(board)
        # Troncamento verso zero come l'aritmetica intera di Rust.
        score = int((out_full - QAB // 2) * EVAL_SCALE / QAB)
        return score if board.turn == chess.WHITE else -score

    def winprob(self, board):
        return self.raw_output(board) / QAB


class FloatModel:
    """Modello PyTorch float dequantizzato dagli stessi interi del file .nnue-equivalente."""

    def __init__(self, ckpt_path):
        checkpoint = torch.load(ckpt_path, map_location="cpu", weights_only=False)
        sd = checkpoint["state_dict"]
        self.W1 = sd["input.weight"].detach().cpu().numpy()          # [256,768]
        self.B1 = sd["input.bias"].detach().cpu().numpy()
        self.W2 = sd["layer_stacks.0.weight"].detach().cpu().numpy() # [32,256]
        self.B2 = sd["layer_stacks.0.bias"].detach().cpu().numpy()
        self.W3 = sd["layer_stacks.2.weight"].detach().cpu().numpy() # [1,32]
        self.B3 = sd["layer_stacks.2.bias"].detach().cpu().numpy()

    def winprob(self, board):
        x = np.zeros(INPUT_SIZE)
        for f in active_features(board):
            x[f] = 1.0
        w_acc = np.clip(self.W1 @ x + self.B1, 0.0, 1.0)   # torch.clamp(...,0,1)
        h2 = np.maximum(self.W2 @ w_acc + self.B2, 0.0)    # nn.ReLU()
        return float(self.W3 @ h2) + float(self.B3[0])


def main():
    ap = argparse.ArgumentParser(description="Verifica una rete Luna NNUE")
    ap.add_argument("--nnue", default="luna.nnue", help="file .nnue da verificare")
    ap.add_argument("--ckpt", default=None, help="checkpoint .ckpt per il confronto float (opzionale)")
    ap.add_argument("--fens", default=None, help="file con un FEN per riga (opzionale)")
    args = ap.parse_args()

    if not os.path.exists(args.nnue):
        print(f"❌ {args.nnue} non trovato")
        return

    net = IntNet(args.nnue)
    print(f"✅ Caricato {args.nnue}")

    fens = DEFAULT_FENS
    if args.fens:
        with open(args.fens) as fh:
            fens = [(f"fen{i}", line.strip()) for i, line in enumerate(fh) if line.strip()]

    fmodel = None
    if args.ckpt:
        if not _HAS_TORCH:
            print("⚠️  torch/simple_model non disponibili: salto il confronto col checkpoint.")
        elif not os.path.exists(args.ckpt):
            print(f"⚠️  {args.ckpt} non trovato: salto il confronto col checkpoint.")
        else:
            fmodel = FloatModel(args.ckpt)
            print(f"✅ Caricato modello float da {args.ckpt}")

    header = f"{'posizione':12s} {'winprob':>9s} {'cp(STM)':>8s}"
    if fmodel is not None:
        header += f" {'model_wp':>9s} {'|diff|':>8s}"
    print("\n" + header)
    print("-" * len(header))

    max_diff = 0.0
    for name, fen in fens:
        board = chess.Board(fen)
        wp = net.winprob(board)
        cp = net.eval_cp(board)
        row = f"{name:12s} {wp:9.4f} {cp:+8d}"
        if fmodel is not None:
            mwp = fmodel.winprob(board)
            d = abs(wp - mwp)
            max_diff = max(max_diff, d)
            row += f" {mwp:9.4f} {d:8.5f}"
        print(row)

    if fmodel is not None:
        ok = max_diff < 0.01
        print(f"\n{'✅' if ok else '❌'} differenza massima int-vs-float = {max_diff:.5f} "
              f"({'coerente' if ok else 'INCOERENTE: controlla scale/layout export'})")

    # Sanity check base: la valutazione deve VARIARE col materiale.
    probs = [net.winprob(chess.Board(f)) for _, f in fens]
    if max(probs) - min(probs) < 1e-4:
        print("❌ ATTENZIONE: la valutazione e' PIATTA (non varia col materiale).")
    else:
        print("✅ La valutazione varia con la posizione.")


if __name__ == "__main__":
    main()
