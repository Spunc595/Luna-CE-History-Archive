import torch
import numpy as np
import os

# --- CONFIGURAZIONE ---
CHECKPOINT = "checkpoints/luna-halfka-ultimo.ckpt" # Cambia col tuo file .ckpt
OUTPUT = "luna_brain.nnue"

# Dimensioni richieste dal tuo nnue.rs
FILE_INPUT_SIZE = 49216
FILE_FEAT_STRIDE = 264  # 256 neuroni + 8 di padding
REAL_HIDDEN_SIZE = 256

def export():
    print(f"📂 Caricamento checkpoint HalfKA...")
    checkpoint = torch.load(CHECKPOINT, map_location="cpu", weights_only=False)
    state_dict = checkpoint['state_dict']

    # Estrazione pesi (i nomi devono coincidere con quelli nel tuo modello PyTorch)
    # Di solito: 'input.weight', 'input.bias', 'layer_stacks.0.weight', ecc.
    l1_w = state_dict['input.weight'].numpy() # [256, 49216]
    l1_b = state_dict['input.bias'].numpy()   # [256]
    l2_w = state_dict['layer_stacks.0.weight'].numpy() # [1, 256]
    l2_b = state_dict['layer_stacks.0.bias'].numpy()   # [1]

    # Moltiplicatori di quantizzazione
    QA, QB = 255, 64 

    print("⚙️  Packing binario in corso (Stride 264)...")

    with open(OUTPUT, "wb") as f:
        # 1. Feature Weights (49216 * 264)
        # Dobbiamo aggiungere il padding di 8 per arrivare a 264
        for i in range(FILE_INPUT_SIZE):
            weights = (l1_w[:, i] * QA).astype(np.int16)
            f.write(weights.tobytes())
            f.write(np.zeros(8, dtype=np.int16).tobytes()) # Padding per arrivare a 264

        # 2. Feature Biases (264)
        biases = (l1_b * QA).astype(np.int16)
        f.write(biases.tobytes())
        f.write(np.zeros(8, dtype=np.int16).tobytes()) # Padding

        # 3. Output Weights (256)
        out_w = (l2_w.flatten() * QB).astype(np.int16)
        f.write(out_w.tobytes())

        # 4. Output Bias (8)
        out_b = np.zeros(8, dtype=np.int16)
        out_b[0] = int(l2_b[0] * QB)
        f.write(out_b.tobytes())

    size = os.path.getsize(OUTPUT)
    print(f"✅ SUCCESSO! Generato {OUTPUT}")
    print(f"📏 Dimensione finale: {size} bytes")
    if size == 25987104:
        print("🎯 COMBACIA PERFETTAMENTE col motore Rust!")

if __name__ == "__main__":
    export()