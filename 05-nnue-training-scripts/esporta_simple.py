import torch
import os
import simple_model as M
import numpy as np

# USA IL TUO ULTIMO CHECKPOINT (es. 3902)
CHECKPOINT = "checkpoints/luna-simple-step=19000.ckpt" 
OUTPUT = "luna_simple.nnue"

def export():
    if not os.path.exists(CHECKPOINT):
        print(f"❌ Errore: {CHECKPOINT} non trovato!")
        return

    checkpoint_data = torch.load(CHECKPOINT, map_location="cpu", weights_only=False)
    model = M.NNUE(M.SimpleFeatureSet(), M.ModelConfig(), M.QuantizationConfig())
    model.load_state_dict(checkpoint_data['state_dict'])
    model.eval()

    # ALZIAMO IL VOLUME: QA=2048 per non perdere i pesi piccoli
    QA, QB = 2048, 128

    with open(OUTPUT, "wb") as f:
        # Layer 1
        f.write((model.input.weight.data.t().numpy() * QA).astype('int16').tobytes())
        f.write((model.input.bias.data.numpy() * QA).astype('int16').tobytes())
        # Layer 2
        f.write((model.layer_stacks[0].weight.data.t().numpy() * QB).astype('int16').tobytes())
        f.write((model.layer_stacks[0].bias.data.numpy() * QB).astype('int16').tobytes())
        # Layer 3
        f.write((model.layer_stacks[2].weight.data.t().numpy() * QB).astype('int16').tobytes())
        f.write((model.layer_stacks[2].bias.data.numpy() * QB).astype('int16').tobytes())

    print(f"✅ Esportazione con QA={QA} completata!")

if __name__ == "__main__":
    export()