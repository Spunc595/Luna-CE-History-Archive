use crate::board::{Scacchiera, Pezzo, Colore, Casella, Mossa, MoveFlag, Bitboard};
use crate::attacks::{pawn_attacks, knight_attacks, king_attacks, bishop_attacks, rook_attacks, queen_attacks};
use std::boxed::Box;

// ============================================
// STRUTTURE PER L'ORDINAMENTO (Heuristics)
// ============================================

pub struct HistoryHeuristic { table: Box<[[i32; 64]; 64]> }
impl Default for HistoryHeuristic { fn default() -> Self { Self { table: Box::new([[0; 64]; 64]) } } }
impl HistoryHeuristic {
    pub fn new() -> Self { Self::default() }
    pub fn increment(&mut self, m: &Mossa, d: i32) {
        let da = m.da().indice();
        let a = m.a().indice();
        // Incremento quadratico: premia molto le mosse buone a grandi profondità
        // Limitiamo a 2000 per evitare valori troppo alti che superano le catture
        let bonus = (d * d).min(400); 
        self.table[da][a] = self.table[da][a].saturating_add(bonus).min(20000);
    }
    pub fn get(&self, m: &Mossa) -> i32 { self.table[m.da().indice()][m.a().indice()] }
    pub fn clear(&mut self) { for r in self.table.iter_mut() { r.fill(0); } }
}

pub struct KillerMoves { killers: Vec<[u16; 2]> }
impl KillerMoves {
    pub fn new(max_depth: usize) -> Self { Self { killers: vec![[0; 2]; max_depth + 1] } }
    pub fn add(&mut self, m: &Mossa, ply: usize) {
        if ply < self.killers.len() {
            let val = m.to_u16();
            // Salva solo se è diversa dalla killer primaria attuale
            if self.killers[ply][0] != val {
                self.killers[ply][1] = self.killers[ply][0]; // Shift
                self.killers[ply][0] = val; // Insert
            }
        }
    }
    pub fn is_killer(&self, m: &Mossa, ply: usize) -> bool {
        if ply >= self.killers.len() { return false; }
        let val = m.to_u16();
        self.killers[ply][0] == val || self.killers[ply][1] == val
    }
    pub fn clear(&mut self) { for k in &mut self.killers { *k = [0; 2]; } }
}

pub struct MossaConInfo { pub mossa: Mossa, pub score: i32 }
pub struct MoveOrderer { pub history: HistoryHeuristic, pub killers: KillerMoves }

impl MoveOrderer {
    pub fn new(depth: usize) -> Self { Self { history: HistoryHeuristic::new(), killers: KillerMoves::new(depth) } }

    pub fn score_moves(&self, mosse: &mut [MossaConInfo], scacchiera: &Scacchiera, tt_move: Option<&Mossa>, ply: usize) {
        for info in mosse.iter_mut() {
            let m = &info.mossa;
            
            // 1. Mossa Hash (Transposition Table) - Priorità Assoluta
            if let Some(tt) = tt_move {
                if m.to_u16() == tt.to_u16() {
                    info.score = 2_000_000;
                    continue;
                }
            }

            // 2. Catture (MVV-LVA: Most Valuable Victim - Least Valuable Attacker)
            if m.flag().is_capture() {
                let victim = scacchiera.pezzo_su_casella(m.a())
                    .map_or(0, |(p, _): (Pezzo, Colore)| p.valore());
                let attacker = scacchiera.pezzo_su_casella(m.da())
                    .map_or(0, |(p, _): (Pezzo, Colore)| p.valore());
                
                // MVV-LVA base + offset alto per stare sopra alle Killer
                info.score = 1_000_000 + (victim * 10 - attacker);
            } 
            // 3. Killer Moves
            else if self.killers.is_killer(m, ply) {
                info.score = 900_000;
            } 
            // 4. History Heuristic
            else {
                info.score = self.history.get(m);
            }
        }
    }
}

// ============================================
// GENERAZIONE MOSSE LEGALI (Public API)
// ============================================

pub fn genera_mosse_legali(scacchiera: &mut Scacchiera) -> Vec<Mossa> {
    let mut mosse = Vec::with_capacity(64);
    genera_tutte_mosse_pseudo_legali(scacchiera, &mut mosse);
    
    let colore = scacchiera.turno(); 
    
    // Filtro in-place: eseguiamo e verifichiamo lo scacco
    let mut i = 0;
    while i < mosse.len() {
        if scacchiera.esegui_mossa(&mosse[i]) {
            let illegal = scacchiera.re_in_scacco(colore);
            scacchiera.annulla_mossa();
            if illegal {
                mosse.swap_remove(i);
            } else {
                i += 1;
            }
        } else {
            // Se esegui_mossa ritorna false (es. errore interno), rimuovi
            mosse.swap_remove(i);
        }
    }
    mosse
}

pub fn genera_e_filtra_mosse_ordinate(scacchiera: &mut Scacchiera, orderer: &MoveOrderer, tt: Option<&Mossa>, ply: usize) -> Vec<Mossa> {
    // Genera legali
    let mosse = genera_mosse_legali(scacchiera);
    
    // Assegna punteggi
    let mut info: Vec<MossaConInfo> = mosse.into_iter()
        .map(|m| MossaConInfo { mossa: m, score: 0 })
        .collect();
    
    orderer.score_moves(&mut info, scacchiera, tt, ply);
    
    // Ordina (unstable è più veloce)
    info.sort_unstable_by(|a, b| b.score.cmp(&a.score));
    
    // Ritorna solo le mosse
    info.into_iter().map(|i| i.mossa).collect()
}

// ============================================
// SEE (Static Exchange Evaluation)
// ============================================



pub fn see(s: &Scacchiera, m: Mossa) -> i32 {
    let mut gain = Vec::with_capacity(32);
    let mut d = 0;
    
    let from = m.da().indice();
    let to = m.a().indice();
    
    // Identifichiamo il pezzo che attacca e quello che viene mangiato
    let attacker_piece = if let Some((p, _)) = s.pezzo_su_casella(m.da()) { p } else { return 0; };
    let target_piece = s.pezzo_su_casella(m.a()).map(|(p, _)| p);
    
    let mut val_victim = target_piece.map_or(0, |p| p.valore());
    
    // Caso speciale: En Passant (la vittima è un pedone, anche se la casella 'a' è vuota)
    if m.flag() == MoveFlag::EnPassant { 
        val_victim = 100; 
    } 
    
    // Caso speciale: Promozione (il valore dell'attaccante cambia dopo la mossa)
    let mut val_attacker = attacker_piece.valore();
    if let Some(p) = m.promozione() {
        // Guadagniamo il valore del pezzo promosso meno il pedone sacrificato
        val_attacker += p.valore() - 100; 
    }
    
    gain.push(val_victim);
    
    // Simuliamo la sequenza
    let mut attacker_val_for_loop = val_attacker;
    let mut side = s.turno().opposto(); // Dopo la prima mossa, tocca all'avversario
    let mut occ = s.occupazione();
    
    // Aggiorniamo l'occupazione per la prima mossa
    occ &= !(1u64 << from); 
    occ |= 1u64 << to; 
    
    loop {
        d += 1;
        // Il guadagno di questo step è: Valore Vittima - Valore Attaccante
        gain.push(val_victim - attacker_val_for_loop); 
        
        // Cerchiamo il prossimo attaccante meno prezioso
        // (Nota: Richiede che la funzione get_least_valuable_attacker sia in board.rs)
        if let Some((from_sq, piece)) = s.get_least_valuable_attacker(to, side, occ) {
            occ &= !(1u64 << from_sq); // Rimuovi l'attaccante dalla sua casa
            side = side.opposto();
            val_victim = attacker_val_for_loop; // L'attaccante precedente diventa la vittima
            attacker_val_for_loop = piece.valore();
        } else {
            break; // Nessun altro attaccante
        }
    }
    
    // Negamax back-propagation: risaliamo la catena per vedere chi ha convenienza a mangiare
    while d > 1 {
        d -= 1;
        gain[d-1] = -(-gain[d-1]).max(gain[d]);
    }
    
    gain[0]
}

// ============================================
// GENERAZIONE PSEUDO-LEGALE (Internal)
// ============================================

fn genera_tutte_mosse_pseudo_legali(scacchiera: &Scacchiera, mosse: &mut Vec<Mossa>) {
    let c = scacchiera.turno();
    genera_mosse_pedone(scacchiera, c, mosse);
    genera_mosse_cavallo(scacchiera, c, mosse);
    genera_sliding(scacchiera, c, mosse, Pezzo::Alfiere);
    genera_sliding(scacchiera, c, mosse, Pezzo::Torre);
    genera_sliding(scacchiera, c, mosse, Pezzo::Regina);
    genera_mosse_re(scacchiera, c, mosse);
}

fn genera_mosse_pedone(scacchiera: &Scacchiera, colore: Colore, mosse: &mut Vec<Mossa>) {
    let pedoni = scacchiera.bitboard_pezzo_colore(Pezzo::Pedone, colore);
    let empty = !scacchiera.occupazione();
    let enemy = scacchiera.bitboard_colore(colore.opposto());
    
    let (rank_promo, shift_up, shift_double, start_rank_mask) = if colore == Colore::Bianco {
        (6, 8, 16, 0xFF00u64) // Rank 2 start
    } else {
        (1, -8, -16, 0x00FF000000000000u64) // Rank 7 start
    };

    let mut bb = pedoni;
    while bb != 0 {
        let from_idx = bb.trailing_zeros() as usize; bb &= bb - 1;
        let from = Casella(from_idx);
        let rank = (from_idx / 8) as u8;
        
        // 1. Spinta Singola
        let to_idx = (from_idx as i32 + shift_up) as usize;
        if to_idx < 64 && (empty & (1u64 << to_idx)) != 0 {
            let to = Casella(to_idx);
            if rank == rank_promo {
                add_promotions(mosse, from, to, false);
            } else {
                mosse.push(Mossa::new_with_flag(from, to, None, MoveFlag::None));
                // 2. Spinta Doppia (solo se singola è valida e siamo al rank di partenza)
                if (1u64 << from_idx) & start_rank_mask != 0 {
                    let double_idx = (from_idx as i32 + shift_double) as usize;
                    if (empty & (1u64 << double_idx)) != 0 {
                        mosse.push(Mossa::new_with_flag(from, Casella(double_idx), None, MoveFlag::DoublePawnPush));
                    }
                }
            }
        }

        // 3. Catture (Diagonali)
        let mut captures = pawn_attacks(from_idx, colore) & enemy;
        while captures != 0 {
            let to_idx = captures.trailing_zeros() as usize; captures &= captures - 1;
            let to = Casella(to_idx);
            if rank == rank_promo {
                add_promotions(mosse, from, to, true);
            } else {
                mosse.push(Mossa::new_with_flag(from, to, None, MoveFlag::Capture));
            }
        }

        // 4. En Passant
        if let Some(ep_idx) = scacchiera.en_passant {
            // Controlla se il pedone può catturare la casella EP
            if (pawn_attacks(from_idx, colore) & (1u64 << ep_idx)) != 0 {
                mosse.push(Mossa::new_with_flag(from, Casella(ep_idx), None, MoveFlag::EnPassant));
            }
        }
    }
}

fn add_promotions(mosse: &mut Vec<Mossa>, from: Casella, to: Casella, capture: bool) {
    let flag = if capture { MoveFlag::PromotionCapture } else { MoveFlag::Promotion };
    // Genera tutte e 4 le promozioni possibili
    for p in [Pezzo::Regina, Pezzo::Torre, Pezzo::Alfiere, Pezzo::Cavallo] {
        mosse.push(Mossa::new_with_flag(from, to, Some(p), flag));
    }
}

fn genera_mosse_cavallo(scacchiera: &Scacchiera, colore: Colore, mosse: &mut Vec<Mossa>) {
    let mut bb = scacchiera.bitboard_pezzo_colore(Pezzo::Cavallo, colore);
    while bb != 0 {
        let from = bb.trailing_zeros() as usize; bb &= bb - 1;
        let mut attacks = knight_attacks(from) & !scacchiera.bitboard_colore(colore);
        while attacks != 0 {
            let to = attacks.trailing_zeros() as usize; attacks &= attacks - 1;
            let cap = (scacchiera.bitboard_colore(colore.opposto()) & (1u64 << to)) != 0;
            mosse.push(Mossa::new_with_flag(Casella(from), Casella(to), None, if cap { MoveFlag::Capture } else { MoveFlag::None }));
        }
    }
}

fn genera_sliding(scacchiera: &Scacchiera, colore: Colore, mosse: &mut Vec<Mossa>, pezzo: Pezzo) {
    let mut bb = scacchiera.bitboard_pezzo_colore(pezzo, colore);
    let occ = scacchiera.occupazione();
    let enemy = scacchiera.bitboard_colore(colore.opposto());
    let friends = scacchiera.bitboard_colore(colore);

    while bb != 0 {
        let from = bb.trailing_zeros() as usize; bb &= bb - 1;
        // Usa le funzioni di attacco da attacks.rs
        let attacks = match pezzo {
            Pezzo::Alfiere => bishop_attacks(from, occ),
            Pezzo::Torre => rook_attacks(from, occ),
            Pezzo::Regina => queen_attacks(from, occ),
            _ => 0
        } & !friends;
        
        let mut m_bb = attacks;
        while m_bb != 0 {
            let to = m_bb.trailing_zeros() as usize; m_bb &= m_bb - 1;
            let cap = (enemy & (1u64 << to)) != 0;
            mosse.push(Mossa::new_with_flag(Casella(from), Casella(to), None, if cap { MoveFlag::Capture } else { MoveFlag::None }));
        }
    }
}

fn genera_mosse_re(scacchiera: &Scacchiera, colore: Colore, mosse: &mut Vec<Mossa>) {
    let bb = scacchiera.bitboard_pezzo_colore(Pezzo::Re, colore);
    if bb == 0 { return; }
    let from = bb.trailing_zeros() as usize;
    
    // Mosse normali del Re
    let mut attacks = king_attacks(from) & !scacchiera.bitboard_colore(colore);
    while attacks != 0 {
        let to = attacks.trailing_zeros() as usize; attacks &= attacks - 1;
        let cap = (scacchiera.bitboard_colore(colore.opposto()) & (1u64 << to)) != 0;
        mosse.push(Mossa::new_with_flag(Casella(from), Casella(to), None, if cap { MoveFlag::Capture } else { MoveFlag::None }));
    }

    // Gestione Arrocco
    if !scacchiera.re_in_scacco(colore) {
        let r = scacchiera.diritti_arrocco_struct();
        let occ = scacchiera.occupazione();
        
        // Helper per verificare se una casella è attaccata dal nemico
        let is_att = |sq| scacchiera.casella_attaccata(Casella(sq), colore.opposto());

        if colore == Colore::Bianco {
            // Arrocco Corto (O-O)
            if r.bianco_lato_re && (occ & (1 << 5 | 1 << 6)) == 0 && !is_att(5) && !is_att(6) {
                mosse.push(Mossa::new_with_flag(Casella(4), Casella(6), None, MoveFlag::CastleKingSide));
            }
            // Arrocco Lungo (O-O-O)
            if r.bianco_lato_regina && (occ & (1 << 1 | 1 << 2 | 1 << 3)) == 0 && !is_att(2) && !is_att(3) {
                mosse.push(Mossa::new_with_flag(Casella(4), Casella(2), None, MoveFlag::CastleQueenSide));
            }
        } else {
            // Arrocco Corto Nero
            if r.nero_lato_re && (occ & (1 << 61 | 1 << 62)) == 0 && !is_att(61) && !is_att(62) {
                mosse.push(Mossa::new_with_flag(Casella(60), Casella(62), None, MoveFlag::CastleKingSide));
            }
            // Arrocco Lungo Nero
            if r.nero_lato_regina && (occ & (1 << 57 | 1 << 58 | 1 << 59)) == 0 && !is_att(58) && !is_att(59) {
                mosse.push(Mossa::new_with_flag(Casella(60), Casella(58), None, MoveFlag::CastleQueenSide));
            }
        }
    }
}