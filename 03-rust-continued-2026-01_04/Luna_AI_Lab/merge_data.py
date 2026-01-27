import random

# Nomi dei tuoi file
FILE_VECCHIO = "dataset_2500_elo.txt"   # Il tuo file da 80k
FILE_NUOVO = "dataset_new.txt"          # Il file generato dalle 4000 partite (convertito)
FILE_OUTPUT = "dataset_completo.txt"    # Il file finale

def merge_and_shuffle():
    print("Lettura file vecchio...")
    with open(FILE_VECCHIO, "r") as f:
        lines_old = f.readlines()
        
    print(f"-> Trovate {len(lines_old)} posizioni.")

    print("Lettura file nuovo...")
    try:
        with open(FILE_NUOVO, "r") as f:
            lines_new = f.readlines()
        print(f"-> Trovate {len(lines_new)} posizioni.")
    except FileNotFoundError:
        print("ERRORE: Non trovo il file nuovo. Assicurati di aver parsato i PGN!")
        return

    # Unione
    all_lines = lines_old + lines_new
    print(f"Totale grezzo: {len(all_lines)}")

    # Mescolamento (Shuffle) - CRUCIALE per il training
    print("Mescolamento in corso...")
    random.shuffle(all_lines)

    # Scrittura
    print(f"Scrittura su {FILE_OUTPUT}...")
    with open(FILE_OUTPUT, "w") as f:
        f.writelines(all_lines)

    print("Fatto! Ora usa 'dataset_completo.txt' per il training.")

if __name__ == "__main__":
    merge_and_shuffle()