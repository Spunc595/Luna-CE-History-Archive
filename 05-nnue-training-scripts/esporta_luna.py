import torch
import numpy as np
import os
import model as M

# --- CONFIGURAZIONE ---
CHECKPOINT = "checkpoints/luna-turbo-step=0010000.ckpt" # Assicurati che il nome sia corretto!
OUTPUT_FILE = "luna.nnue"

def export_luna():
    print(f"📂 Caricamento checkpoint: {CHECKPOINT}...")
    
    # Setup Feature Set
    fs = M.get_feature_set_from_name("HalfKA")
    
    # --- CORREZIONE QUI ---
    # Dobbiamo passare anche quantize_config, altrimenti la classe NNUE si arrabbia.
    model = M.NNUE.load_from_checkpoint(
        CHECKPOINT, 
        feature_set=fs, 
        config=M.ModelConfig(L1=256),
        quantize_config=M.quantize.QuantizationConfig() # <--- ECCO IL PEZZO MANCANTE
    )
    model.eval()

    print(f"🧠 Esportazione modello HalfKA (45056 -> 256 -> 1)...")

    with open(OUTPUT_FILE, "wb") as f:
        # --- LAYER 1 (Input -> Hidden) ---
        # Trasposizione necessaria per Rust: [256, 45056] -> [45056, 256]
        w1 = model.model.input.weight.data.t().cpu().numpy()
        
        # Scaling a 255 e conversione a i16
        w1_quant = (w1 * 255).astype(np.int16).flatten()
        f.write(w1_quant.tobytes())
        print(f"   -> Pesi Input scritti: {w1_quant.size} valori")

        # Bias Input
        b1 = model.model.input.bias.data.cpu().numpy()
        b1_quant = (b1 * 255).astype(np.int16).flatten()
        f.write(b1_quant.tobytes())
        print(f"   -> Bias Input scritti: {b1_quant.size} valori")

        # --- LAYER 2 (Hidden -> Output) ---
        w2 = model.model.layer_stacks.output.linear.weight.data.cpu().numpy()
        w2_quant = (w2 * 255).astype(np.int16).flatten()
        f.write(w2_quant.tobytes())
        print(f"   -> Pesi Output scritti: {w2_quant.size} valori")

        # Bias Output
        b2 = model.model.layer_stacks.output.linear.bias.data.cpu().numpy()
        b2_quant = (b2 * 255).astype(np.int16).flatten()
        f.write(b2_quant.tobytes())
        print(f"   -> Bias Output scritto: {b2_quant.size} valori")

    if os.path.exists(OUTPUT_FILE):
        size_mb = os.path.getsize(OUTPUT_FILE) / (1024 * 1024)
        print(f"\n✅ Successo! File creato: {OUTPUT_FILE} ({size_mb:.2f} MB)")
        print("   ORA SPOSTA QUESTO FILE NELLA CARTELLA DEL TUO PROGETTO RUST!")
    else:
        print("\n❌ Errore: Il file non è stato creato.")

if __name__ == "__main__":
    if not os.path.exists(CHECKPOINT):
        print(f"❌ Errore: Il file {CHECKPOINT} non esiste. Controlla il nome nella cartella checkpoints!")
    else:
        export_luna()