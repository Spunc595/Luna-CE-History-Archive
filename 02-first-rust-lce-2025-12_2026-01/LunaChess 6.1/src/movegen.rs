// src/movegen.rs

use crate::board::{Scacchiera, Pezzo, Colore, Casella, Mossa, MoveFlag};
use crate::attacks::{pawn_attacks, knight_attacks, king_attacks};
use crate::attacks::{bishop_attacks, rook_attacks, queen_attacks};
use std::boxed::Box;

// ============================================
// STRUTTURE EURISTICHE (Move Ordering)
// ============================================

pub struct HistoryHeuristic {
    table: Box<[[i32; 64]; 64]>,
}

// Implementazione manuale di Default per evitare stack overflow e errori sui trait
impl Default for HistoryHeuristic {
    fn default() -> Self {
        Self {
            table: Box::new([[0; 64]; 64]),
        }
    }
}

impl HistoryHeuristic {
    pub fn new() -> Self { Self::default() }
    
    pub fn increment(&mut self, m: &Mossa, d: i32) {
        let f = m.da().indice(); 
        let t = m.a().indice();
        // Bonus proporzionale alla profondità al quadrato
        let bonus = d * d;
        self.table[f][t] = self.table[f][t].saturating_add(bonus);
        
        // Aging: dimezza i valori se diventano troppo alti per evitare overflow futuri
        if self.table[f][t] > 100_000_000 {
            for r in self.table.iter_mut() {
                for c in r.iter_mut() {
                    *c /= 2;
                }
            }
        }
    }
    
    pub fn get(&self, m: &Mossa) -> i32 { 
        self.table[m.da().indice()][m.a().indice()] 
    }
    
    pub fn clear(&mut self) { 
        for r in self.table.iter_mut() {
            r.fill(0);
        }
    }
}

pub struct KillerMoves { 
    killers: Vec<[u32; 2]> 
}

impl KillerMoves {
    pub fn new(max_depth: usize) -> Self { 
        Self { killers: vec![[0; 2]; max_depth + 1] } 
    }
    
    pub fn add(&mut self, m: &Mossa, ply: usize) {
        if ply < self.killers.len() {
            let val = m.to_u32();
            // Se la mossa non è già la killer primaria
            if self.killers[ply][0] != val {
                // Sposta la primaria a secondaria e salva la nuova
                self.killers[ply][1] = self.killers[ply][0];
                self.killers[ply][0] = val;
            }
        }
    }
    
    pub fn is_killer(&self, m: &Mossa, ply: usize) -> bool {
        if ply >= self.killers.len() { return false; }
        let val = m.to_u32();
        self.killers[ply][0] == val || self.killers[ply][1] == val
    }
    
    pub fn clear(&mut self) { 
        for k in &mut self.killers { *k = [0; 2]; } 
    }
}

pub struct CounterMoves { 
    table: Box<[[u32; 64]; 12]> 
}

impl Default for CounterMoves {
    fn default() -> Self {
        Self {
            table: Box::new([[0; 64]; 12]), // 12 pezzi (6 bianchi + 6 neri) * 64 caselle
        }
    }
}

impl CounterMoves {
    pub fn new() -> Self { Self::default() }
    
    pub fn add(&mut self, prev_move: &Mossa, current_move: &Mossa, prev_color: Colore, prev_piece: Pezzo) {
        let idx = prev_piece.indice() + prev_color.indice() * 6;
        let dest_sq = prev_move.a().indice();
        self.table[idx][dest_sq] = current_move.to_u32();
    }
    
    pub fn get(&self, prev_move: &Mossa, prev_color: Colore, prev_piece: Pezzo) -> Option<Mossa> {
        let idx = prev_piece.indice() + prev_color.indice() * 6;
        let dest_sq = prev_move.a().indice();
        let val = self.table[idx][dest_sq];
        if val != 0 { Mossa::from_u32(val) } else { None }
    }
    
    pub fn clear(&mut self) { 
        for r in self.table.iter_mut() {
            r.fill(0);
        }
    }
}

#[derive(Clone, Copy)]
pub struct MossaConInfo {
    pub mossa: Mossa,
    pub score: i32,
}

impl MossaConInfo {
    pub fn new(mossa: Mossa, score: i32) -> Self { Self { mossa, score } }
}

pub struct MoveOrderer {
    pub history: HistoryHeuristic,
    pub killers: KillerMoves,
    pub countermoves: CounterMoves,
}

impl MoveOrderer {
    pub fn new(depth: usize) -> Self {
        Self {
            history: HistoryHeuristic::new(),
            killers: KillerMoves::new(depth),
            countermoves: CounterMoves::new(),
        }
    }
    
    pub fn clear(&mut self) { 
        self.history.clear(); 
        self.killers.clear(); 
        self.countermoves.clear(); 
    }
    
    pub fn score_moves(&self, mosse: &mut [MossaConInfo], scacchiera: &Scacchiera, tt_move: Option<&Mossa>, ply: usize) {
        for info in mosse.iter_mut() {
            let m = &info.mossa;
            
            // 1. TT Move (Mossa Hash) - Priorità Massima
            if let Some(tt) = tt_move {
                if m.to_u32() == tt.to_u32() { 
                    info.score = 2_000_000; 
                    continue; 
                }
            }
            
            // 2. Catture (MVV-LVA)
            if m.flag().is_capture() {
                let victim = scacchiera.pezzo_su_casella(m.a()).map_or(Pezzo::Pedone, |(p,_)| p).valore();
                let attacker = scacchiera.pezzo_su_casella(m.da()).map_or(Pezzo::Pedone, |(p,_)| p).valore();
                // Esempio: Pedone(100) mangia Regina(900) -> 1000 + 8900 = 9900
                // Esempio: Regina(900) mangia Pedone(100) -> 1000 + 100 = 1100
                info.score = 1_000_000 + (victim * 10 - attacker);
            } 
            // 3. Killer Moves
            else if self.killers.is_killer(m, ply) {
                info.score = 900_000;
            } 
            // 4. History Heuristic (Mosse quiete storicamente buone)
            else {
                info.score = self.history.get(m);
            }
        }
    }
}

// ============================================
// GENERAZIONE MOSSE
// ============================================

pub fn genera_mosse_legali(scacchiera: &mut Scacchiera) -> Vec<Mossa> {
    let mut mosse = Vec::with_capacity(64);
    
    // 1. Genera tutte le mosse pseudo-legali
    genera_tutte_mosse_pseudo_legali(scacchiera, &mut mosse);
    
    // 2. Filtra quelle illegali (che lasciano il Re sotto scacco)
    let colore = scacchiera.turno; 
    let mut i = 0;
    
    while i < mosse.len() {
        // STRATEGIA CLONE: Creiamo una copia per verificare la legalità.
        // È l'unico modo sicuro se "annulla_mossa" non è implementato perfettamente.
        let mut board_copy = scacchiera.clone();
        
        // Se la mossa può essere eseguita (strutturalmente valida)
        if board_copy.esegui_mossa(&mosse[i]) {
            // Se dopo la mossa il nostro Re è sotto scacco, la mossa è illegale
            if board_copy.re_in_scacco(colore) {
                mosse.swap_remove(i);
            } else {
                // Mossa valida
                i += 1;
            }
        } else {
            // Mossa strutturalmente invalida (es. casella destinazione occupata da amico)
            mosse.swap_remove(i);
        }
    }
    mosse
}

// Funzione helper per il search: genera, ordina e ritorna solo le mosse
pub fn genera_e_filtra_mosse_ordinate(
    scacchiera: &mut Scacchiera, 
    orderer: &MoveOrderer, 
    tt: Option<&Mossa>, 
    _d: i32, 
    ply: usize
) -> Vec<Mossa> {
    // Genera già filtrate per legalità
    let mosse = genera_mosse_legali(scacchiera);
    
    // Converte in MossaConInfo per assegnare punteggi
    let mut info: Vec<MossaConInfo> = mosse.into_iter()
        .map(|m| MossaConInfo::new(m, 0))
        .collect();
        
    orderer.score_moves(&mut info, scacchiera, tt, ply);
    
    // Ordinamento decrescente (punteggio più alto prima)
    info.sort_unstable_by(|a, b| b.score.cmp(&a.score));
    
    // Estrae solo le mosse
    info.into_iter().map(|i| i.mossa).collect()
}

fn genera_tutte_mosse_pseudo_legali(scacchiera: &Scacchiera, mosse: &mut Vec<Mossa>) {
    let colore = scacchiera.turno;
    
    genera_mosse_pedone(scacchiera, colore, mosse);
    genera_mosse_cavallo(scacchiera, colore, mosse);
    genera_mosse_alfiere(scacchiera, colore, mosse);
    genera_mosse_torre(scacchiera, colore, mosse);
    genera_mosse_regina(scacchiera, colore, mosse);
    genera_mosse_re(scacchiera, colore, mosse);
}

// --- PEDONI ---
fn genera_mosse_pedone(scacchiera: &Scacchiera, colore: Colore, mosse: &mut Vec<Mossa>) {
    let pedoni = scacchiera.bitboard_pezzo_colore(Pezzo::Pedone, colore);
    if pedoni == 0 { return; }

    let empty = !scacchiera.occupazione();
    let enemy = scacchiera.bitboard_colore(colore.opposto());
    
    let (rank_promo, shift_up, shift_double) = if colore == Colore::Bianco {
        (6, 8, 16)
    } else {
        (1, -8, -16)
    };

    let start_rank_mask = if colore == Colore::Bianco { 
        0x000000000000FF00 // Rank 2
    } else { 
        0x00FF000000000000 // Rank 7
    };

    let mut bb = pedoni;
    while bb != 0 {
        let from_idx = bb.trailing_zeros() as usize;
        bb &= bb - 1;
        let from = Casella::da_indice(from_idx).unwrap();
        let rank = from.rank();

        // 1. Spinta singola
        let to_idx = (from_idx as i32 + shift_up) as usize;
        
        if to_idx < 64 && (empty & (1u64 << to_idx)) != 0 {
            let to = Casella::da_indice(to_idx).unwrap();
            
            if rank == rank_promo {
                add_promotions(mosse, from, to, false);
            } else {
                mosse.push(Mossa::new(from, to, None));
                
                // 2. Spinta doppia (solo se la singola era libera e siamo al rank di partenza)
                if (1u64 << from_idx) & start_rank_mask != 0 {
                    let double_idx = (from_idx as i32 + shift_double) as usize;
                    if (empty & (1u64 << double_idx)) != 0 {
                        let to_double = Casella::da_indice(double_idx).unwrap();
                        mosse.push(Mossa::new_with_flag(from, to_double, None, MoveFlag::DoublePawnPush));
                    }
                }
            }
        }

        // 3. Catture
        let attacks = pawn_attacks(from_idx, colore);
        let mut captures = attacks & enemy;
        
        while captures != 0 {
            let to_idx = captures.trailing_zeros() as usize;
            captures &= captures - 1;
            let to = Casella::da_indice(to_idx).unwrap();
            
            if rank == rank_promo {
                add_promotions(mosse, from, to, true);
            } else {
                mosse.push(Mossa::new_with_flag(from, to, None, MoveFlag::Capture));
            }
        }

        // 4. En Passant
        if let Some(ep_idx) = scacchiera.en_passant {
            // Verifica se il pedone attacca la casa di en passant
            if (attacks & (1u64 << ep_idx)) != 0 {
                let to = Casella::da_indice(ep_idx).unwrap();
                mosse.push(Mossa::new_with_flag(from, to, None, MoveFlag::EnPassant));
            }
        }
    }
}

fn add_promotions(mosse: &mut Vec<Mossa>, from: Casella, to: Casella, is_capture: bool) {
    let flag = if is_capture { MoveFlag::PromotionCapture } else { MoveFlag::Promotion };
    mosse.push(Mossa::new_with_flag(from, to, Some(Pezzo::Regina), flag));
    mosse.push(Mossa::new_with_flag(from, to, Some(Pezzo::Torre), flag));
    mosse.push(Mossa::new_with_flag(from, to, Some(Pezzo::Alfiere), flag));
    mosse.push(Mossa::new_with_flag(from, to, Some(Pezzo::Cavallo), flag));
}

// --- CAVALLO ---
fn genera_mosse_cavallo(scacchiera: &Scacchiera, colore: Colore, mosse: &mut Vec<Mossa>) {
    let mut bb = scacchiera.bitboard_pezzo_colore(Pezzo::Cavallo, colore);
    let friends = scacchiera.bitboard_colore(colore);
    let enemies = scacchiera.bitboard_colore(colore.opposto());

    while bb != 0 {
        let from_idx = bb.trailing_zeros() as usize;
        bb &= bb - 1;
        let from = Casella::da_indice(from_idx).unwrap();
        
        let attacks = knight_attacks(from_idx) & !friends;
        let mut moves_bb = attacks;
        
        while moves_bb != 0 {
            let to_idx = moves_bb.trailing_zeros() as usize;
            moves_bb &= moves_bb - 1;
            let to = Casella::da_indice(to_idx).unwrap();
            let flag = if (enemies & (1u64 << to_idx)) != 0 { MoveFlag::Capture } else { MoveFlag::None };
            mosse.push(Mossa::new_with_flag(from, to, None, flag));
        }
    }
}

// --- SLIDERS (Alfiere, Torre, Regina) ---
fn genera_mosse_alfiere(scacchiera: &Scacchiera, colore: Colore, mosse: &mut Vec<Mossa>) {
    genera_mosse_sliding(scacchiera, colore, mosse, Pezzo::Alfiere);
}

fn genera_mosse_torre(scacchiera: &Scacchiera, colore: Colore, mosse: &mut Vec<Mossa>) {
    genera_mosse_sliding(scacchiera, colore, mosse, Pezzo::Torre);
}

fn genera_mosse_regina(scacchiera: &Scacchiera, colore: Colore, mosse: &mut Vec<Mossa>) {
    genera_mosse_sliding(scacchiera, colore, mosse, Pezzo::Regina);
}

fn genera_mosse_sliding(scacchiera: &Scacchiera, colore: Colore, mosse: &mut Vec<Mossa>, pezzo: Pezzo) {
    let mut bb = scacchiera.bitboard_pezzo_colore(pezzo, colore);
    let friends = scacchiera.bitboard_colore(colore);
    let enemies = scacchiera.bitboard_colore(colore.opposto());
    let occ = scacchiera.occupazione();

    while bb != 0 {
        let from_idx = bb.trailing_zeros() as usize;
        bb &= bb - 1;
        let from = Casella::da_indice(from_idx).unwrap();

        let attacks = match pezzo {
            Pezzo::Alfiere => bishop_attacks(from_idx, occ),
            Pezzo::Torre => rook_attacks(from_idx, occ),
            Pezzo::Regina => queen_attacks(from_idx, occ),
            _ => 0,
        };

        let moves_bb = attacks & !friends;
        let mut m_bb = moves_bb;
        
        while m_bb != 0 {
            let to_idx = m_bb.trailing_zeros() as usize;
            m_bb &= m_bb - 1;
            let to = Casella::da_indice(to_idx).unwrap();
            let flag = if (enemies & (1u64 << to_idx)) != 0 { MoveFlag::Capture } else { MoveFlag::None };
            mosse.push(Mossa::new_with_flag(from, to, None, flag));
        }
    }
}

// --- RE ---
fn genera_mosse_re(scacchiera: &Scacchiera, colore: Colore, mosse: &mut Vec<Mossa>) {
    let mut bb = scacchiera.bitboard_pezzo_colore(Pezzo::Re, colore);
    if bb == 0 { return; }
    let from_idx = bb.trailing_zeros() as usize;
    let from = Casella::da_indice(from_idx).unwrap();
    
    let friends = scacchiera.bitboard_colore(colore);
    let enemies = scacchiera.bitboard_colore(colore.opposto());

    // 1. Mosse normali
    let attacks = king_attacks(from_idx) & !friends;
    let mut moves_bb = attacks;
    while moves_bb != 0 {
        let to_idx = moves_bb.trailing_zeros() as usize;
        moves_bb &= moves_bb - 1;
        let to = Casella::da_indice(to_idx).unwrap();
        let flag = if (enemies & (1u64 << to_idx)) != 0 { MoveFlag::Capture } else { MoveFlag::None };
        mosse.push(Mossa::new_with_flag(from, to, None, flag));
    }

    // 2. Arrocco
    // Verifichiamo che il re non sia sotto scacco (requisito base per arroccare)
    if !scacchiera.re_in_scacco(colore) {
        let rights = scacchiera.diritti_arrocco_struct();
        let occ = scacchiera.occupazione();
        
        // Helper: controlla che le case siano vuote E non attaccate
        let path_clear = |squares: &[usize]| -> bool {
            for &sq in squares {
                if (occ & (1u64 << sq)) != 0 { return false; } // Occupata
                if is_square_attacked(scacchiera, sq, colore.opposto()) { return false; } // Attaccata
            }
            true
        };
        
        // Helper: controlla solo che le case siano vuote (per la casa "b1/b8" nel queenside)
        let empty_check = |squares: &[usize]| -> bool {
            for &sq in squares {
                if (occ & (1u64 << sq)) != 0 { return false; }
            }
            true
        };

        if colore == Colore::Bianco {
            // Kingside (e1 -> g1). Verifica f1, g1
            if rights.bianco_lato_re && empty_check(&[5, 6]) && path_clear(&[5, 6]) {
                 mosse.push(Mossa::new_with_flag(from, Casella::da_indice(6).unwrap(), None, MoveFlag::CastleKingSide));
            }
            // Queenside (e1 -> c1). Verifica c1, d1 (path clear) E b1 (empty)
            if rights.bianco_lato_regina && empty_check(&[1, 2, 3]) && path_clear(&[2, 3]) {
                 mosse.push(Mossa::new_with_flag(from, Casella::da_indice(2).unwrap(), None, MoveFlag::CastleQueenSide));
            }
        } else {
            // Kingside (e8 -> g8). Verifica f8, g8
            if rights.nero_lato_re && empty_check(&[61, 62]) && path_clear(&[61, 62]) {
                 mosse.push(Mossa::new_with_flag(from, Casella::da_indice(62).unwrap(), None, MoveFlag::CastleKingSide));
            }
            // Queenside (e8 -> c8). Verifica c8, d8 (path clear) E b8 (empty)
            if rights.nero_lato_regina && empty_check(&[57, 58, 59]) && path_clear(&[58, 59]) {
                 mosse.push(Mossa::new_with_flag(from, Casella::da_indice(58).unwrap(), None, MoveFlag::CastleQueenSide));
            }
        }
    }
}

// Wrapper per compatibilità che usa il metodo implementato in board.rs
pub fn is_square_attacked(scacchiera: &Scacchiera, sq: usize, by_color: Colore) -> bool {
    scacchiera.casella_attaccata(Casella::da_indice(sq).unwrap(), by_color)
}