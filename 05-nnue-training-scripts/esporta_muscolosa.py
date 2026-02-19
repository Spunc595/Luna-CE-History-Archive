import torch
import numpy as np
import os
import simple_model as M

# --- CONFIGURAZIONE SICURA (ANTI-OVERFLOW) ---
CHECKPOINT = "Checkpoints/luna-simple-step=200000.ckpt" 
OUTPUT_NAME = "luna.nnue" # Nome file aggiornato

# Usiamo valori che stanno comodamente in un int16 (max 32767)
QA = 255  # Prima era 512 (Troppo alto!)
QB = 64   # Prima era 128

# SBLOCCO SICUREZZA
torch.serialization.add_safe_globals([M.ModelConfig, M.QuantizationConfig, M.NNUE, M.SimpleFeatureSet])

def export():
    if not os.path.exists(CHECKPOINT):
        print(f"❌ Errore: Il file {CHECKPOINT} non esiste!")
        return

    print(f"🛡️  Esportazione Luna 52.000 (SAFE MODE) in corso...")
    
    checkpoint = torch.load(CHECKPOINT, map_location='cpu', weights_only=False)
    state_dict = checkpoint['state_dict']
    
    with open(OUTPUT_NAME, "wb") as f:
        # Layer 1: Input (QA=255)
        print("-> Esportazione Layer 1...")
        l1_w = state_dict['input.weight'].data.numpy()
        l1_b = state_dict['input.bias'].data.numpy()
        (l1_w * QA).astype(np.int16).tofile(f)
        (l1_b * QA).astype(np.int16).tofile(f)
        
        # Layer 2: Hidden (QB=64)
        print("-> Esportazione Layer 2...")
        l2_w = state_dict['layer_stacks.0.weight'].data.numpy()
        l2_b = state_dict['layer_stacks.0.bias'].data.numpy()
        (l2_w * QB).astype(np.int16).tofile(f)
        (l2_b * QB).astype(np.int16).tofile(f)
        
        # Layer 3: Output (QB=64)
        print("-> Esportazione Layer 3...")
        l3_w = state_dict['layer_stacks.2.weight'].data.numpy()
        l3_b = state_dict['layer_stacks.2.bias'].data.numpy()
        (l3_w * QB).astype(np.int16).tofile(f)
        (l3_b * QB).astype(np.int16).tofile(f)

    print(f"\n✅ FILE SICURO GENERATO: {OUTPUT_NAME}")
    print(f"Ora aggiorna 'main.rs' per caricare questo file e ricompila!")

if __name__ == "__main__":
    export()