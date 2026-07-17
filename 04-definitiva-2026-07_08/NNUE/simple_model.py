import torch
import torch.nn as nn
import lightning as L
import chess

# --- CONFIGURAZIONI ---
class ModelConfig:
    def __init__(self, L1=256):
        self.L1 = L1
        self.INPUT_SIZE = 768 # 64 caselle * 12 pezzi

class QuantizationConfig:
    def __init__(self):
        self.qa = 255
        self.qb = 64

# --- FEATURE SET SEMPLIFICATO (768) ---
class SimpleFeatureSet:
    def __init__(self):
        self.num_features = 768
        self.piece_map = {
            chess.PAWN: 0, chess.KNIGHT: 1, chess.BISHOP: 2, 
            chess.ROOK: 3, chess.QUEEN: 4, chess.KING: 5
        }

    def get_active_features(self, board):
        w_features = []
        b_features = [] 
        
        for sq, piece in board.piece_map().items():
            # --- PROSPETTIVA BIANCO ---
            pt_w = self.piece_map[piece.piece_type]
            if piece.color == chess.BLACK:
                pt_w += 6 
            idx_w = pt_w * 64 + sq
            w_features.append(idx_w)
            
        return w_features, b_features

def get_feature_set_from_name(name):
    return SimpleFeatureSet()

# --- MODELLO NEURALE ---
class NNUE(L.LightningModule):
    def __init__(self, feature_set, config, quantize_config):
        super().__init__()
        self.save_hyperparameters(ignore=['feature_set'])
        self.feature_set = feature_set
        self.config = config
        
        self.input = nn.Linear(config.INPUT_SIZE, config.L1)
        self.layer_stacks = nn.Sequential(
            nn.Linear(config.L1, 32),
            nn.ReLU(),
            nn.Linear(32, 1)
        )

    def forward(self, idx_w, val_w):
        batch_size = idx_w.shape[0]
        w_acc = torch.zeros(batch_size, self.config.L1, device=self.device)
        mask = (idx_w != -1)
        weight_matrix = self.input.weight.t() 
        
        for b in range(batch_size):
            valid_indices = idx_w[b][mask[b]].long()
            if len(valid_indices) > 0:
                w_acc[b] = torch.sum(weight_matrix[valid_indices], dim=0)

        w_acc = torch.clamp(w_acc, 0.0, 1.0)
        return self.layer_stacks(w_acc)

    def training_step(self, batch, batch_idx):
        # --- MODIFICA QUI: Riceviamo solo 3 tensori puliti ---
        idx_w, val_w, target = batch
        
        pred = self(idx_w, val_w)
        loss = nn.MSELoss()(pred, target)
        self.log("train_loss", loss, prog_bar=True)
        return loss

    def configure_optimizers(self):
        return torch.optim.Adam(self.parameters(), lr=1e-3)