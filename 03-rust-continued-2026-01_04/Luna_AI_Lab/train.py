import torch
import torch.nn as nn
import torch.optim as optim
from torch.utils.data import Dataset, DataLoader
import numpy as np
import struct
import os

# --- CONFIGURAZIONE ---
BATCH_SIZE = 8192      # Più grande per velocità
LEARNING_RATE = 0.001
EPOCHS = 10            # Bastano poche epoche per vedere se funziona
DATA_FILE = "dataset_completo.txt" 
MODEL_SAVE_PATH = "luna_quantized.nnue"
INPUT_SIZE = 768
HIDDEN_SIZE = 256
SCALE = 255.0          # Fattore di quantizzazione

# Mappa pezzi standard
PIECE_MAP = {
    'P': 0, 'N': 1, 'B': 2, 'R': 3, 'Q': 4, 'K': 5,
    'p': 6, 'n': 7, 'b': 8, 'r': 9, 'q': 10, 'k': 11
}

class LunaModel(nn.Module):
    def __init__(self):
        super().__init__()
        # Simple-768 Architecture
        self.input = nn.Linear(INPUT_SIZE, HIDDEN_SIZE)
        self.activ = nn.ReLU() # Clipped ReLU in inference
        self.output = nn.Linear(HIDDEN_SIZE, 1)

    def forward(self, x):
        x = self.input(x)
        x = torch.clamp(x, 0, 1.0) # ReLU bloccata a 1.0 nel training float
        x = self.output(x)
        return x

def parse_fen(fen):
    """
    Converte FEN in input 768 features.
    Applica SEMPRE il perspective flip: 'Chi muove' è sempre il colore 'Amico' (Index 0-5).
    """
    parts = fen.split()
    board_str = parts[0]
    turn = parts[1] # 'w' o 'b'

    features = np.zeros(INPUT_SIZE, dtype=np.float32)
    
    rank = 7
    col = 0
    
    # Per supportare il flip, dobbiamo raccogliere i pezzi prima
    pieces = [] # (tipo, riga, colonna)

    for char in board_str:
        if char == '/':
            rank -= 1
            col = 0
        elif char.isdigit():
            col += int(char)
        else:
            pieces.append((char, rank, col))
            col += 1
            
    # Ora riempiamo il vettore features applicando la prospettiva
    is_black = (turn == 'b')
    
    for char, r, c in pieces:
        p_idx = PIECE_MAP[char]
        
        # Se tocca al Nero, dobbiamo girare tutto
        if is_black:
            # 1. Flip Verticale della scacchiera
            r = 7 - r
            # c = c ^ 7 (Mirror orizzontale? Di solito no per i pezzi centrali, 
            # ma Stockfish lo fa. Per Simple-768 facciamo solo vertical flip per semplicità)
            
            # 2. Swap Colore (Il mio pezzo deve finire negli indici 0-5)
            # Se era p (6..11) diventa P (0..5)
            if p_idx >= 6: p_idx -= 6 # Nemico diventa Amico
            else: p_idx += 6          # Amico diventa Nemico

        # Calcolo indice finale
        sq = r * 8 + c
        feat_idx = p_idx * 64 + sq
        
        if feat_idx < INPUT_SIZE:
            features[feat_idx] = 1.0

    return features, is_black

class ChessDataset(Dataset):
    def __init__(self, filepath):
        self.samples = []
        print(f"Caricamento {filepath}...")
        with open(filepath, 'r') as f:
            for line in f:
                try:
                    parts = line.split('|')
                    fen = parts[0].strip()
                    # Assumiamo che score nel file sia relativo al BIANCO (es. +100 bianco vince)
                    # Oppure "1.0" bianco vince. Adattare in base al tuo file.
                    # Qui assumo formato CP (centipawns)
                    raw_score = float(parts[1].strip()) 
                    
                    # Normalizza score tra -1 e 1 per tanh? O tieni CP?
                    # Meglio tenere CP scalato per l'output.
                    # Diciamo che il target è in CP.
                    
                    self.samples.append((fen, raw_score))
                except: continue
        print(f"Trovate {len(self.samples)} posizioni.")

    def __len__(self): return len(self.samples)

    def __getitem__(self, idx):
        fen, score = self.samples[idx]
        features, is_black_turn = parse_fen(fen)
        
        # Se tocca al nero, lo score dal punto di vista del bianco va invertito
        # Esempio: Score +100 (vantaggio bianco). Tocca al nero.
        # Per il nero, la posizione vale -100.
        final_score = -score if is_black_turn else score
        
        return torch.tensor(features), torch.tensor([final_score], dtype=torch.float32)

def export_quantized(model, filename):
    model.cpu()
    print(f"Esportazione Quantizzata (Int16) su {filename}...")
    
    with open(filename, "wb") as f:
        # 1. Feature Weights (Input -> Hidden)
        # Scaliamo per SCALE
        w1 = (model.input.weight.data * SCALE).round().to(torch.int16)
        # Trasponiamo per avere (768, 256) lineare
        w1_flat = w1.t().contiguous().numpy().flatten()
        f.write(struct.pack(f'{len(w1_flat)}h', *w1_flat)) # 'h' = short (2 bytes)

        # 2. Feature Bias
        b1 = (model.input.bias.data * SCALE).round().to(torch.int16)
        b1_flat = b1.contiguous().numpy().flatten()
        f.write(struct.pack(f'{len(b1_flat)}h', *b1_flat))

        # 3. Output Weights
        # Nota: L'output layer spesso ha una scala diversa, ma usiamo SCALE anche qui per semplicità
        # Stockfish usa QA=255 per hidden, QB=? per output.
        # Proviamo a scalare anche questi per SCALE.
        w2 = (model.output.weight.data * SCALE).round().to(torch.int16)
        w2_flat = w2.contiguous().numpy().flatten()
        f.write(struct.pack(f'{len(w2_flat)}h', *w2_flat))

        # 4. Output Bias
        b2 = (model.output.bias.data * SCALE * SCALE).round().to(torch.int16) # Output bias ha scala doppia spesso?
        # Semplifichiamo: tutto a SCALE. Se i valori sono piccoli, potrebbe servire SCALE maggiore per output.
        # Manteniamo SCALE singola per ora.
        b2_val = int(model.output.bias.item() * SCALE)
        f.write(struct.pack('h', b2_val))
        
    print("Fatto.")

def train():
    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    print(f"Training su: {device}")
    
    dataset = ChessDataset(DATA_FILE)
    loader = DataLoader(dataset, batch_size=BATCH_SIZE, shuffle=True, num_workers=0) # workers=0 per evitare problemi windows
    
    model = LunaModel().to(device)
    optimizer = optim.Adam(model.parameters(), lr=LEARNING_RATE)
    loss_fn = nn.MSELoss()

    for epoch in range(EPOCHS):
        total_loss = 0
        model.train()
        for X, y in loader:
            X, y = X.to(device), y.to(device)
            optimizer.zero_grad()
            pred = model(X)
            loss = loss_fn(pred, y)
            loss.backward()
            optimizer.step()
            total_loss += loss.item()
            
        print(f"Epoch {epoch+1} | Loss: {total_loss / len(loader):.2f}")
        
    export_quantized(model, MODEL_SAVE_PATH)

if __name__ == "__main__":
    train()