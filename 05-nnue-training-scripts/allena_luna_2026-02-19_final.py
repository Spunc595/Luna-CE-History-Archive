import sys
import os
import torch
import lightning as L
from torch.utils.data import IterableDataset, DataLoader
import chess.pgn
from lightning.pytorch.callbacks import ModelCheckpoint

# Assicuriamoci che la cartella corrente sia nel path per importare simple_model
sys.path.append(os.getcwd())
import simple_model as M

# Diciamo a PyTorch che questi oggetti sono sicuri da caricare (Fix per PyTorch 2.x)
torch.serialization.add_safe_globals([M.ModelConfig, M.QuantizationConfig, M.NNUE, M.SimpleFeatureSet])

# --- CONFIGURAZIONI ---
BATCH_SIZE = 2048 
LEARNING_RATE = 1e-3

class PGNDataset(IterableDataset):
    def __init__(self, pgn_file, feature_set):
        self.pgn_file = pgn_file
        self.feature_set = feature_set
        self.target_dim = 32 

    def prepara_tenser(self, indices):
        idx_t = torch.full((self.target_dim,), -1, dtype=torch.int32)
        real_idx = torch.as_tensor(indices, dtype=torch.int32)
        n = min(len(real_idx), self.target_dim)
        idx_t[:n] = real_idx[:n]
        return idx_t, torch.ones(self.target_dim) 

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
                            active = self.feature_set.get_active_features(board)
                            idx_w, val_w = self.prepara_tenser(active[0])
                            
                            # Generatore pulito e solido
                            yield (idx_w, val_w, target)
        except Exception as e:
            # Utile per non far morire il programma in silenzio in caso di PGN corrotto
            print(f"Fine lettura PGN o errore: {e}")
            pass

if __name__ == "__main__":
    print("🚀 LUNA SIMPLE-768: TRAINING RAPIDO")
    
    fs = M.SimpleFeatureSet()
    model = M.NNUE(fs, M.ModelConfig(), M.QuantizationConfig())
    
    dataset = PGNDataset("training_data.pgn", fs)
    loader = DataLoader(dataset, batch_size=BATCH_SIZE, num_workers=0)

    checkpoint_callback = ModelCheckpoint(
        dirpath='checkpoints/',
        filename='luna-simple-{step:05d}',
        every_n_train_steps=500, 
        save_top_k=-1
    )

    trainer = L.Trainer(
        max_epochs=10, 
        callbacks=[checkpoint_callback], 
        log_every_n_steps=10
    )
    
    # --- LA MAGIA DEL RESUME ---
    ckpt_file = "checkpoints/luna-simple-step=197000.ckpt"
    
    # Controlliamo che il file esista davvero, altrimenti ti avvisa!
    if os.path.exists(ckpt_file):
        print(f"✅ Checkpoint trovato! Riprendo l'addestramento da: {ckpt_file}")
        # In Lightning, ckpt_path ripristina PESI + OTTIMIZZATORE + STEP in automatico
        trainer.fit(model, train_dataloaders=loader, ckpt_path=ckpt_file)
    else:
        print(f"⚠️ ATTENZIONE: Checkpoint non trovato in '{ckpt_file}'. Parto da zero!")
        trainer.fit(model, train_dataloaders=loader)