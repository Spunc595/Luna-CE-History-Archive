import sys
import os
import torch
import chess
import model as M

def carica_luna():
    # 1. Trova l'ultimo checkpoint nella cartella
    folder = 'checkpoints/'
    if not os.path.exists(folder):
        print("❌ Cartella checkpoints non trovata!")
        return None
    
    files = [f for f in os.listdir(folder) if f.endswith('.ckpt')]
    if not files:
        print("❌ Nessun file .ckpt trovato!")
        return None
    
    # Prende il file con lo step più alto (il più recente)
    files.sort()
    latest_file = os.path.join(folder, files[-1])
    print(f"📂 Caricamento di Luna da: {latest_file}")

    # 2. Inizializzazione configurazione
    fs = M.get_feature_set_from_name("HalfKA")
    config = M.ModelConfig(L1=256)
    q_config = M.quantize.QuantizationConfig()

    try:
        # Carichiamo il modello (usando la CPU per i test)
        model = M.NNUE.load_from_checkpoint(
            latest_file, 
            feature_set=fs, 
            config=config, 
            quantize_config=q_config,
            map_location='cpu'
        )
        model.eval() 
        return model, fs
    except Exception as e:
        print(f"❌ Errore durante il caricamento del file: {e}")
        return None

def prepara_tensor(indices, target_dim=512):
    """Prepara gli indici nel formato corretto per la rete."""
    idx_t = torch.full((1, target_dim), -1, dtype=torch.int32)
    val_t = torch.zeros((1, target_dim), dtype=torch.float32)
    real_idx = torch.as_tensor(indices, dtype=torch.int32)
    n = min(len(real_idx), target_dim)
    idx_t[0, :n] = real_idx[:n]
    val_t[0, :n] = 1.0
    return idx_t, val_t

def valuta_posizione(model, fs, fen):
    """Calcola il punteggio della posizione data."""
    board = chess.Board(fen)
    
    # --- PROSPETTIVE ---
    # Fondamentale: prendiamo sia la vista del Bianco (0) che del Nero (1)
    active = fs.get_active_features(board)
    idx_w, val_w = prepara_tensor(active[0])
    idx_b, val_b = prepara_tensor(active[1])

    # Parametri di turno e indici ausiliari
    us = torch.tensor([1.0 if board.turn == chess.WHITE else 0.0])
    them = torch.tensor([0.0 if board.turn == chess.WHITE else 1.0])
    p_idx = torch.tensor([0], dtype=torch.long)
    l_idx = torch.tensor([0], dtype=torch.long)

    with torch.no_grad():
        # Passaggio dei dati alla rete neurale
        output = model.model(us, them, idx_w, val_w, idx_b, val_b, p_idx, l_idx)
        
    # Moltiplichiamo per 100 per avere i centipedoni (cp)
    score = output.item() * 100
    return score

if __name__ == "__main__":
    print("\n" + "="*40)
    print("   LUNA TESTER: Daniele's Edition v2.0   ")
    print("="*40)
    
    res = carica_luna()
    
    if res:
        luna, fs = res
        while True:
            print("\n" + "-"*40)
            print(" Inserisci FEN da testare:")
            print(" [Invio] Posizione Iniziale | [q] Esci | [r] Ricarica")
            fen = input(" >> ").strip()
            
            if fen.lower() == 'q': break
            if fen.lower() == 'r':
                res = carica_luna()
                if res: luna, fs = res
                continue
            
            if not fen: fen = chess.STARTING_FEN
            
            try:
                score = valuta_posizione(luna, fs, fen)
                
                print(f"\n 📍 Posizione: {fen}")
                # Visualizziamo 4 decimali: essenziale nelle fasi iniziali del training
                print(f" 🌕 Valutazione Luna: {score:+.4f} cp")
                
                # Interpretazione del risultato
                if abs(score) < 0.0001:
                    print(" ℹ️  Risultato neutro: Probabile simmetria o pesi ancora acerbi.")
                elif score > 0.1:
                    print(" ⚪ Esito: Vantaggio Bianco")
                elif score < -0.1:
                    print(" ⚫ Esito: Vantaggio Nero")
                else:
                    print(" ⚖️  Esito: Posizione quasi in equilibrio")
                
            except Exception as e:
                print(f" ❌ Errore durante l'analisi: {e}")