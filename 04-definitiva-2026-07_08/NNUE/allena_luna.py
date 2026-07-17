import sys
import os
import math
import random
import argparse

import torch
import lightning as L
from torch.utils.data import IterableDataset, DataLoader
import chess
import chess.pgn
from lightning.pytorch.callbacks import ModelCheckpoint

# Assicuriamoci che la cartella corrente sia nel path per importare simple_model
sys.path.append(os.getcwd())
import simple_model as M

# Diciamo a PyTorch che questi oggetti sono sicuri da caricare (Fix per PyTorch 2.x)
torch.serialization.add_safe_globals([M.ModelConfig, M.QuantizationConfig, M.NNUE, M.SimpleFeatureSet])

# --- CONFIGURAZIONI ---
BATCH_SIZE = 2048
LEARNING_RATE = 1e-3          # (usato da simple_model.configure_optimizers)
TARGET_DIM = 32               # massimo numero di feature attive (<= 32 pezzi sulla scacchiera)

# Buffer di shuffle: le posizioni di una stessa partita sono molto correlate.
# Mescolarle su una finestra ampia rende l'addestramento molto piu' stabile.
SHUFFLE_BUFFER = 65536

# Miscelazione delle etichette:
#   target = EVAL_LAMBDA * winprob(eval_posizione) + (1 - EVAL_LAMBDA) * risultato_partita
# EVAL_LAMBDA=0.0  -> solo risultato partita (WDL), come la versione originale.
# EVAL_LAMBDA=1.0  -> solo valutazione per-posizione (richiede PGN con commenti [%eval ...]).
# Se una posizione NON ha [%eval], si ricade automaticamente sul solo WDL.
EVAL_LAMBDA = 0.7
# Conversione centipedine -> probabilita' di vittoria: sigmoid(cp / EVAL_SCALING).
EVAL_SCALING = 410.0


def wdl_from_result(res):
    """1.0 = vittoria Bianco, 0.0 = vittoria Nero, 0.5 = patta."""
    if res == "1-0":
        return 1.0
    if res == "0-1":
        return 0.0
    return 0.5


def cp_to_winprob(cp):
    return 1.0 / (1.0 + math.exp(-cp / EVAL_SCALING))


class PGNDataset(IterableDataset):
    """
    Genera campioni (idx_features, val_features, target) da un file PGN.

    Miglioramenti rispetto alla versione originale:
      * etichetta per-posizione da [%eval ...] miscelata col risultato partita;
      * buffer di shuffle per decorrelare le posizioni consecutive;
      * lettura robusta (una partita corrotta non interrompe il training).
    """

    def __init__(self, pgn_file, feature_set, eval_lambda=EVAL_LAMBDA, shuffle_buffer=SHUFFLE_BUFFER):
        self.pgn_file = pgn_file
        self.feature_set = feature_set
        self.eval_lambda = eval_lambda
        self.shuffle_buffer = shuffle_buffer

    def prepara_tenser(self, indices):
        idx_t = torch.full((TARGET_DIM,), -1, dtype=torch.int32)
        real_idx = torch.as_tensor(indices, dtype=torch.int32)
        n = min(len(real_idx), TARGET_DIM)
        idx_t[:n] = real_idx[:n]
        return idx_t, torch.ones(TARGET_DIM)

    def _target_for(self, node, board, wdl):
        """Etichetta per la posizione corrente (prospettiva del Bianco)."""
        if self.eval_lambda <= 0.0:
            return wdl
        pov = None
        try:
            pov = node.eval()  # legge il commento [%eval ...] se presente
        except Exception:
            pov = None
        if pov is None:
            return wdl  # nessuna eval per questa posizione: ricado sul WDL
        score = pov.white()
        if score.is_mate():
            wp = 1.0 if score.mate() > 0 else 0.0
        else:
            wp = cp_to_winprob(score.score())
        return self.eval_lambda * wp + (1.0 - self.eval_lambda) * wdl

    def _positions(self):
        with open(self.pgn_file, "r", errors="ignore") as pgn:
            while True:
                try:
                    game = chess.pgn.read_game(pgn)
                except Exception as e:
                    print(f"Partita saltata (errore di parsing): {e}")
                    continue
                if game is None:
                    break

                wdl = wdl_from_result(game.headers.get("Result", "1/2"))
                board = game.board()
                node = game
                while node.variations:
                    node = node.variation(0)
                    board.push(node.move)
                    if board.is_game_over():
                        break

                    target = self._target_for(node, board, wdl)
                    active = self.feature_set.get_active_features(board)
                    idx_w, val_w = self.prepara_tenser(active[0])
                    yield (idx_w, val_w, torch.tensor([target], dtype=torch.float32))

    def __iter__(self):
        # Reservoir-style shuffle buffer su flusso infinito/lungo.
        buf = []
        for item in self._positions():
            if len(buf) < self.shuffle_buffer:
                buf.append(item)
            else:
                j = random.randrange(self.shuffle_buffer)
                yield buf[j]
                buf[j] = item
        random.shuffle(buf)
        for item in buf:
            yield item


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Training rapido Luna SIMPLE-768")
    parser.add_argument("--pgn", default="training_data.pgn", help="file PGN di training")
    parser.add_argument("--resume", default=None, help="checkpoint da cui riprendere (es. checkpoints/luna-simple-step=197000.ckpt)")
    parser.add_argument("--epochs", type=int, default=10)
    parser.add_argument("--eval-lambda", type=float, default=EVAL_LAMBDA,
                        help="peso delle etichette da [%%eval] rispetto al risultato partita (0..1)")
    args = parser.parse_args()

    print("🚀 LUNA SIMPLE-768: TRAINING")
    print(f"   PGN={args.pgn}  epochs={args.epochs}  eval_lambda={args.eval_lambda}  shuffle_buffer={SHUFFLE_BUFFER}")
    
    # Calcola num_workers prima di stampar le info
    num_workers = min(4, os.cpu_count() or 1)
    print(f"   num_workers={num_workers}  (CPU cores={os.cpu_count()})")

    fs = M.SimpleFeatureSet()
    model = M.NNUE(fs, M.ModelConfig(), M.QuantizationConfig())

    dataset = PGNDataset(args.pgn, fs, eval_lambda=args.eval_lambda)
    loader = DataLoader(dataset, batch_size=BATCH_SIZE, num_workers=num_workers)

    checkpoint_callback = ModelCheckpoint(
        dirpath="checkpoints/",
        filename="luna-simple-{step:05d}",
        every_n_train_steps=500,
        save_top_k=-1,
    )

    trainer = L.Trainer(
        max_epochs=args.epochs,
        callbacks=[checkpoint_callback],
        log_every_n_steps=10,
    )

    # --- RESUME OPZIONALE ---
    # In Lightning, ckpt_path ripristina PESI + OTTIMIZZATORE + STEP in automatico.
    if args.resume and os.path.exists(args.resume):
        print(f"✅ Checkpoint trovato! Riprendo da: {args.resume}")
        trainer.fit(model, train_dataloaders=loader, ckpt_path=args.resume)
    else:
        if args.resume:
            print(f"⚠️ ATTENZIONE: '{args.resume}' non trovato. Parto da zero!")
        trainer.fit(model, train_dataloaders=loader)
