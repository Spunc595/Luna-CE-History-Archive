import sys
import os
import torch
import lightning as L
from torch.utils.data import IterableDataset, DataLoader
import chess.pgn
import traceback
from lightning.pytorch.callbacks import ModelCheckpoint

# Percorsi
sys.path.append(os.getcwd())
import model as M

class PGNDataset(IterableDataset):
    def __init__(self, pgn_file, feature_set):
        self.pgn_file = pgn_file
        self.feature_set = feature_set
        self.target_dim = 512 

    def __iter__(self):
        try:
            with open(self.pgn_file, "r") as pgn:
                while True:
                    game = chess.pgn.read_game(pgn)
                    if game is None: break
                    
                    res = game.headers.get("Result", "1/2")
                    score_val = 1.0 if res == "1-0" else (0.0 if res == "0-1" else 0.5)
                    target = torch.tensor([score_val], dtype=torch.float32)

                    board = game.board()
                    for move in game.mainline_moves():
                        board.push(move)
                        if not board.is_game_over():
                            is_white = board.turn == chess.WHITE
                            us = torch.tensor([1.0 if is_white else 0.0], dtype=torch.float32)
                            them = torch.tensor([0.0 if is_white else 1.0], dtype=torch.float32)

                            active = self.feature_set.get_active_features(board)
                            f_indices = active[0] if isinstance(active, (list, tuple)) else active
                            
                            idx_t = torch.full((self.target_dim,), -1, dtype=torch.int32)
                            val_t = torch.zeros(self.target_dim, dtype=torch.float32)
                            
                            real_idx = torch.as_tensor(f_indices, dtype=torch.int32)
                            n = min(len(real_idx), self.target_dim)
                            idx_t[:n] = real_idx[:n]
                            val_t[:n] = 1.0

                            # Indici puri
                            p_idx = torch.tensor(0, dtype=torch.long)
                            l_idx = torch.tensor(0, dtype=torch.long)

                            # --- ESATTAMENTE 10 ELEMENTI ---
                            yield (
                                us, them, 
                                idx_t, val_t, 
                                idx_t, val_t, 
                                p_idx, l_idx, 
                                target, p_idx
                            )
        except Exception:
            print(traceback.format_exc())

if __name__ == "__main__":
    print("--- LUNA NNUE: SINCRONIZZAZIONE GLOBALE ---")
    
    fs = M.get_feature_set_from_name("HalfKA")
    # Nota: Assicurati che model.py sia salvato come NNUE o NNUEModel
    model = M.NNUE(feature_set=fs, config=M.ModelConfig(L1=256), quantize_config=M.quantize.QuantizationConfig())

    dataset = PGNDataset("training_data.pgn", fs)
    train_loader = DataLoader(dataset, batch_size=16, num_workers=0)

    checkpoint_callback = ModelCheckpoint(
        dirpath='checkpoints/',
        filename='luna-{step:07d}',
        every_n_train_steps=5000,
        save_top_k=-1
    )

    trainer = L.Trainer(
        accelerator="cpu",
        max_epochs=800,
        callbacks=[checkpoint_callback],
        precision="32"
    )

    print("\n🚀 Daniele, ci siamo. L'allineamento è perfetto. Luna, vai!")
    try:
        trainer.fit(model, train_loader)
    except Exception:
        print("\n--- ANALISI ERRORE ---")
        print(traceback.format_exc())