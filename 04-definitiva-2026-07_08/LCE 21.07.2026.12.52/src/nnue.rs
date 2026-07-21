use std::fs::File;
use std::io::Read;
use crate::board::{Scacchiera, Colore};

const INPUT_SIZE: usize = 768;
const L1_SIZE: usize = 256;
const L2_SIZE: usize = 32;

// ============================================================================
// QUANTIZZAZIONE: COSTANTI DI SCALA (CORRETTE)
// ============================================================================
//
// Prima di questa revisione i divisori/clamp dei vari layer erano stati
// "dimezzati" a mano (commenti tipo "Dimezzato da 4096 a 2048") senza una
// derivazione coerente: il commento sul layer 1 dichiarava QA=255, ma il
// clamp effettivo era 2048 — un valore totalmente scollegato dal QA
// dichiarato. Idem per gli altri layer. Il risultato pratico era che
// l'uscita finale della rete non aveva alcuna relazione garantita con i
// centipawn, il che è disastroso per qualunque euristica di ricerca che
// confronti lo static eval con alpha/beta (Null Move Pruning, Reverse
// Futility Pruning, stand-pat in quiescence, finestra di aspirazione): un
// errore di scala si traduce direttamente in pruning troppo aggressivo o
// troppo timido, indipendentemente dalla qualità della rete stessa.
//
// Schema di quantizzazione adottato (stile "feature transformer" classico):
//  - QA: scala dei pesi/bias del layer 1 e quindi dell'accumulatore. Un
//    peso quantizzato memorizza `peso_reale * QA`. Dopo la ClippedReLU,
//    l'attivazione vive nel range [0, QA], cioè rappresenta un valore reale
//    in [0, 1]. IL CLAMP USA ORA ESATTAMENTE QA, non un valore arbitrario.
//  - QB: scala dei pesi del layer 2 e del layer 3.
//  - Il prodotto (attivazione scalata QA) * (peso scalato QB) è alla scala
//    QA*QB: dividendo per QA si torna alla scala QB, coerente col bias del
//    layer successivo (anch'esso quantizzato a QB). IL DIVISORE È ORA QA,
//    non un valore arbitrario scollegato.
//  - Il layer 3 produce un valore alla scala QB: un UNICO fattore di
//    conversione finale (OUTPUT_SCALE) lo porta in centipawn, invece dei
//    vari "aggiustamenti" sparsi nei layer intermedi della vecchia
//    implementazione.
const QA: i32 = 255;
const QB: i32 = 64;

/// Fattore di conversione finale: mappa l'uscita quantizzata (scala QB) in
/// centipawn, in modo che 100 rappresenti approssimativamente il valore di
/// un pedone, coerentemente con `EvalParams::mg_pawn` in evaluation.rs.
/// È l'UNICO punto in cui si tara la rete sui centipawn: quando verrà
/// caricata una rete realmente allenata, questo è il valore da ricalibrare
/// (tipicamente confrontando l'output medio della rete su un set di
/// posizioni con la valutazione materiale/PST classica tramite
/// `debug_scale_diff`, vedi sotto).
const OUTPUT_SCALE: i32 = 1;

/// Limite di sicurezza per il punteggio finale restituito dalla rete. Il
/// motore di ricerca (search.rs) codifica i punteggi di matto come
/// `±(MATE_SCORE - ply)`, con MATE_SCORE = 49_000. Se lo static eval
/// potesse mai avvicinarsi a quell'ordine di grandezza, la logica di
/// `is_mate_score()` in search.rs potrebbe confondere una valutazione
/// "normale" (per quanto estrema) con un punteggio di matto, corrompendo
/// pruning e gestione della TT. Per questo l'uscita della rete viene
/// SEMPRE clampata ben al di sotto di quella soglia, indipendentemente da
/// quanto "confidente" sia la rete stessa.
pub const NNUE_EVAL_CLAMP: i32 = 15_000;

pub struct LunaNNUE {
    l1_weights: Vec<i16>,
    l1_bias: Vec<i16>,
    l2_weights: Vec<i16>,
    l2_bias: Vec<i16>,
    l3_weights: Vec<i16>,
    l3_bias: Vec<i16>,
}

// ============================================================================
// ACCUMULATORE INCREMENTALE (Layer 1, PRE-ClippedReLU)
// ============================================================================
//
// Il layer 1 è puramente lineare (somma di bias + contributi additivi di
// ciascun pezzo presente sulla scacchiera): questo è esattamente ciò che
// rende possibile l'aggiornamento incrementale "in stile Stockfish". Invece
// di ricalcolare l1_acc da zero (loop su tutti i pezzi, ~O(pezzi presenti))
// ad ogni singola chiamata a evaluate(), board.rs mantiene un solo
// Accumulator per posizione e lo aggiorna con add_piece/remove_piece
// esattamente per le feature toccate da ciascuna mossa (~O(pezzi coinvolti
// nella mossa), tipicamente 2-4), sia in esegui_mossa che nel suo
// annullamento. evaluate_from_accumulator() si limita quindi a eseguire i
// layer 2 e 3, che sono già O(L1_SIZE * L2_SIZE) indipendentemente dal
// numero di pezzi sulla scacchiera.
//
// IMPORTANTE: la somma viene mantenuta in i32, NON in i16 con
// saturating_add/sub come nella prima versione di questo file. La
// saturazione non è invertibile: se una dimensione arriva a toccare il
// limite di i16 durante un add_piece, un remove_piece successivo non può
// più ricostruire il valore reale (l'informazione "in eccesso" è già
// andata persa), e l'accumulatore incrementale diverge silenziosamente da
// un ricalcolo completo. Con i32 la somma di ~32 pezzi * pesi i16 resta
// per costruzione ben lontana dal limite (±32767 * ~32 << i32::MAX), quindi
// add_piece/remove_piece sono l'uno l'esatto inverso dell'altro in ogni
// caso, senza eccezioni legate all'ampiezza dei pesi caricati.
#[derive(Clone, Copy, Debug)]
pub struct Accumulator {
    pub v: [i32; L1_SIZE],
}

impl Accumulator {
    pub const fn zero() -> Self {
        Accumulator { v: [0i32; L1_SIZE] }
    }
}

impl Default for Accumulator {
    fn default() -> Self { Self::zero() }
}

impl LunaNNUE {
    pub fn load(path: &str) -> Option<Self> {
        let mut file = File::open(path).ok()?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).ok()?;

        let mut net = LunaNNUE {
            l1_weights: vec![0; INPUT_SIZE * L1_SIZE],
            l1_bias: vec![0; L1_SIZE],
            l2_weights: vec![0; L1_SIZE * L2_SIZE],
            l2_bias: vec![0; L2_SIZE],
            l3_weights: vec![0; L2_SIZE],
            l3_bias: vec![0; 1],
        };

        let mut offset = 0;
        let mut read_i16 = |count: usize| -> Vec<i16> {
            let mut v = Vec::with_capacity(count);
            for _ in 0..count {
                if offset + 2 <= buffer.len() {
                    v.push(i16::from_le_bytes([buffer[offset], buffer[offset + 1]]));
                    offset += 2;
                }
            }
            v
        };

        net.l1_weights = read_i16(INPUT_SIZE * L1_SIZE);
        net.l1_bias = read_i16(L1_SIZE);
        net.l2_weights = read_i16(L1_SIZE * L2_SIZE);
        net.l2_bias = read_i16(L2_SIZE);
        net.l3_weights = read_i16(L2_SIZE);
        net.l3_bias = read_i16(1);

        // Check semplice per vedere se il file è valido
        if net.l1_weights.iter().take(100).all(|&x| x == 0) {
            println!("⚠️ ATTENZIONE: Il file NNUE sembra vuoto!");
        } else {
            println!("✅ NNUE: Pesi caricati.");
        }

        Some(net)
    }

    /// Indice della feature (colore, pezzo, casella) nello spazio di input
    /// a 768 dimensioni (12 piani da 64 caselle: 6 tipi di pezzo x 2 colori).
    /// Va moltiplicato per L1_SIZE per ottenere l'offset nella matrice dei
    /// pesi del layer 1 (`l1_weights`), organizzata come 768 righe da
    /// L1_SIZE colonne.
    #[inline(always)]
    fn feature_index(color: usize, piece: usize, sq: usize) -> usize {
        (piece + color * 6) * 64 + sq
    }

    /// Ricostruisce l'accumulatore da zero (bias del layer 1 + contributo
    /// di ogni pezzo presente sulla scacchiera). Costo O(pezzi presenti *
    /// L1_SIZE): da usare solo quando non esiste ancora un accumulatore
    /// incrementale valido per la posizione corrente (tipicamente una sola
    /// volta, subito dopo aver impostato una nuova posizione/FEN). Durante
    /// la discesa nell'albero di ricerca, l'accumulatore viene invece
    /// mantenuto aggiornato incrementalmente da board.rs tramite
    /// `add_piece`/`remove_piece` ad ogni make/unmake, senza mai richiamare
    /// questa funzione.
    pub fn refresh(&self, s: &Scacchiera) -> Accumulator {
        let mut acc = Accumulator::zero();
        for i in 0..L1_SIZE {
            acc.v[i] = self.l1_bias[i] as i32;
        }

        for p_idx in 0..6 {
            let mut bb_w = s.pezzi[p_idx] & s.colori[0];
            while bb_w != 0 {
                let sq = bb_w.trailing_zeros() as usize;
                self.add_piece(&mut acc, 0, p_idx, sq);
                bb_w &= bb_w - 1;
            }
            let mut bb_b = s.pezzi[p_idx] & s.colori[1];
            while bb_b != 0 {
                let sq = bb_b.trailing_zeros() as usize;
                self.add_piece(&mut acc, 1, p_idx, sq);
                bb_b &= bb_b - 1;
            }
        }
        acc
    }

    /// Aggiunge all'accumulatore il contributo della feature (colore,
    /// pezzo, casella): da chiamare quando un pezzo di quel tipo COMPARE su
    /// quella casella (destinazione di una mossa, promozione, o un pezzo
    /// che ricompare durante l'annullamento di una cattura/mossa).
    #[inline(always)]
    pub fn add_piece(&self, acc: &mut Accumulator, color: usize, piece: usize, sq: usize) {
        let offset = Self::feature_index(color, piece, sq) * L1_SIZE;
        for i in 0..L1_SIZE {
            acc.v[i] += self.l1_weights[offset + i] as i32;
        }
    }

    /// Rimuove dall'accumulatore il contributo della feature (colore,
    /// pezzo, casella): l'esatto inverso di `add_piece`, da chiamare quando
    /// un pezzo SPARISCE da quella casella (partenza di una mossa, pezzo
    /// catturato, o annullamento della comparsa di un pezzo).
    #[inline(always)]
    pub fn remove_piece(&self, acc: &mut Accumulator, color: usize, piece: usize, sq: usize) {
        let offset = Self::feature_index(color, piece, sq) * L1_SIZE;
        for i in 0..L1_SIZE {
            acc.v[i] -= self.l1_weights[offset + i] as i32;
        }
    }

    /// Layer 2 + Layer 3, a partire da un accumulatore GIÀ aggiornato
    /// (incrementalmente o via `refresh`). È la parte della rete
    /// effettivamente eseguita ad ogni nodo di ricerca sul percorso caldo:
    /// il layer 1 (il più costoso, O(pezzi presenti)) non compare più qui,
    /// perché il suo risultato è mantenuto aggiornato altrove. Costo fisso
    /// O(L1_SIZE * L2_SIZE + L2_SIZE), indipendente dal numero di pezzi
    /// sulla scacchiera.
    pub fn evaluate_from_accumulator(&self, acc: &Accumulator, turno: Colore) -> i32 {
        let mut l1_out = [0i16; L1_SIZE];
        for i in 0..L1_SIZE {
            // CLAMP L1: coincide esattamente con QA (255), coerente con la
            // scala dichiarata dei pesi del layer 1. Attivazione risultante
            // in [0, QA]. Il clamp qui è l'UNICA non-linearità voluta della
            // rete (ClippedReLU): a differenza della saturazione rimossa
            // dall'accumulatore, qui è intenzionale e non deve essere
            // "invertita" da nessuno.
            l1_out[i] = acc.v[i].clamp(0, QA) as i16;
        }

        // --- LAYER 2 (input scala QA, pesi scala QB) ---
        let mut l2_pre = [0i32; L2_SIZE];
        for i in 0..L2_SIZE {
            let mut sum: i32 = self.l2_bias[i] as i32;
            for j in 0..L1_SIZE {
                // DIVISORE L2: QA. Attivazione (scala QA) * peso (scala QB)
                // / QA riporta correttamente il prodotto alla scala QB del
                // bias.
                sum += (l1_out[j] as i32 * self.l2_weights[j * L2_SIZE + i] as i32) / QA;
            }
            l2_pre[i] = sum.clamp(-32768, 32767);
        }

        let mut l2_out = [0i16; L2_SIZE];
        for i in 0..L2_SIZE {
            // CLAMP L2: [0, QB].
            l2_out[i] = l2_pre[i].clamp(0, QB) as i16;
        }

        // --- LAYER 3: Output (input scala QB, pesi scala QB) ---
        let mut score: i32 = self.l3_bias[0] as i32;
        for j in 0..L2_SIZE {
            // DIVISORE L3: QB.
            score += (l2_out[j] as i32 * self.l3_weights[j] as i32) / QB;
        }

        // --- Conversione finale in centipawn + clamp di sicurezza ---
        // Da qui in poi il valore è, per costruzione, nella stessa unità di
        // misura (centipawn) usata da `evaluate()` in evaluation.rs, e
        // quindi confrontabile in modo sicuro con alpha/beta nelle
        // euristiche di pruning statiche (RFP, NMP, stand-pat). Il clamp
        // finale garantisce inoltre che il punteggio non possa mai
        // collidere con la codifica dei punteggi di matto usata da
        // search.rs, qualunque sia il comportamento della rete caricata.
        let centipawns = (score * OUTPUT_SCALE).clamp(-NNUE_EVAL_CLAMP, NNUE_EVAL_CLAMP);

        if turno == Colore::Bianco { centipawns } else { -centipawns }
    }

    /// Valutazione "da zero", senza un accumulatore incrementale esterno:
    /// `refresh` + `evaluate_from_accumulator`. Comoda per usi occasionali
    /// fuori dal percorso caldo (comando UCI "eval" manuale,
    /// `debug_scale_diff`), ma NON va chiamata nel loop di negamax/
    /// quiescence: lì è disponibile l'accumulatore incrementale mantenuto
    /// in `Scacchiera::nnue_acc`, molto più economico da valutare.
    pub fn evaluate(&self, s: &Scacchiera) -> i32 {
        let acc = self.refresh(s);
        self.evaluate_from_accumulator(&acc, s.turno)
    }

    /// Confronta l'uscita della rete con una valutazione di riferimento
    /// (tipicamente `evaluation::evaluate` sulla stessa posizione) per
    /// verificare che la scala sia effettivamente compatibile con i
    /// centipawn. Non è usato nel percorso caldo della ricerca: va
    /// invocato una tantum in fase di validazione (es. da un comando UCI
    /// custom tipo "eval compare" o da un test), specialmente dopo aver
    /// caricato una rete realmente allenata, quando i pesi non sono più i
    /// placeholder di default. Una differenza sistematicamente grande (es.
    /// > 200-300 cp su posizioni "tranquille") è il segnale che
    /// OUTPUT_SCALE va ricalibrato.
    pub fn debug_scale_diff(&self, s: &Scacchiera, reference_score: i32) -> i32 {
        (self.evaluate(s) - reference_score).abs()
    }
}