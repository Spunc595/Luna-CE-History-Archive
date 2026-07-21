use std::fs::File;
use std::io::Read;
use crate::board::Scacchiera;

// ============================================================================
// FORMATO FILE: Stockfish NNUE classico, architettura HalfKP 256x2-32-32-1
// ============================================================================
//
// Questa NON è più la rete "giocattolo" a 768 feature piatte delle revisioni
// precedenti: è un parser per il formato binario REALE usato da Stockfish
// (era "nodchip/Stockfish", 2020, la stessa architettura di gran parte dei
// primi net .nnue ancora in circolazione: 41024 feature HalfKP, doppia
// prospettiva, 256 neuroni per prospettiva, poi 512->32->32->1). Le costanti
// sotto (hash, offset, scale bits, FV_SCALE) sono state verificate leggendo
// direttamente il codice sorgente di quella revisione di Stockfish
// (src/nnue/nnue_common.h, features/half_kp.{h,cpp}, nnue_feature_transformer.h,
// layers/{affine_transform,clipped_relu,input_slice}.h), non ricostruite a
// memoria: un errore anche di un solo offset qui non farebbe "fallire" in modo
// visibile il caricamento, produrrebbe semplicemente una valutazione basata su
// pesi disallineati, indistinguibile a occhio da una rete debole.
//
// Limite noto: supporta SOLO questa architettura (HalfKP classico). I file
// .nnue "moderni" di Stockfish (HalfKAv2_hm, layer-stack a bucket, pesi FT
// compressi LEB128) hanno un formato completamente diverso e vengono
// correttamente rifiutati al caricamento (non silenziosamente letti male).

/// Numero di caselle della scacchiera.
const SQUARE_NB: usize = 64;

/// PS_END dell'enum PieceSquareIndex originale (10 * SQUARE_NB + 1): numero
/// di "slot" piece-square per un singolo king-bucket, pedoni compresi ma RE
/// escluso (i re non sono mai feature tracciate in HalfKP, sono usati solo
/// per selezionare il king-bucket). I valori PS_W_PAWN=1, PS_B_PAWN=65, ...,
/// PS_W_QUEEN=513, PS_B_QUEEN=577 derivano dalla stessa formula (vedi
/// `ps_base` sotto) e non sono ripetuti qui singolarmente.
const PS_END: u32 = 10 * SQUARE_NB as u32 + 1; // 641

/// Dimensioni dello spazio di input HalfKP: un blocco di PS_END feature per
/// ciascuna delle 64 possibili caselle del re "associato" (king-bucket).
pub const HALFKP_INPUT_DIMENSIONS: usize = SQUARE_NB * PS_END as usize; // 41024

/// Numero di neuroni del feature transformer PER PROSPETTIVA (kTransformedFeatureDimensions
/// nel codice originale). L'input del primo layer nascosto è il doppio
/// (concatenazione [prospettiva "noi", prospettiva "loro"]).
const L1_SIZE: usize = 256;
/// Larghezza dei due layer nascosti (512->32->32->1).
const L2_SIZE: usize = 32;
const L3_SIZE: usize = 32;

/// Magic number di versione del formato file (kVersion in nnue_common.h).
const SF_NNUE_VERSION: u32 = 0x7AF3_2F16;

/// Numero di bit di shift tra un layer affine (int8) e il successivo
/// (kWeightScaleBits): i pesi dei layer nascosti sono quantizzati con un
/// fattore di scala 2^6=64, quindi la somma pesata va riportata alla scala
/// dell'input con uno shift aritmetico a destra di 6 PRIMA della ClippedReLU
/// successiva. Il feature transformer e il layer di output sono gli unici
/// due punti che NON applicano questo shift (vedi commenti più sotto).
const WEIGHT_SCALE_BITS: u32 = 6;

/// Fattore di conversione finale dall'uscita intera del layer di output ai
/// centipawn (FV_SCALE in nnue_common.h). Aggiornamento: nei file HalfKP
/// classici è l'UNICO punto di conversione in centipawn, non serve nessun
/// OUTPUT_SCALE aggiuntivo come nella vecchia rete "giocattolo".
const FV_SCALE: i32 = 16;

/// Limite di sicurezza per il punteggio finale, per non confondere una
/// valutazione estrema con un punteggio di matto codificato da search.rs
/// (±(MATE_SCORE - ply), MATE_SCORE = 49_000). Vedi ragionamento identico
/// nella revisione precedente di questo file.
pub const NNUE_EVAL_CLAMP: i32 = 15_000;

// ----------------------------------------------------------------------------
// Hash di compatibilità architetturale (informativi, NON usati come gate)
// ----------------------------------------------------------------------------
//
// Stockfish rifiuta un file .nnue se l'hash embedded non corrisponde
// esattamente a quello calcolato dall'architettura compilata. Abbiamo
// ricostruito la stessa catena di hash leggendo il codice sorgente (vedi
// commento in testa al modulo), ma un file .nnue REALE su cui verificarla non
// era disponibile al momento di scrivere questo parser. Per non rischiare di
// rifiutare un file altrimenti valido per un mio errore di trascrizione,
// l'hash viene calcolato e confrontato solo a scopo diagnostico (un
// messaggio "info string" in caso di mismatch): il vero gate di correttezza è
// il controllo strutturale sulla dimensione del file (`expected_len` in
// `parse`), che non dipende da nessuna di queste costanti.
const HALFKP_HASH: u32 = 0x5D69_D5B9 ^ 1; // AssociatedKing::kFriend -> true -> 1
const FT_OUTPUT_DIMS: u32 = (L1_SIZE * 2) as u32; // 512
const FT_HASH: u32 = HALFKP_HASH ^ FT_OUTPUT_DIMS;
const INPUT_SLICE_HASH: u32 = 0xEC42_E90D ^ FT_OUTPUT_DIMS; // Offset = 0

const fn affine_hash(out_dims: u32, prev_hash: u32) -> u32 {
    let mut h: u32 = 0xCC03_DAE4u32.wrapping_add(out_dims);
    h ^= prev_hash >> 1;
    h ^= prev_hash << 31;
    h
}
const fn clipped_relu_hash(prev_hash: u32) -> u32 {
    0x538D_24C7u32.wrapping_add(prev_hash)
}

const LAYER1_HASH: u32 = affine_hash(L2_SIZE as u32, INPUT_SLICE_HASH);
const RELU1_HASH: u32 = clipped_relu_hash(LAYER1_HASH);
const LAYER2_HASH: u32 = affine_hash(L3_SIZE as u32, RELU1_HASH);
const RELU2_HASH: u32 = clipped_relu_hash(LAYER2_HASH);
const OUTPUT_HASH: u32 = affine_hash(1, RELU2_HASH);
const TOP_HASH: u32 = FT_HASH ^ OUTPUT_HASH;

// ----------------------------------------------------------------------------
// Indicizzazione delle feature HalfKP
// ----------------------------------------------------------------------------

/// Orienta una casella secondo la prospettiva: per il nero la scacchiera
/// viene ruotata di 180° (XOR con 63 flippa contemporaneamente i 3 bit di
/// rank e i 3 bit di file, dato che casella = rank*8+file). Per il bianco è
/// un no-op. Corrisponde esattamente a `orient()` in half_kp.cpp.
#[inline(always)]
fn orient(perspective_black: bool, sq: usize) -> usize {
    sq ^ (if perspective_black { 63 } else { 0 })
}

/// Offset base PS_W_<pt>/PS_B_<pt> per un tipo di pezzo (1=Pedone..5=Regina,
/// il Re non è mai passato qui) visto da una data prospettiva: "amico" (stesso
/// colore della prospettiva) usa lo slot W, "nemico" lo slot B. Corrisponde
/// alla tabella `kpp_board_index` di evaluate_nnue.cpp, ristretta ai soli
/// pezzi non-Re (gli unici per cui HalfKP genera feature).
#[inline(always)]
fn ps_base(piece_type: usize, is_friend: bool) -> u32 {
    let slot = 2 * (piece_type as u32 - 1) + if is_friend { 0 } else { 1 };
    1 + slot * SQUARE_NB as u32
}

/// Indice di riga (0..HALFKP_INPUT_DIMENSIONS) nella matrice dei pesi del
/// feature transformer per la feature (prospettiva, casella, tipo di pezzo,
/// amico/nemico, casella del re associato). Corrisponde a `make_index()` in
/// half_kp.cpp: orient(perspective,s) + kpp_board_index[pc][perspective] +
/// PS_END * ksq, con `ksq` già orientato per la stessa prospettiva.
#[inline(always)]
fn feature_index(perspective_black: bool, sq: usize, piece_type: usize, is_friend: bool, ksq_oriented: usize) -> usize {
    orient(perspective_black, sq) + ps_base(piece_type, is_friend) as usize + PS_END as usize * ksq_oriented
}

// ----------------------------------------------------------------------------
// Accumulatore incrementale: DUE metà (una per prospettiva), pre-ClippedReLU
// ----------------------------------------------------------------------------
//
// A differenza della rete "piatta" precedente (una sola prospettiva, feature
// assolute per colore), HalfKP richiede un accumulatore per la prospettiva
// bianca E uno per la prospettiva nera, perché ciascuno usa un king-bucket
// diverso (il proprio re) per indicizzare le stesse feature. Ogni volta che
// il proprio re si muove, l'INTERO king-bucket cambia per quella prospettiva:
// tutte le feature attive vanno reindicizzate, quindi quella metà va
// ricalcolata da zero (vedi `refresh_one_perspective`), mentre l'altra metà
// (il cui re non si è mosso) resta aggiornabile in modo incrementale.
//
// Somma mantenuta in i32 (non i16 con saturating_add come in una prima bozza
// abbandonata): la saturazione non è invertibile, e qui serve che
// add_piece/remove_piece siano l'uno l'esatto inverso dell'altro in ogni
// caso, indipendentemente dall'ampiezza dei pesi caricati.
#[derive(Clone, Copy, Debug)]
pub struct Accumulator {
    pub white: [i32; L1_SIZE],
    pub black: [i32; L1_SIZE],
}

impl Accumulator {
    pub const fn zero() -> Self {
        Accumulator { white: [0i32; L1_SIZE], black: [0i32; L1_SIZE] }
    }
}

impl Default for Accumulator {
    fn default() -> Self { Self::zero() }
}

pub struct LunaNNUE {
    ft_bias: [i16; L1_SIZE],
    /// weights_[feature_index * L1_SIZE + j]: una riga di L1_SIZE pesi per
    /// ciascuna delle HALFKP_INPUT_DIMENSIONS feature. ~21 MB: deve vivere
    /// sull'heap (Vec), non sullo stack né come array embedded nel binario.
    ft_weight: Vec<i16>,
    l1_bias: [i32; L2_SIZE],
    l1_weight: [i8; L2_SIZE * L1_SIZE * 2],
    l2_bias: [i32; L3_SIZE],
    l2_weight: [i8; L3_SIZE * L2_SIZE],
    out_bias: i32,
    out_weight: [i8; L3_SIZE],
}

/// Cursore di lettura little-endian su un buffer in memoria: evita di tirare
/// dentro una dipendenza esterna (byteorder) solo per questo parser una
/// tantum, eseguito al più una volta per avvio del motore.
struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(buf: &'a [u8]) -> Self { Reader { buf, pos: 0 } }
    fn remaining(&self) -> usize { self.buf.len() - self.pos }

    fn read_u32(&mut self) -> Option<u32> {
        let b = self.buf.get(self.pos..self.pos + 4)?;
        self.pos += 4;
        Some(u32::from_le_bytes(b.try_into().unwrap()))
    }
    fn read_i32(&mut self) -> Option<i32> {
        let b = self.buf.get(self.pos..self.pos + 4)?;
        self.pos += 4;
        Some(i32::from_le_bytes(b.try_into().unwrap()))
    }
    fn read_i16(&mut self) -> Option<i16> {
        let b = self.buf.get(self.pos..self.pos + 2)?;
        self.pos += 2;
        Some(i16::from_le_bytes(b.try_into().unwrap()))
    }
    fn read_i8(&mut self) -> Option<i8> {
        let b = *self.buf.get(self.pos)?;
        self.pos += 1;
        Some(b as i8)
    }
    fn skip(&mut self, n: usize) -> Option<()> {
        if self.remaining() < n { return None; }
        self.pos += n;
        Some(())
    }
}

impl LunaNNUE {
    pub fn load(path: &str) -> Option<Self> {
        let mut file = File::open(path).ok()?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).ok()?;
        Self::parse(&buffer)
    }

    fn parse(buffer: &[u8]) -> Option<Self> {
        let mut r = Reader::new(buffer);

        let version = r.read_u32()?;
        if version != SF_NNUE_VERSION {
            println!(
                "⚠️ NNUE: versione file 0x{:08X} non riconosciuta (attesa 0x{:08X}, formato HalfKP classico 256x2-32-32-1). File ignorato.",
                version, SF_NNUE_VERSION
            );
            return None;
        }
        let top_hash = r.read_u32()?;
        let desc_len = r.read_u32()? as usize;
        r.skip(desc_len)?;

        // --- VALIDAZIONE STRUTTURALE (il vero gate) ---
        // Indipendentemente da qualunque hash: per l'architettura HalfKP
        // 256x2-32-32-1 il numero di byte rimanenti nel file è fissato
        // esattamente da queste dimensioni. Un file di un'architettura
        // diversa (es. le reti moderne HalfKAv2_hm, molto più grandi e con
        // pesi compressi) non potrà mai avere esattamente questa lunghezza:
        // viene rifiutato qui, PRIMA di allocare i ~21 MB della tabella pesi
        // del feature transformer.
        let ft_bytes = 4 + L1_SIZE * 2 + HALFKP_INPUT_DIMENSIONS * L1_SIZE * 2;
        let net_bytes = 4
            + (L2_SIZE * 4 + L2_SIZE * (L1_SIZE * 2))
            + (L3_SIZE * 4 + L3_SIZE * L2_SIZE)
            + (4 + L3_SIZE);
        let expected = ft_bytes + net_bytes;
        if r.remaining() != expected {
            println!(
                "⚠️ NNUE: dimensione inattesa ({} byte rimanenti, attesi {}): non è una rete HalfKP 256x2-32-32-1 compatibile. File ignorato.",
                r.remaining(), expected
            );
            return None;
        }

        if top_hash != TOP_HASH {
            println!(
                "info string NNUE: hash architettura 0x{:08X} diverso da quello calcolato 0x{:08X} (continuo comunque, il controllo di dimensione ha già validato il layout)",
                top_hash, TOP_HASH
            );
        }

        let ft_hash = r.read_u32()?;
        if ft_hash != FT_HASH {
            println!("info string NNUE: hash feature transformer inatteso (continuo comunque)");
        }

        let mut ft_bias = [0i16; L1_SIZE];
        for b in ft_bias.iter_mut() { *b = r.read_i16()?; }

        let mut ft_weight = vec![0i16; HALFKP_INPUT_DIMENSIONS * L1_SIZE];
        for w in ft_weight.iter_mut() { *w = r.read_i16()?; }

        let net_hash = r.read_u32()?;
        if net_hash != OUTPUT_HASH {
            println!("info string NNUE: hash rete inatteso (continuo comunque)");
        }

        let mut l1_bias = [0i32; L2_SIZE];
        for b in l1_bias.iter_mut() { *b = r.read_i32()?; }
        let mut l1_weight = [0i8; L2_SIZE * L1_SIZE * 2];
        for w in l1_weight.iter_mut() { *w = r.read_i8()?; }

        let mut l2_bias = [0i32; L3_SIZE];
        for b in l2_bias.iter_mut() { *b = r.read_i32()?; }
        let mut l2_weight = [0i8; L3_SIZE * L2_SIZE];
        for w in l2_weight.iter_mut() { *w = r.read_i8()?; }

        let out_bias = r.read_i32()?;
        let mut out_weight = [0i8; L3_SIZE];
        for w in out_weight.iter_mut() { *w = r.read_i8()?; }

        if r.remaining() != 0 {
            println!("⚠️ NNUE: {} byte non consumati a fine file (formato inatteso). File ignorato.", r.remaining());
            return None;
        }

        println!("✅ NNUE: rete HalfKP 256x2-32-32-1 caricata ({} feature di input).", HALFKP_INPUT_DIMENSIONS);

        Some(LunaNNUE { ft_bias, ft_weight, l1_bias, l1_weight, l2_bias, l2_weight, out_bias, out_weight })
    }

    #[inline(always)]
    fn add_row(&self, half: &mut [i32; L1_SIZE], row: usize) {
        let off = row * L1_SIZE;
        for i in 0..L1_SIZE {
            half[i] += self.ft_weight[off + i] as i32;
        }
    }
    #[inline(always)]
    fn sub_row(&self, half: &mut [i32; L1_SIZE], row: usize) {
        let off = row * L1_SIZE;
        for i in 0..L1_SIZE {
            half[i] -= self.ft_weight[off + i] as i32;
        }
    }

    /// Ricalcola da zero la metà dell'accumulatore relativa a UNA sola
    /// prospettiva (`perspective_white`), a partire dal bias e da tutti i
    /// pezzi non-Re presenti sulla scacchiera. Va usata quando il re di
    /// quella prospettiva si è appena mosso (l'intero king-bucket cambia,
    /// nessuna feature attiva resta valida) o per l'inizializzazione di una
    /// nuova posizione. `piece_of` deve restituire, per ogni casella
    /// occupata da un pezzo non-Re, la coppia (colore_bianco: bool,
    /// piece_type 1..=5).
    pub fn refresh_one_perspective(&self, half: &mut [i32; L1_SIZE], board: &Scacchiera, perspective_white: bool) {
        for i in 0..L1_SIZE { half[i] = self.ft_bias[i] as i32; }

        let perspective_black = !perspective_white;
        let own_king_sq = king_square(board, perspective_white);
        let ksq = orient(perspective_black, own_king_sq);

        // Pezzo = indice Luna 0..=4 (Pedone..Regina); il Re (indice 5) è
        // escluso dal range stesso del ciclo, non con un controllo esplicito:
        // in HalfKP i re non sono mai feature, solo selettori di king-bucket.
        for p_idx in 0..5 {
            let piece_type = p_idx + 1;
            let mut bb_w = board.pezzi[p_idx] & board.colori[0];
            while bb_w != 0 {
                let sq = bb_w.trailing_zeros() as usize;
                let is_friend = perspective_white; // pezzo bianco, prospettiva bianca => amico
                self.add_row(half, feature_index(perspective_black, sq, piece_type, is_friend, ksq));
                bb_w &= bb_w - 1;
            }
            let mut bb_b = board.pezzi[p_idx] & board.colori[1];
            while bb_b != 0 {
                let sq = bb_b.trailing_zeros() as usize;
                let is_friend = !perspective_white; // pezzo nero, prospettiva bianca => nemico
                self.add_row(half, feature_index(perspective_black, sq, piece_type, is_friend, ksq));
                bb_b &= bb_b - 1;
            }
        }
    }

    /// Ricalcola entrambe le metà da zero. Da usare una sola volta dopo aver
    /// impostato una nuova posizione (nuova partita, FEN, comando UCI
    /// "position"); da lì in avanti board.rs mantiene l'accumulatore con
    /// `add_piece`/`remove_piece` e `refresh_one_perspective` mirato.
    pub fn refresh(&self, board: &Scacchiera) -> Accumulator {
        let mut acc = Accumulator::zero();
        self.refresh_one_perspective(&mut acc.white, board, true);
        self.refresh_one_perspective(&mut acc.black, board, false);
        acc
    }

    /// Aggiorna INCREMENTALMENTE entrambe le prospettive per un pezzo che
    /// compare sulla casella `sq` (colore `color_white`, tipo Luna
    /// `piece_type_luna` 0..=5). Se `piece_type_luna == 5` (Re) la chiamata è
    /// un no-op: il Re non è mai una feature tracciata in HalfKP, il suo
    /// movimento va gestito da board.rs con `refresh_one_perspective` sulla
    /// propria prospettiva, non con questa funzione. `white_ksq`/`black_ksq`
    /// devono essere le caselle dei due re PRIMA delle mutazioni di questa
    /// mossa (per la prospettiva il cui re non si muove in questa mossa,
    /// sono anche le caselle corrette dopo; per l'altra, il valore passato è
    /// irrilevante perché quella metà verrà comunque ricalcolata da zero).
    #[inline]
    pub fn add_piece(&self, acc: &mut Accumulator, color_white: bool, piece_type_luna: usize, sq: usize, white_ksq: usize, black_ksq: usize) {
        if piece_type_luna >= 5 { return; }
        let piece_type = piece_type_luna + 1;
        self.add_row(&mut acc.white, feature_index(false, sq, piece_type, color_white, orient(false, white_ksq)));
        self.add_row(&mut acc.black, feature_index(true, sq, piece_type, !color_white, orient(true, black_ksq)));
    }

    /// Esatto inverso di `add_piece`: da chiamare quando un pezzo (non Re)
    /// SPARISCE dalla casella `sq`.
    #[inline]
    pub fn remove_piece(&self, acc: &mut Accumulator, color_white: bool, piece_type_luna: usize, sq: usize, white_ksq: usize, black_ksq: usize) {
        if piece_type_luna >= 5 { return; }
        let piece_type = piece_type_luna + 1;
        self.sub_row(&mut acc.white, feature_index(false, sq, piece_type, color_white, orient(false, white_ksq)));
        self.sub_row(&mut acc.black, feature_index(true, sq, piece_type, !color_white, orient(true, black_ksq)));
    }

    /// Layer 1+2 (affini, int8, con ClippedReLU e shift di scala) e layer di
    /// uscita (affine, senza ClippedReLU) a partire da un accumulatore già
    /// aggiornato. `side_to_move_white` determina l'ordine di concatenazione
    /// (prospettiva di chi deve muovere prima, dell'avversario dopo): è
    /// esattamente il modo in cui la rete è stata allenata, ed è anche ciò
    /// che rende il risultato già "firmato" correttamente per la convenzione
    /// negamax di search.rs, senza bisogno di un flip di segno finale come
    /// nella vecchia rete a singola prospettiva.
    pub fn evaluate_from_accumulator(&self, acc: &Accumulator, side_to_move_white: bool) -> i32 {
        let (us, them) = if side_to_move_white { (&acc.white, &acc.black) } else { (&acc.black, &acc.white) };

        // --- Trasformazione feature: clamp diretto a [0,127], NESSUNO shift ---
        // (la ClippedReLU del feature transformer opera sulla somma int16
        // grezza, a differenza di quella dei layer nascosti più sotto).
        let mut l1_in = [0u8; L1_SIZE * 2];
        for i in 0..L1_SIZE { l1_in[i] = us[i].clamp(0, 127) as u8; }
        for i in 0..L1_SIZE { l1_in[L1_SIZE + i] = them[i].clamp(0, 127) as u8; }

        // --- Hidden layer 1: 512 -> 32, poi ClippedReLU con shift di scala ---
        let mut h1 = [0u8; L2_SIZE];
        for o in 0..L2_SIZE {
            let mut sum = self.l1_bias[o];
            let row_off = o * L1_SIZE * 2;
            for j in 0..L1_SIZE * 2 {
                sum += l1_in[j] as i32 * self.l1_weight[row_off + j] as i32;
            }
            h1[o] = (sum >> WEIGHT_SCALE_BITS).clamp(0, 127) as u8;
        }

        // --- Hidden layer 2: 32 -> 32, poi ClippedReLU con shift di scala ---
        let mut h2 = [0u8; L3_SIZE];
        for o in 0..L3_SIZE {
            let mut sum = self.l2_bias[o];
            let row_off = o * L2_SIZE;
            for j in 0..L2_SIZE {
                sum += h1[j] as i32 * self.l2_weight[row_off + j] as i32;
            }
            h2[o] = (sum >> WEIGHT_SCALE_BITS).clamp(0, 127) as u8;
        }

        // --- Layer di uscita: 32 -> 1, NESSUNA ClippedReLU dopo ---
        let mut out = self.out_bias;
        for j in 0..L3_SIZE {
            out += h2[j] as i32 * self.out_weight[j] as i32;
        }

        // --- Conversione finale in centipawn (unico punto, FV_SCALE=16) ---
        (out / FV_SCALE).clamp(-NNUE_EVAL_CLAMP, NNUE_EVAL_CLAMP)
    }

    /// Valutazione "da zero" (refresh completo + forward pass), per usi
    /// occasionali fuori dal percorso caldo (comando UCI "eval" manuale). NON
    /// va chiamata nel loop di negamax/quiescence: lì è disponibile
    /// l'accumulatore incrementale mantenuto in `Scacchiera::nnue_acc`.
    pub fn evaluate(&self, board: &Scacchiera) -> i32 {
        let acc = self.refresh(board);
        let side_to_move_white = board.turno == crate::board::Colore::Bianco;
        self.evaluate_from_accumulator(&acc, side_to_move_white)
    }
}

/// Casella del re bianco (`perspective_white=true`) o nero.
#[inline(always)]
fn king_square(board: &Scacchiera, perspective_white: bool) -> usize {
    let color_idx = if perspective_white { 0 } else { 1 };
    (board.pezzi[5] & board.colori[color_idx]).trailing_zeros() as usize
}
