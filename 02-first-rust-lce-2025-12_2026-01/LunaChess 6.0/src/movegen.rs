use crate::board::{Scacchiera, Bitboard, Pezzo, Colore, Casella, Mossa, MoveFlag};
use std::default::Default;

// Re-export delle funzioni di attacco per compatibilità
pub use crate::attacks::{
    pawn_attacks, knight_attacks, king_attacks, 
    bishop_attacks, rook_attacks, queen_attacks
};

// Costanti per file
const FILE_A: Bitboard = 0x0101010101010101;
const FILE_H: Bitboard = 0x8080808080808080;

// ===== STRUCTS PER MOVE ORDERING =====

pub struct HistoryHeuristic {
    table: Box<[[i32; 64]; 64]>, // Box per ridurre stack usage
}

impl Default for HistoryHeuristic {
    fn default() -> Self {
        HistoryHeuristic {
            table: Box::new([[0; 64]; 64]),
        }
    }
}

impl HistoryHeuristic {
    pub fn new() -> Self {
        Self::default()
    }
    
    #[inline]
    pub fn increment(&mut self, mossa: &Mossa, depth: i32) {
        let from = mossa.da().indice();
        let to = mossa.a().indice();
        let increment = depth * depth;
        
        self.table[from][to] = self.table[from][to].saturating_add(increment);
        
        // Age the table se necessario
        const MAX_HISTORY: i32 = 1_000_000;
        if self.table[from][to] > MAX_HISTORY {
            for row in self.table.iter_mut() {
                for val in row.iter_mut() {
                    *val /= 2;
                }
            }
        }
    }
    
    #[inline(always)]
    pub fn get(&self, mossa: &Mossa) -> i32 {
        self.table[mossa.da().indice()][mossa.a().indice()]
    }
    
    pub fn clear(&mut self) {
        for row in self.table.iter_mut() {
            for val in row.iter_mut() {
                *val = 0;
            }
        }
    }
}

pub struct KillerMoves {
    killers: Vec<[u32; 2]>, // Usiamo u32 invece di Option<Mossa> per performance
}

impl KillerMoves {
    pub fn new(max_depth: usize) -> Self {
        Self {
            killers: vec![[u32::MAX; 2]; max_depth + 1],
        }
    }
    
    #[inline]
    pub fn add(&mut self, mossa: &Mossa, depth: usize) {
        if depth >= self.killers.len() {
            return;
        }
        
        let mossa_key = mossa.to_u32();
        
        // Non aggiungere mosse duplicate
        if self.killers[depth][0] == mossa_key || self.killers[depth][1] == mossa_key {
            return;
        }
        
        // Shift e inserisci
        self.killers[depth][0] = self.killers[depth][1];
        self.killers[depth][1] = mossa_key;
    }
    
    #[inline(always)]
    pub fn is_killer(&self, mossa: &Mossa, depth: usize) -> bool {
        if depth >= self.killers.len() {
            return false;
        }
        
        let mossa_key = mossa.to_u32();
        self.killers[depth][0] == mossa_key || self.killers[depth][1] == mossa_key
    }
    
    pub fn clear(&mut self) {
        for killers in self.killers.iter_mut() {
            killers[0] = u32::MAX;
            killers[1] = u32::MAX;
        }
    }
}

pub struct CounterMoves {
    table: Box<[[u32; 64]; 12]>, // 6 pezzi × 2 colori
}

impl Default for CounterMoves {
    fn default() -> Self {
        CounterMoves {
            table: Box::new([[u32::MAX; 64]; 12]),
        }
    }
}

impl CounterMoves {
    pub fn new() -> Self {
        Self::default()
    }
    
    #[inline]
    pub fn add(&mut self, prev_move: &Mossa, counter_move: &Mossa, color: Colore, piece: Pezzo) {
        let piece_idx = Self::piece_index(piece, color);
        let to_square = prev_move.a().indice();
        if piece_idx < 12 && to_square < 64 {
            self.table[piece_idx][to_square] = counter_move.to_u32();
        }
    }
    
    #[inline(always)]
    pub fn get(&self, prev_move: &Mossa, color: Colore, piece: Pezzo) -> Option<Mossa> {
        let piece_idx = Self::piece_index(piece, color);
        let to_square = prev_move.a().indice();
        if piece_idx < 12 && to_square < 64 {
            let key = self.table[piece_idx][to_square];
            if key != u32::MAX {
                return Mossa::from_u32(key);
            }
        }
        None
    }
    
    #[inline(always)]
    fn piece_index(piece: Pezzo, color: Colore) -> usize {
        piece.indice() + color.indice() * 6
    }
    
    pub fn clear(&mut self) {
        for row in self.table.iter_mut() {
            for val in row.iter_mut() {
                *val = u32::MAX;
            }
        }
    }
}

// ===== STRUCT PER CONTENERE INFO MOSSA (OTTIMIZZATA) =====

#[derive(Clone, Copy)]
pub struct MossaConInfo {
    pub mossa: Mossa,
    pub score: i32,
    pub flags: u8, // Bit flags: bit0=is_capture, bit1=is_promotion, bit2-7=attacker_type
}

impl MossaConInfo {
    pub fn new(mossa: Mossa) -> Self {
        Self {
            mossa,
            score: 0,
            flags: 0,
        }
    }
    
    #[inline(always)]
    pub fn is_capture(&self) -> bool {
        (self.flags & 0x01) != 0
    }
    
    #[inline(always)]
    pub fn is_promotion(&self) -> bool {
        (self.flags & 0x02) != 0
    }
    
    #[inline(always)]
    pub fn attacker_type(&self) -> u8 {
        (self.flags >> 2) & 0x3F
    }
    
    pub fn set_info(&mut self, scacchiera: &Scacchiera) {
        let mossa = &self.mossa;
        
        // Determina se è una cattura
        if let Some((_, victim_color)) = scacchiera.pezzo_su_casella(mossa.a()) {
            if victim_color != scacchiera.colore_attivo() {
                self.flags |= 0x01; // Set capture flag
            }
        }
        
        // Check en passant - CORRETTO: scacchiera.en_passant() restituisce Option<usize>
        if let Some(en_passant_idx) = scacchiera.en_passant() {
            if let Some(en_passant_sq) = Casella::da_indice(en_passant_idx) {
                if mossa.a() == en_passant_sq {
                    self.flags |= 0x01; // En passant è una cattura
                }
            }
        }
        
        // Check promozione
        if mossa.promozione().is_some() {
            self.flags |= 0x02;
        }
        
        // Set attacker type
        if let Some((piece, _)) = scacchiera.pezzo_su_casella(mossa.da()) {
            self.flags |= (piece.indice() as u8) << 2;
        }
    }
}

// ===== MOVE ORDERER OTTIMIZZATO =====

pub struct MoveOrderer {
    pub history: HistoryHeuristic,
    pub killers: KillerMoves,
    pub countermoves: CounterMoves,
}

impl MoveOrderer {
    pub fn new(max_depth: usize) -> Self {
        Self {
            history: HistoryHeuristic::new(),
            killers: KillerMoves::new(max_depth),
            countermoves: CounterMoves::new(),
        }
    }
    
    #[inline]
    pub fn order_moves(&self, mosse: &mut [MossaConInfo], tt_move: Option<&Mossa>, 
                       _depth: i32, ply: usize) {
        for mossa_info in mosse.iter_mut() {
            let mut score;
            
            // 1. TT move
            if let Some(tt) = tt_move {
                if mossa_info.mossa == *tt {
                    score = 10_000_000;
                    mossa_info.score = score;
                    continue;
                }
            }
            
            // 2. Catture: MVV-LVA
            if mossa_info.is_capture() {
                let victim_value = estimate_victim_value(&mossa_info.mossa);
                let attacker_type = mossa_info.attacker_type();
                let attacker_value = piece_value_from_type(attacker_type);
                score = 9_000_000 + victim_value * 100 - attacker_value;
                
                if mossa_info.is_promotion() {
                    score += 8_000_000;
                }
            }
            // 3. Killer moves
            else if self.killers.is_killer(&mossa_info.mossa, ply) {
                score = 8_000_000;
            }
            // 4. History heuristic
            else {
                score = self.history.get(&mossa_info.mossa);
            }
            
            // 5. Promozioni (senza cattura)
            let final_score = if mossa_info.is_promotion() && !mossa_info.is_capture() {
                score + 6_000_000
            } else {
                score
            };
            
            mossa_info.score = final_score;
        }
        
        // Sort più veloce per piccoli array
        if mosse.len() <= 8 {
            insertion_sort_moves(mosse);
        } else {
            mosse.sort_unstable_by(|a, b| b.score.cmp(&a.score));
        }
    }
    
    pub fn clear(&mut self) {
        self.history.clear();
        self.killers.clear();
        self.countermoves.clear();
    }
}

// Insertion sort per piccoli array (più veloce)
fn insertion_sort_moves(mosse: &mut [MossaConInfo]) {
    for i in 1..mosse.len() {
        let key = mosse[i];
        let mut j = i;
        
        while j > 0 && mosse[j - 1].score < key.score {
            mosse[j] = mosse[j - 1];
            j -= 1;
        }
        mosse[j] = key;
    }
}

// ===== FUNZIONI DI GENERAZIONE MOSSE OTTIMIZZATE =====

// Versione ottimizzata che genera direttamente MossaConInfo
pub fn genera_e_filtra_mosse_ordinate(scacchiera: &Scacchiera, orderer: &MoveOrderer, 
                                      tt_move: Option<&Mossa>, depth: i32, ply: usize) -> Vec<Mossa> {
    let mut mosse_legali = Vec::with_capacity(40);
    
    // Genera tutte le mosse pseudo-legali
    let mut mosse_pseudo = Vec::with_capacity(45);
    genera_tutte_mosse_pseudo_legali(scacchiera, &mut mosse_pseudo);
    
    // Filtra mosse legali e converte a MossaConInfo
    for mossa in mosse_pseudo {
        if is_mossa_legale_fast(scacchiera, &mossa) {
            let mut info = MossaConInfo::new(mossa);
            info.set_info(scacchiera);
            mosse_legali.push(info);
        }
    }
    
    // Ordina le mosse
    orderer.order_moves(&mut mosse_legali, tt_move, depth, ply);
    
    // Converti di nuovo a Mossa
    mosse_legali.into_iter().map(|info| info.mossa).collect()
}

// Genera tutte le mosse pseudo-legali (senza check legality)
fn genera_tutte_mosse_pseudo_legali(scacchiera: &Scacchiera, mosse: &mut Vec<Mossa>) {
    let colore = scacchiera.turno();
    
    // Genera mosse per ogni tipo di pezzo
    genera_mosse_pedone_fast(scacchiera, colore, mosse);
    genera_mosse_cavallo_fast(scacchiera, colore, mosse);
    genera_mosse_alfiere_fast(scacchiera, colore, mosse);
    genera_mosse_torre_fast(scacchiera, colore, mosse);
    genera_mosse_regina_fast(scacchiera, colore, mosse);
    genera_mosse_re_fast(scacchiera, colore, mosse);
}

// Controllo legale veloce
fn is_mossa_legale_fast(scacchiera: &Scacchiera, mossa: &Mossa) -> bool {
    let mut scacchiera_test = scacchiera.clone();
    esegui_mossa_veloce(&mut scacchiera_test, mossa).is_some()
}

// ===== GENERAZIONE MOSSE VELOCE (PER TIPO) =====

fn genera_mosse_pedone_fast(scacchiera: &Scacchiera, colore: Colore, mosse: &mut Vec<Mossa>) {
    let pedoni = scacchiera.bitboard_colore(colore) & scacchiera.bitboard_pezzo(Pezzo::Pedone);
    if pedoni == 0 { return; }
    
    let direzione = if colore == Colore::Bianco { 1 } else { -1 };
    let occupazione_totale = scacchiera.occupazione();
    let occupazione_avversario = scacchiera.bitboard_colore(colore.opposto());
    let occupazione_colore = scacchiera.bitboard_colore(colore);
    
    let mut pedoni_bb = pedoni;
    while pedoni_bb != 0 {
        let square = pedoni_bb.trailing_zeros() as usize;
        pedoni_bb &= pedoni_bb - 1;
        
        let casella_da = Casella::da_indice(square).unwrap();
        
        // Mossa in avanti
        let target_square = (square as i32 + direzione * 8) as usize;
        if target_square < 64 {
            let target_bit = 1u64 << target_square;
            
            if (occupazione_totale & target_bit) == 0 {
                let target_casella = Casella::da_indice(target_square).unwrap();
                
                // Promozione
                if (colore == Colore::Bianco && target_square >= 56) || 
                   (colore == Colore::Nero && target_square < 8) {
                    mosse.push(Mossa::new_with_flag(casella_da, target_casella, Some(Pezzo::Regina), MoveFlag::Promotion));
                    mosse.push(Mossa::new_with_flag(casella_da, target_casella, Some(Pezzo::Cavallo), MoveFlag::Promotion));
                    mosse.push(Mossa::new_with_flag(casella_da, target_casella, Some(Pezzo::Torre), MoveFlag::Promotion));
                    mosse.push(Mossa::new_with_flag(casella_da, target_casella, Some(Pezzo::Alfiere), MoveFlag::Promotion));
                } else {
                    mosse.push(Mossa::new(casella_da, target_casella, None));
                    
                    // Doppio passo
                    let start_rank = if colore == Colore::Bianco { 1 } else { 6 };
                    if (square / 8) == start_rank {
                        let double_square = (square as i32 + direzione * 16) as usize;
                        if double_square < 64 {
                            let double_bit = 1u64 << double_square;
                            if (occupazione_totale & double_bit) == 0 {
                                let double_casella = Casella::da_indice(double_square).unwrap();
                                mosse.push(Mossa::new(casella_da, double_casella, None));
                            }
                        }
                    }
                }
            }
        }
        
        // Catture
        let attacchi = pawn_attacks(square, colore);
        let catture = attacchi & occupazione_avversario;
        
        let mut catture_bb = catture;
        while catture_bb != 0 {
            let capture_square = catture_bb.trailing_zeros() as usize;
            catture_bb &= catture_bb - 1;
            
            let target_casella = Casella::da_indice(capture_square).unwrap();
            
            // Promozione su cattura
            if (colore == Colore::Bianco && capture_square >= 56) || 
               (colore == Colore::Nero && capture_square < 8) {
                mosse.push(Mossa::new_with_flag(casella_da, target_casella, Some(Pezzo::Regina), MoveFlag::PromotionCapture));
                mosse.push(Mossa::new_with_flag(casella_da, target_casella, Some(Pezzo::Cavallo), MoveFlag::PromotionCapture));
                mosse.push(Mossa::new_with_flag(casella_da, target_casella, Some(Pezzo::Torre), MoveFlag::PromotionCapture));
                mosse.push(Mossa::new_with_flag(casella_da, target_casella, Some(Pezzo::Alfiere), MoveFlag::PromotionCapture));
            } else {
                mosse.push(Mossa::new_with_flag(casella_da, target_casella, None, MoveFlag::Capture));
            }
        }
        
        // En passant - CORRETTO: scacchiera.en_passant() restituisce Option<usize>
        if let Some(en_passant_idx) = scacchiera.en_passant() {
            // en_passant_idx è già usize
            let en_passant_target = (en_passant_idx as i32 - direzione * 8) as usize;
            
            // Controlla se siamo nella stessa riga del pedone che può fare en passant
            let pawn_attacks = pawn_attacks(square, colore);
            let en_passant_bit = 1u64 << en_passant_idx;
            
            if (pawn_attacks & en_passant_bit) != 0 {
                // Verifica che ci sia davvero un pedone avversario nella posizione giusta
                let enemy_pawn_square = en_passant_target;
                let enemy_pawn_bb = 1u64 << enemy_pawn_square;
                
                let enemy_pawns = scacchiera.bitboard_colore(colore.opposto()) & 
                                scacchiera.bitboard_pezzo(Pezzo::Pedone);
                
                if (enemy_pawns & enemy_pawn_bb) != 0 {
                    // Converti en_passant_idx in Casella
                    if let Some(en_passant_sq) = Casella::da_indice(en_passant_idx) {
                        mosse.push(Mossa::new_with_flag(casella_da, en_passant_sq, None, MoveFlag::EnPassant));
                    }
                }
            }
        }
    }
}

fn genera_mosse_cavallo_fast(scacchiera: &Scacchiera, colore: Colore, mosse: &mut Vec<Mossa>) {
    let cavalli = scacchiera.bitboard_colore(colore) & scacchiera.bitboard_pezzo(Pezzo::Cavallo);
    if cavalli == 0 { return; }
    
    let occupazione_colore = scacchiera.bitboard_colore(colore);
    
    let mut cavalli_bb = cavalli;
    while cavalli_bb != 0 {
        let square = cavalli_bb.trailing_zeros() as usize;
        cavalli_bb &= cavalli_bb - 1;
        
        let casella_da = Casella::da_indice(square).unwrap();
        let attacchi = knight_attacks(square);
        let mosse_possibili = attacchi & !occupazione_colore;
        
        let mut target_bb = mosse_possibili;
        while target_bb != 0 {
            let target_square = target_bb.trailing_zeros() as usize;
            target_bb &= target_bb - 1;
            
            let target_casella = Casella::da_indice(target_square).unwrap();
            // Controlla se è una cattura
            let is_capture = (scacchiera.bitboard_colore(colore.opposto()) >> target_square) & 1 != 0;
            let flag = if is_capture { MoveFlag::Capture } else { MoveFlag::None };
            mosse.push(Mossa::new_with_flag(casella_da, target_casella, None, flag));
        }
    }
}

fn genera_mosse_alfiere_fast(scacchiera: &Scacchiera, colore: Colore, mosse: &mut Vec<Mossa>) {
    let alfieri = scacchiera.bitboard_colore(colore) & scacchiera.bitboard_pezzo(Pezzo::Alfiere);
    if alfieri == 0 { return; }
    
    let occupazione_totale = scacchiera.occupazione();
    let occupazione_colore = scacchiera.bitboard_colore(colore);
    
    let mut alfieri_bb = alfieri;
    while alfieri_bb != 0 {
        let square = alfieri_bb.trailing_zeros() as usize;
        alfieri_bb &= alfieri_bb - 1;
        
        let casella_da = Casella::da_indice(square).unwrap();
        let attacchi = bishop_attacks(square, occupazione_totale);
        let mosse_possibili = attacchi & !occupazione_colore;
        
        let mut target_bb = mosse_possibili;
        while target_bb != 0 {
            let target_square = target_bb.trailing_zeros() as usize;
            target_bb &= target_bb - 1;
            
            let target_casella = Casella::da_indice(target_square).unwrap();
            let is_capture = (scacchiera.bitboard_colore(colore.opposto()) >> target_square) & 1 != 0;
            let flag = if is_capture { MoveFlag::Capture } else { MoveFlag::None };
            mosse.push(Mossa::new_with_flag(casella_da, target_casella, None, flag));
        }
    }
}

fn genera_mosse_torre_fast(scacchiera: &Scacchiera, colore: Colore, mosse: &mut Vec<Mossa>) {
    let torri = scacchiera.bitboard_colore(colore) & scacchiera.bitboard_pezzo(Pezzo::Torre);
    if torri == 0 { return; }
    
    let occupazione_totale = scacchiera.occupazione();
    let occupazione_colore = scacchiera.bitboard_colore(colore);
    
    let mut torri_bb = torri;
    while torri_bb != 0 {
        let square = torri_bb.trailing_zeros() as usize;
        torri_bb &= torri_bb - 1;
        
        let casella_da = Casella::da_indice(square).unwrap();
        let attacchi = rook_attacks(square, occupazione_totale);
        let mosse_possibili = attacchi & !occupazione_colore;
        
        let mut target_bb = mosse_possibili;
        while target_bb != 0 {
            let target_square = target_bb.trailing_zeros() as usize;
            target_bb &= target_bb - 1;
            
            let target_casella = Casella::da_indice(target_square).unwrap();
            let is_capture = (scacchiera.bitboard_colore(colore.opposto()) >> target_square) & 1 != 0;
            let flag = if is_capture { MoveFlag::Capture } else { MoveFlag::None };
            mosse.push(Mossa::new_with_flag(casella_da, target_casella, None, flag));
        }
    }
}

fn genera_mosse_regina_fast(scacchiera: &Scacchiera, colore: Colore, mosse: &mut Vec<Mossa>) {
    let regine = scacchiera.bitboard_colore(colore) & scacchiera.bitboard_pezzo(Pezzo::Regina);
    if regine == 0 { return; }
    
    let occupazione_totale = scacchiera.occupazione();
    let occupazione_colore = scacchiera.bitboard_colore(colore);
    
    let mut regine_bb = regine;
    while regine_bb != 0 {
        let square = regine_bb.trailing_zeros() as usize;
        regine_bb &= regine_bb - 1;
        
        let casella_da = Casella::da_indice(square).unwrap();
        let attacchi = queen_attacks(square, occupazione_totale);
        let mosse_possibili = attacchi & !occupazione_colore;
        
        let mut target_bb = mosse_possibili;
        while target_bb != 0 {
            let target_square = target_bb.trailing_zeros() as usize;
            target_bb &= target_bb - 1;
            
            let target_casella = Casella::da_indice(target_square).unwrap();
            let is_capture = (scacchiera.bitboard_colore(colore.opposto()) >> target_square) & 1 != 0;
            let flag = if is_capture { MoveFlag::Capture } else { MoveFlag::None };
            mosse.push(Mossa::new_with_flag(casella_da, target_casella, None, flag));
        }
    }
}

fn genera_mosse_re_fast(scacchiera: &Scacchiera, colore: Colore, mosse: &mut Vec<Mossa>) {
    let re = scacchiera.bitboard_colore(colore) & scacchiera.bitboard_pezzo(Pezzo::Re);
    if re == 0 { return; }
    
    let square = re.trailing_zeros() as usize;
    let casella_da = Casella::da_indice(square).unwrap();
    let occupazione_colore = scacchiera.bitboard_colore(colore);
    
    // Mosse normali del re
    let attacchi = king_attacks(square);
    let mosse_possibili = attacchi & !occupazione_colore;
    
    let mut target_bb = mosse_possibili;
    while target_bb != 0 {
        let target_square = target_bb.trailing_zeros() as usize;
        target_bb &= target_bb - 1;
        
        let target_casella = Casella::da_indice(target_square).unwrap();
        let is_capture = (scacchiera.bitboard_colore(colore.opposto()) >> target_square) & 1 != 0;
        let flag = if is_capture { MoveFlag::Capture } else { MoveFlag::None };
        mosse.push(Mossa::new_with_flag(casella_da, target_casella, None, flag));
    }
    
    // Arrocco (solo se re non è sotto scacco)
    if !scacchiera.sotto_scacco(colore) {
        let diritti = scacchiera.diritti_arrocco();
        let occupazione_totale = scacchiera.occupazione();
        
        if colore == Colore::Bianco {
            // Arrocco corto
            if (diritti & 1) != 0 && // bianco lato re
               (occupazione_totale & (1u64 << 5 | 1u64 << 6)) == 0 &&
               !is_square_attacked(scacchiera, 5, Colore::Nero) &&
               !is_square_attacked(scacchiera, 6, Colore::Nero) {
                let target = Casella::da_indice(6).unwrap();
                mosse.push(Mossa::new_with_flag(casella_da, target, None, MoveFlag::CastleKingSide));
            }
            
            // Arrocco lungo
            if (diritti & 2) != 0 && // bianco lato regina
               (occupazione_totale & (1u64 << 1 | 1u64 << 2 | 1u64 << 3)) == 0 &&
               !is_square_attacked(scacchiera, 2, Colore::Nero) &&
               !is_square_attacked(scacchiera, 3, Colore::Nero) {
                let target = Casella::da_indice(2).unwrap();
                mosse.push(Mossa::new_with_flag(casella_da, target, None, MoveFlag::CastleQueenSide));
            }
        } else {
            // Arrocco nero corto
            if (diritti & 4) != 0 && // nero lato re
               (occupazione_totale & (1u64 << 61 | 1u64 << 62)) == 0 &&
               !is_square_attacked(scacchiera, 61, Colore::Bianco) &&
               !is_square_attacked(scacchiera, 62, Colore::Bianco) {
                let target = Casella::da_indice(62).unwrap();
                mosse.push(Mossa::new_with_flag(casella_da, target, None, MoveFlag::CastleKingSide));
            }
            
            // Arrocco nero lungo
            if (diritti & 8) != 0 && // nero lato regina
               (occupazione_totale & (1u64 << 57 | 1u64 << 58 | 1u64 << 59)) == 0 &&
               !is_square_attacked(scacchiera, 58, Colore::Bianco) &&
               !is_square_attacked(scacchiera, 59, Colore::Bianco) {
                let target = Casella::da_indice(58).unwrap();
                mosse.push(Mossa::new_with_flag(casella_da, target, None, MoveFlag::CastleQueenSide));
            }
        }
    }
}

// Controllo rapido se una casella è attaccata
pub fn is_square_attacked(scacchiera: &Scacchiera, square: usize, attacker_color: Colore) -> bool {
    let _square_bb = 1u64 << square;
    
    // Controlla attacchi di pedoni
    let pawn_attacks = pawn_attacks(square, attacker_color.opposto());
    let pawns = scacchiera.bitboard_colore(attacker_color) & scacchiera.bitboard_pezzo(Pezzo::Pedone);
    if (pawn_attacks & pawns) != 0 {
        return true;
    }
    
    // Controlla attacchi di cavalli
    let knight_attacks = knight_attacks(square);
    let knights = scacchiera.bitboard_colore(attacker_color) & scacchiera.bitboard_pezzo(Pezzo::Cavallo);
    if (knight_attacks & knights) != 0 {
        return true;
    }
    
    // Controlla attacchi di re
    let king_attacks = king_attacks(square);
    let king = scacchiera.bitboard_colore(attacker_color) & scacchiera.bitboard_pezzo(Pezzo::Re);
    if (king_attacks & king) != 0 {
        return true;
    }
    
    // Controlla attacchi di pezzi diagonali (alfiere, regina)
    let occupancy = scacchiera.occupazione();
    let bishop_attacks = bishop_attacks(square, occupancy);
    let bishops = scacchiera.bitboard_colore(attacker_color) & scacchiera.bitboard_pezzo(Pezzo::Alfiere);
    let queens = scacchiera.bitboard_colore(attacker_color) & scacchiera.bitboard_pezzo(Pezzo::Regina);
    if (bishop_attacks & (bishops | queens)) != 0 {
        return true;
    }
    
    // Controlla attacchi di pezzi ortogonali (torre, regina)
    let rook_attacks = rook_attacks(square, occupancy);
    let rooks = scacchiera.bitboard_colore(attacker_color) & scacchiera.bitboard_pezzo(Pezzo::Torre);
    if (rook_attacks & (rooks | queens)) != 0 {
        return true;
    }
    
    false
}

// ===== ESECUZIONE MOSSE VELOCE =====

// Versione ottimizzata di esegui_mossa_completa
pub fn esegui_mossa_veloce(scacchiera: &mut Scacchiera, mossa: &Mossa) -> Option<()> {
    // Usa il metodo esistente di Scacchiera che prende &Mossa
    if scacchiera.esegui_mossa(mossa) {
        Some(())
    } else {
        None
    }
}

// Mantieni la vecchia funzione per compatibilità
pub fn esegui_mossa_completa(scacchiera: &mut Scacchiera, mossa: &Mossa) -> bool {
    scacchiera.esegui_mossa(mossa)
}

// Alias per compatibilità
pub fn esegui_mossa(scacchiera: &mut Scacchiera, mossa: &Mossa) -> bool {
    esegui_mossa_veloce(scacchiera, mossa).is_some()
}

// ===== FUNZIONI HELPER =====

fn estimate_victim_value(_mossa: &Mossa) -> i32 {
    // Stima il valore della vittima (per MVV-LVA)
    // In una implementazione reale, dovresti conoscere il pezzo effettivo
    // Qui usiamo una stima basata sul tipo di mossa
    // Valore di default (pedone)
    100
}

fn piece_value_from_type(piece_type: u8) -> i32 {
    match piece_type {
        0 => 100,   // Pedone
        1 => 320,   // Cavallo
        2 => 330,   // Alfiere
        3 => 500,   // Torre
        4 => 900,   // Regina
        5 => 10000, // Re
        _ => 100,
    }
}

// ===== FUNZIONI PUBLIC PER COMPATIBILITÀ =====

pub fn genera_e_filtra_mosse(scacchiera: &Scacchiera) -> Vec<Mossa> {
    let mut mosse = Vec::with_capacity(40);
    genera_tutte_mosse_pseudo_legali(scacchiera, &mut mosse);
    
    // Filtra mosse legali
    mosse.retain(|m| is_mossa_legale_fast(scacchiera, m));
    mosse
}

pub fn genera_mosse_legali(scacchiera: &Scacchiera) -> Vec<Mossa> {
    genera_e_filtra_mosse(scacchiera)
}

pub fn mosse_legali_count(scacchiera: &Scacchiera) -> usize {
    genera_e_filtra_mosse(scacchiera).len()
}

pub fn ha_mosse_legali(scacchiera: &Scacchiera) -> bool {
    !genera_e_filtra_mosse(scacchiera).is_empty()
}

// Funzione per parse_mossa_da_stringa
pub fn parse_mossa_da_stringa(_scacchiera: &Scacchiera, mossa_str: &str) -> Option<Mossa> {
    Mossa::from_uci(mossa_str)
}

// ===== TEST =====

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_move_generation() {
        let scacchiera = Scacchiera::nuova();
        let mosse = genera_e_filtra_mosse(&scacchiera);
        
        // Posizione iniziale: 20 mosse legali
        assert_eq!(mosse.len(), 20);
    }
    
    #[test]
    fn test_pawn_attacks() {
        // Test attacchi pedone bianco su e2
        let attacks = pawn_attacks(12, Colore::Bianco); // e2
        assert_eq!(attacks, (1u64 << 19) | (1u64 << 21)); // d3 e f3
        
        // Test attacchi pedone nero su e7
        let attacks = pawn_attacks(52, Colore::Nero); // e7
        assert_eq!(attacks, (1u64 << 43) | (1u64 << 45)); // d6 e f6
    }
    
    #[test]
    fn test_mossa_encoding() {
        let mossa = Mossa::new(
            Casella::da_indice(12).unwrap(), // e2
            Casella::da_indice(28).unwrap(), // e4
            None
        );
        
        let encoded = mossa.to_u32();
        let decoded = Mossa::from_u32(encoded).unwrap();
        
        assert_eq!(mossa.da(), decoded.da());
        assert_eq!(mossa.a(), decoded.a());
    }
    
    #[test]
    fn test_is_square_attacked() {
        let scacchiera = Scacchiera::nuova();
        
        // e2 è attaccato da pedone nero su d7 o f7
        assert!(is_square_attacked(&scacchiera, 12, Colore::Nero)); // e2
        
        // e4 non dovrebbe essere attaccato in posizione iniziale
        assert!(!is_square_attacked(&scacchiera, 28, Colore::Nero)); // e4
    }
    
    #[test]
    fn test_history_heuristic() {
        let mut history = HistoryHeuristic::new();
        let mossa = Mossa::new(
            Casella::da_indice(12).unwrap(),
            Casella::da_indice(28).unwrap(),
            None
        );
        
        assert_eq!(history.get(&mossa), 0);
        history.increment(&mossa, 3);
        assert_eq!(history.get(&mossa), 9);
        
        history.clear();
        assert_eq!(history.get(&mossa), 0);
    }
}