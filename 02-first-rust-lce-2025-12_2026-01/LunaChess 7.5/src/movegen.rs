use crate::board::{Scacchiera, Pezzo, Colore, Casella, Mossa, MoveFlag};
use crate::attacks::{pawn_attacks, knight_attacks, king_attacks};
use crate::attacks::{bishop_attacks, rook_attacks, queen_attacks};
use std::boxed::Box;

pub struct HistoryHeuristic { table: Box<[[i32; 64]; 64]> }
impl Default for HistoryHeuristic { fn default() -> Self { Self { table: Box::new([[0; 64]; 64]) } } }
impl HistoryHeuristic {
    pub fn new() -> Self { Self::default() }
    pub fn increment(&mut self, m: &Mossa, d: i32) {
        let f = m.da().indice(); let t = m.a().indice();
        self.table[f][t] = self.table[f][t].saturating_add(d * d);
        if self.table[f][t] > 100_000_000 { for r in self.table.iter_mut() { for c in r.iter_mut() { *c /= 2; } } }
    }
    pub fn get(&self, m: &Mossa) -> i32 { self.table[m.da().indice()][m.a().indice()] }
    pub fn clear(&mut self) { for r in self.table.iter_mut() { r.fill(0); } }
}

pub struct KillerMoves { killers: Vec<[u32; 2]> }
impl KillerMoves {
    pub fn new(max_depth: usize) -> Self { Self { killers: vec![[0; 2]; max_depth + 1] } }
    pub fn add(&mut self, m: &Mossa, ply: usize) {
        if ply < self.killers.len() {
            let val = m.to_u32();
            if self.killers[ply][0] != val { self.killers[ply][1] = self.killers[ply][0]; self.killers[ply][0] = val; }
        }
    }
    pub fn is_killer(&self, m: &Mossa, ply: usize) -> bool {
        if ply >= self.killers.len() { return false; }
        let val = m.to_u32();
        self.killers[ply][0] == val || self.killers[ply][1] == val
    }
    pub fn clear(&mut self) { for k in &mut self.killers { *k = [0; 2]; } }
}

pub struct CounterMoves { table: Box<[[u32; 64]; 12]> }
impl Default for CounterMoves { fn default() -> Self { Self { table: Box::new([[0; 64]; 12]) } } }
impl CounterMoves {
    pub fn new() -> Self { Self::default() }
    pub fn add(&mut self, prev: &Mossa, curr: &Mossa, pc: Colore, pp: Pezzo) {
        self.table[pp.indice() + pc.indice() * 6][prev.a().indice()] = curr.to_u32();
    }
    pub fn clear(&mut self) { for r in self.table.iter_mut() { r.fill(0); } }
}

#[derive(Clone, Copy)] pub struct MossaConInfo { pub mossa: Mossa, pub score: i32 }
impl MossaConInfo { pub fn new(mossa: Mossa, score: i32) -> Self { Self { mossa, score } } }

pub struct MoveOrderer { pub history: HistoryHeuristic, pub killers: KillerMoves, pub countermoves: CounterMoves }
impl MoveOrderer {
    pub fn new(depth: usize) -> Self { Self { history: HistoryHeuristic::new(), killers: KillerMoves::new(depth), countermoves: CounterMoves::new() } }
    pub fn clear(&mut self) { self.history.clear(); self.killers.clear(); self.countermoves.clear(); }
    pub fn score_moves(&self, mosse: &mut [MossaConInfo], scacchiera: &Scacchiera, tt_move: Option<&Mossa>, ply: usize) {
        for info in mosse.iter_mut() {
            let m = &info.mossa;
            if let Some(tt) = tt_move { if m.to_u32() == tt.to_u32() { info.score = 2_000_000; continue; } }
            if m.flag().is_capture() {
                let victim = scacchiera.pezzo_su_casella(m.a()).map_or(Pezzo::Pedone, |(p,_)| p).valore();
                let attacker = scacchiera.pezzo_su_casella(m.da()).map_or(Pezzo::Pedone, |(p,_)| p).valore();
                info.score = 1_000_000 + (victim * 10 - attacker);
            } else if self.killers.is_killer(m, ply) { info.score = 900_000; }
            else { info.score = self.history.get(m); }
        }
    }
}

pub fn genera_mosse_legali(scacchiera: &mut Scacchiera) -> Vec<Mossa> {
    let mut mosse = Vec::with_capacity(64);
    genera_tutte_mosse_pseudo_legali(scacchiera, &mut mosse);
    let colore = scacchiera.turno; 
    let mut i = 0;
    while i < mosse.len() {
        if scacchiera.esegui_mossa(&mosse[i]) {
            let illegal = scacchiera.re_in_scacco(colore);
            scacchiera.annulla_mossa();
            if illegal { mosse.swap_remove(i); } else { i += 1; }
        } else {
            mosse.swap_remove(i);
        }
    }
    mosse
}

pub fn genera_e_filtra_mosse_ordinate(scacchiera: &mut Scacchiera, orderer: &MoveOrderer, tt: Option<&Mossa>, _d: i32, ply: usize) -> Vec<Mossa> {
    let mosse = genera_mosse_legali(scacchiera);
    let mut info: Vec<MossaConInfo> = mosse.into_iter().map(|m| MossaConInfo::new(m, 0)).collect();
    orderer.score_moves(&mut info, scacchiera, tt, ply);
    info.sort_unstable_by(|a, b| b.score.cmp(&a.score));
    info.into_iter().map(|i| i.mossa).collect()
}

fn genera_tutte_mosse_pseudo_legali(scacchiera: &Scacchiera, mosse: &mut Vec<Mossa>) {
    let c = scacchiera.turno;
    genera_mosse_pedone(scacchiera, c, mosse);
    genera_mosse_cavallo(scacchiera, c, mosse);
    genera_mosse_alfiere(scacchiera, c, mosse);
    genera_mosse_torre(scacchiera, c, mosse);
    genera_mosse_regina(scacchiera, c, mosse);
    genera_mosse_re(scacchiera, c, mosse);
}

fn genera_mosse_pedone(scacchiera: &Scacchiera, colore: Colore, mosse: &mut Vec<Mossa>) {
    let pedoni = scacchiera.bitboard_pezzo_colore(Pezzo::Pedone, colore);
    let empty = !scacchiera.occupazione();
    let enemy = scacchiera.bitboard_colore(colore.opposto());
    let (rank_promo, shift_up, shift_double, start_rank_mask) = if colore == Colore::Bianco { (6, 8, 16, 0xFF00) } else { (1, -8, -16, 0x00FF000000000000) };
    let mut bb = pedoni;
    while bb != 0 {
        let from_idx = bb.trailing_zeros() as usize; bb &= bb - 1;
        let from = Casella::da_indice(from_idx).unwrap();
        let rank = from.rank();
        let to_idx = (from_idx as i32 + shift_up) as usize;
        if to_idx < 64 && (empty & (1u64 << to_idx)) != 0 {
            let to = Casella::da_indice(to_idx).unwrap();
            if rank == rank_promo { add_promotions(mosse, from, to, false); }
            else {
                mosse.push(Mossa::new(from, to, None));
                if (1u64 << from_idx) & start_rank_mask != 0 {
                    let double_idx = (from_idx as i32 + shift_double) as usize;
                    if (empty & (1u64 << double_idx)) != 0 {
                        mosse.push(Mossa::new_with_flag(from, Casella::da_indice(double_idx).unwrap(), None, MoveFlag::DoublePawnPush));
                    }
                }
            }
        }
        let mut captures = pawn_attacks(from_idx, colore) & enemy;
        while captures != 0 {
            let to_idx = captures.trailing_zeros() as usize; captures &= captures - 1;
            let to = Casella::da_indice(to_idx).unwrap();
            if rank == rank_promo { add_promotions(mosse, from, to, true); }
            else { mosse.push(Mossa::new_with_flag(from, to, None, MoveFlag::Capture)); }
        }
        if let Some(ep_idx) = scacchiera.en_passant {
            if (pawn_attacks(from_idx, colore) & (1u64 << ep_idx)) != 0 {
                mosse.push(Mossa::new_with_flag(from, Casella::da_indice(ep_idx).unwrap(), None, MoveFlag::EnPassant));
            }
        }
    }
}

fn add_promotions(mosse: &mut Vec<Mossa>, from: Casella, to: Casella, capture: bool) {
    let flag = if capture { MoveFlag::PromotionCapture } else { MoveFlag::Promotion };
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
            let flag = if (scacchiera.bitboard_colore(colore.opposto()) & (1u64<<to)) != 0 { MoveFlag::Capture } else { MoveFlag::None };
            mosse.push(Mossa::new_with_flag(Casella::da_indice(from).unwrap(), Casella::da_indice(to).unwrap(), None, flag));
        }
    }
}

fn genera_mosse_alfiere(s: &Scacchiera, c: Colore, m: &mut Vec<Mossa>) { genera_sliding(s, c, m, Pezzo::Alfiere); }
fn genera_mosse_torre(s: &Scacchiera, c: Colore, m: &mut Vec<Mossa>) { genera_sliding(s, c, m, Pezzo::Torre); }
fn genera_mosse_regina(s: &Scacchiera, c: Colore, m: &mut Vec<Mossa>) { genera_sliding(s, c, m, Pezzo::Regina); }

fn genera_sliding(scacchiera: &Scacchiera, colore: Colore, mosse: &mut Vec<Mossa>, pezzo: Pezzo) {
    let mut bb = scacchiera.bitboard_pezzo_colore(pezzo, colore);
    let occ = scacchiera.occupazione();
    while bb != 0 {
        let from = bb.trailing_zeros() as usize; bb &= bb - 1;
        let attacks = match pezzo {
            Pezzo::Alfiere => bishop_attacks(from, occ),
            Pezzo::Torre => rook_attacks(from, occ),
            Pezzo::Regina => queen_attacks(from, occ),
            _ => 0
        } & !scacchiera.bitboard_colore(colore);
        let mut m_bb = attacks;
        while m_bb != 0 {
            let to = m_bb.trailing_zeros() as usize; m_bb &= m_bb - 1;
            let flag = if (scacchiera.bitboard_colore(colore.opposto()) & (1u64<<to)) != 0 { MoveFlag::Capture } else { MoveFlag::None };
            mosse.push(Mossa::new_with_flag(Casella::da_indice(from).unwrap(), Casella::da_indice(to).unwrap(), None, flag));
        }
    }
}

fn genera_mosse_re(scacchiera: &Scacchiera, colore: Colore, mosse: &mut Vec<Mossa>) {
    let mut bb = scacchiera.bitboard_pezzo_colore(Pezzo::Re, colore);
    if bb == 0 { return; }
    let from = bb.trailing_zeros() as usize;
    let mut attacks = king_attacks(from) & !scacchiera.bitboard_colore(colore);
    while attacks != 0 {
        let to = attacks.trailing_zeros() as usize; attacks &= attacks - 1;
        let flag = if (scacchiera.bitboard_colore(colore.opposto()) & (1u64<<to)) != 0 { MoveFlag::Capture } else { MoveFlag::None };
        mosse.push(Mossa::new_with_flag(Casella::da_indice(from).unwrap(), Casella::da_indice(to).unwrap(), None, flag));
    }
    if !scacchiera.re_in_scacco(colore) {
        let r = scacchiera.diritti_arrocco_struct();
        let occ = scacchiera.occupazione();
        let is_att = |sq| scacchiera.casella_attaccata(Casella::da_indice(sq).unwrap(), colore.opposto());
        if colore == Colore::Bianco {
            if r.bianco_lato_re && (occ&(1<<5|1<<6))==0 && !is_att(5) && !is_att(6) { mosse.push(Mossa::new_with_flag(Casella::da_indice(from).unwrap(), Casella::da_indice(6).unwrap(), None, MoveFlag::CastleKingSide)); }
            if r.bianco_lato_regina && (occ&(1<<1|1<<2|1<<3))==0 && !is_att(2) && !is_att(3) { mosse.push(Mossa::new_with_flag(Casella::da_indice(from).unwrap(), Casella::da_indice(2).unwrap(), None, MoveFlag::CastleQueenSide)); }
        } else {
            if r.nero_lato_re && (occ&(1<<61|1<<62))==0 && !is_att(61) && !is_att(62) { mosse.push(Mossa::new_with_flag(Casella::da_indice(from).unwrap(), Casella::da_indice(62).unwrap(), None, MoveFlag::CastleKingSide)); }
            if r.nero_lato_regina && (occ&(1<<57|1<<58|1<<59))==0 && !is_att(58) && !is_att(59) { mosse.push(Mossa::new_with_flag(Casella::da_indice(from).unwrap(), Casella::da_indice(58).unwrap(), None, MoveFlag::CastleQueenSide)); }
        }
    }
}