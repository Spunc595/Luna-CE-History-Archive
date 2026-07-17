import os
import argparse

import torch
import numpy as np
import simple_model as M

# --- SCALE DI QUANTIZZAZIONE ---
# DEVONO combaciare con nnue.rs:
#   Layer 1 (input)      -> pesi/bias * QA
#   Layer 2/3 (stacks)   -> pesi/bias * QB
QA = 255
QB = 64
I16_MIN, I16_MAX = -32768, 32767

# SBLOCCO SICUREZZA (PyTorch 2.x)
torch.serialization.add_safe_globals([M.ModelConfig, M.QuantizationConfig, M.NNUE, M.SimpleFeatureSet])


def quantize(tensor, scale, name):
    """
    Quantizza in int16 con ARROTONDAMENTO (non troncamento) e con controllo
    di overflow: se qualche valore supera il range int16 avvisa e clampa,
    invece di fare wrap-around silenzioso (che corromperebbe la rete).
    """
    arr = tensor.detach().cpu().numpy().astype(np.float64)
    scaled = arr * scale
    max_abs = np.abs(scaled).max() if scaled.size else 0.0
    if max_abs > I16_MAX:
        print(f"⚠️  {name}: max |peso*scala| = {max_abs:.0f} > {I16_MAX} "
              f"(OVERFLOW int16!). Valori clampati; considera una scala piu' bassa "
              f"o una regolarizzazione dei pesi in training.")
    return np.clip(np.round(scaled), I16_MIN, I16_MAX).astype(np.int16)


def export(checkpoint_path, output_name):
    if not os.path.exists(checkpoint_path):
        print(f"❌ Errore: Il file {checkpoint_path} non esiste!")
        return

    print(f"🛡️  Esportazione Luna (SAFE MODE) da {checkpoint_path} ...")
    checkpoint = torch.load(checkpoint_path, map_location="cpu", weights_only=False)
    state_dict = checkpoint["state_dict"]

    # NB IMPORTANTE:
    # I pesi vengono scritti COSI' COME li fornisce PyTorch, cioe' in layout
    # [out_features, in_features] (output-major). E' il loader Rust (nnue.rs)
    # che li traspone al caricamento. NON trasporre qui.
    with open(output_name, "wb") as f:
        print("-> Layer 1 (input, scala QA)...")
        quantize(state_dict["input.weight"], QA, "l1_w").tofile(f)
        quantize(state_dict["input.bias"], QA, "l1_b").tofile(f)

        print("-> Layer 2 (layer_stacks.0, scala QB)...")
        quantize(state_dict["layer_stacks.0.weight"], QB, "l2_w").tofile(f)
        quantize(state_dict["layer_stacks.0.bias"], QB, "l2_b").tofile(f)

        print("-> Layer 3 (layer_stacks.2, scala QB)...")
        quantize(state_dict["layer_stacks.2.weight"], QB, "l3_w").tofile(f)
        quantize(state_dict["layer_stacks.2.bias"], QB, "l3_b").tofile(f)

    size = os.path.getsize(output_name)
    expected = (768 * 256 + 256 + 256 * 32 + 32 + 32 + 1) * 2
    status = "OK" if size == expected else "!!! DIMENSIONE INATTESA !!!"
    print(f"\n✅ FILE GENERATO: {output_name}  ({size} byte, atteso {expected}) [{status}]")
    print("Copia il file accanto all'eseguibile e ricompila/riavvia il motore.")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Esporta un checkpoint Luna in luna.nnue")
    parser.add_argument("--ckpt", default="checkpoints/luna-simple-step=200000.ckpt",
                        help="percorso del checkpoint .ckpt")
    parser.add_argument("--out", default="luna.nnue", help="nome del file .nnue in uscita")
    args = parser.parse_args()
    export(args.ckpt, args.out)
