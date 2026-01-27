import torch
import torch.nn as nn

# Configurazione identica a nnue.rs
INPUT_SIZE = 768  # 64 case * 12 pezzi
HIDDEN_SIZE = 256
OUTPUT_SIZE = 1

class LunaNNUE(nn.Module):
    def __init__(self):
        super(LunaNNUE, self).__init__()
        # Input Layer: riceve le feature (pezzo in casa)
        self.input = nn.Linear(INPUT_SIZE, HIDDEN_SIZE)
        # Output Layer: sputa la valutazione
        self.output = nn.Linear(HIDDEN_SIZE, OUTPUT_SIZE)
        
        # Inizializzazione: aiuta la rete a capire subito il valore dei pezzi
        nn.init.kaiming_uniform_(self.input.weight, nonlinearity='relu')
        nn.init.zeros_(self.input.bias)

    def forward(self, x):
        x = self.input(x)
        # Usiamo ReLU standard durante il training. 
        # In Rust il clamp(0, 255) farà il resto.
        x = torch.relu(x) 
        x = self.output(x)
        return x