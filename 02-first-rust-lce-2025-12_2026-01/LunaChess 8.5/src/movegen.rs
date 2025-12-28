use crate::board::{Scacchiera, Pezzo, Colore, Casella, Mossa, MoveFlag, Bitboard};
use crate::attacks::{pawn_attacks, knight_attacks, king_attacks, bishop_attacks, rook_attacks, queen_attacks};

pub struct MossaConScore { pub mossa: Mossa, pub score: i32 }

pub struct MovePicker {
    phase: MovePhase,
    tt_move: Option<Mossa>,
    killers: [Option<Mossa>; 2],
    moves: Vec<MossaConScore>,
    index: usize,
}

#[derive(PartialEq)]
enum MovePhase { TTMove, GenCaptures, Captures, Killers, GenQuiets, Quiets }

impl MovePicker {
    pub fn new(tt_move: Option<Mossa>, killers: [Option<Mossa>; 2]) -> Self {
        Self { phase: MovePhase::TTMove, tt_move, killers, moves: Vec::with_capacity(64), index: 0 }
    }

    pub fn next(&mut self, s: &Scacchiera) -> Option<Mossa> {
        loop {
            match self.phase {
                MovePhase::TTMove => { self.phase = MovePhase::GenCaptures; if let Some(m) = self.tt_move { return Some(m); } }
                MovePhase::GenCaptures => { self.moves.clear(); genera_pseudo_legali(s, &mut self.moves, true); self.score_captures(s); self.moves.sort_unstable_by_key(|m| -m.score); self.phase = MovePhase::Captures; self.index = 0; }
                MovePhase::Captures => {
                    if self.index < self.moves.len() {
                        let m = self.moves[self.index].mossa; self.index += 1;
                        if Some(m) == self.tt_move { continue; } return Some(m);
                    }
                    self.phase = MovePhase::Killers; self.index = 0;
                }
                MovePhase::Killers => {
                    if self.index < 2 {
                        let m = self.killers[self.index]; self.index += 1;
                        if let Some(k) = m { if Some(k) == self.tt_move { continue; } if s.pezzo_su_casella(k.da()).is_some() { return Some(k); } }
                        continue;
                    }
                    self.phase = MovePhase::GenQuiets;
                }
                MovePhase::GenQuiets => { self.moves.clear(); genera_pseudo_legali(s, &mut self.moves, false); self.phase = MovePhase::Quiets; self.index = 0; }
                MovePhase::Quiets => {
                    if self.index < self.moves.len() {
                        let m = self.moves[self.index].mossa; self.index += 1;
                        if Some(m) == self.tt_move || self.killers.contains(&Some(m)) { continue; } return Some(m);
                    }
                    return None;
                }
            }
        }
    }

    fn score_captures(&mut self, s: &Scacchiera) {
        for m_info in self.moves.iter_mut() {
            let m = m_info.mossa;
            let victim = s.pezzo_su_casella(m.a()).map_or(0, |(p, _)| p.valore());
            let attacker = s.pezzo_su_casella(m.da()).map_or(0, |(p, _)| p.valore());
            m_info.score = victim * 10 - attacker;
        }
    }
}

pub fn genera_mosse_legali(s: &mut Scacchiera) -> Vec<Mossa> {
    let mut pseudo = Vec::with_capacity(64);
    genera_pseudo_legali(s, &mut pseudo, false);
    genera_pseudo_legali(s, &mut pseudo, true);
    let mut legali = Vec::new();
    let us = s.turno();
    for m_info in pseudo {
        if s.esegui_mossa(&m_info.mossa) {
            if !s.re_in_scacco(us) { legali.push(m_info.mossa); }
            s.annulla_mossa();
        }
    }
    legali
}

fn genera_pseudo_legali(s: &Scacchiera, mosse: &mut Vec<MossaConScore>, solo_catture: bool) {
    let us = s.turno(); let occ = s.occupazione(); let enemy = s.bitboard_colore(us.opposto());
    let target = if solo_catture { enemy } else { !s.bitboard_colore(us) };
    
    // Pedoni
    let mut bb = s.bitboard_pezzo_colore(Pezzo::Pedone, us);
    while bb != 0 {
        let from_idx = bb.trailing_zeros() as usize; let from = Casella(from_idx); bb &= bb - 1;
        let ep_bit = s.en_passant.map_or(0, |sq| 1u64 << sq);
        let mut attacks = pawn_attacks(from_idx, us) & (enemy | ep_bit);
        while attacks != 0 {
            let to_idx = attacks.trailing_zeros() as usize; let to = Casella(to_idx); attacks &= attacks - 1;
            let flag = if Some(to_idx) == s.en_passant { MoveFlag::EnPassant } else { MoveFlag::Capture };
            if to.rank() == 0 || to.rank() == 7 { add_promo(from, to, true, mosse); }
            else { mosse.push(MossaConScore { mossa: Mossa::new_with_flag(from, to, None, flag), score: 0 }); }
        }
        if !solo_catture {
            let step = if us == Colore::Bianco { 8 } else { -8 };
            let to_idx = (from_idx as i32 + step) as usize;
            if to_idx < 64 && (occ & (1 << to_idx)) == 0 {
                let to = Casella(to_idx);
                if to.rank() == 0 || to.rank() == 7 { add_promo(from, to, false, mosse); }
                else {
                    mosse.push(MossaConScore { mossa: Mossa::new_with_flag(from, to, None, MoveFlag::None), score: 0 });
                    let start_rank = if us == Colore::Bianco { 1 } else { 6 };
                    let double_idx = (to_idx as i32 + step) as usize;
                    if from.rank() == start_rank && double_idx < 64 && (occ & (1 << double_idx)) == 0 {
                        mosse.push(MossaConScore { mossa: Mossa::new_with_flag(from, Casella(double_idx), None, MoveFlag::DoublePawnPush), score: 0 });
                    }
                }
            }
        }
    }
    // Pezzi
    for p in [Pezzo::Cavallo, Pezzo::Alfiere, Pezzo::Torre, Pezzo::Regina, Pezzo::Re] {
        let mut p_bb = s.bitboard_pezzo_colore(p, us);
        while p_bb != 0 {
            let from_idx = p_bb.trailing_zeros() as usize; p_bb &= p_bb - 1;
            let mut att = match p { Pezzo::Cavallo => knight_attacks(from_idx), Pezzo::Alfiere => bishop_attacks(from_idx, occ), Pezzo::Torre => rook_attacks(from_idx, occ), Pezzo::Regina => queen_attacks(from_idx, occ), Pezzo::Re => king_attacks(from_idx), _ => 0 } & target;
            while att != 0 {
                let to_idx = att.trailing_zeros() as usize; att &= att - 1;
                let flag = if (enemy & (1 << to_idx)) != 0 { MoveFlag::Capture } else { MoveFlag::None };
                mosse.push(MossaConScore { mossa: Mossa::new_with_flag(Casella(from_idx), Casella(to_idx), None, flag), score: 0 });
            }
        }
    }
    if !solo_catture { genera_castling(s, us, mosse); }
}

fn add_promo(f: Casella, t: Casella, cap: bool, mosse: &mut Vec<MossaConScore>) {
    let flag = if cap { MoveFlag::PromotionCapture } else { MoveFlag::Promotion };
    for p in [Pezzo::Regina, Pezzo::Torre, Pezzo::Alfiere, Pezzo::Cavallo] { mosse.push(MossaConScore { mossa: Mossa::new_with_flag(f, t, Some(p), flag), score: 0 }); }
}

fn genera_castling(s: &Scacchiera, col: Colore, mosse: &mut Vec<MossaConScore>) {
    let r = s.diritti_arrocco_struct(); let occ = s.occupazione();
    let is_att = |sq| s.casella_attaccata(Casella(sq), col.opposto());
    if col == Colore::Bianco {
        if r.bianco_lato_re && (occ & 0x60) == 0 && !is_att(4) && !is_att(5) && !is_att(6) { mosse.push(MossaConScore { mossa: Mossa::new_with_flag(Casella(4), Casella(6), None, MoveFlag::CastleKingSide), score: 0 }); }
        if r.bianco_lato_regina && (occ & 0x0E) == 0 && !is_att(4) && !is_att(3) && !is_att(2) { mosse.push(MossaConScore { mossa: Mossa::new_with_flag(Casella(4), Casella(2), None, MoveFlag::CastleQueenSide), score: 0 }); }
    } else {
        if r.nero_lato_re && (occ & 0x6000000000000000) == 0 && !is_att(60) && !is_att(61) && !is_att(62) { mosse.push(MossaConScore { mossa: Mossa::new_with_flag(Casella(60), Casella(62), None, MoveFlag::CastleKingSide), score: 0 }); }
        if r.nero_lato_regina && (occ & 0x0E00000000000000) == 0 && !is_att(60) && !is_att(59) && !is_att(58) { mosse.push(MossaConScore { mossa: Mossa::new_with_flag(Casella(60), Casella(58), None, MoveFlag::CastleQueenSide), score: 0 }); }
    }
}