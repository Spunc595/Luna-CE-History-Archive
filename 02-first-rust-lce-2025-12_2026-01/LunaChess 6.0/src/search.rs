// src/search.rs

use crate::board::{Scacchiera, Pezzo, Mossa, Casella};
use crate::movegen::{self, MoveOrderer};
use crate::evaluation::valuta_posizione;
use crate::transposition::{TranspositionTable, Bound};
use std::time::{Instant, Duration};

// Costanti
const INFINITO: i32 = 100_000;
const VITTORIA: i32 = 50_000;
const MAX_NODI: u64 = 100_000_000;
const MAX_PLY: usize = 64;
const MATE_BOUND: i32 = VITTORIA - 100;
const NULL_MOVE_R: i32 = 2;  // R value per null move pruning
const ASPIRATION_INITIAL: i32 = 25;
const ASPIRATION_DELTA: i32 = 25;

pub struct Motore {
    pub transposition_table: TranspositionTable,
    pub nodi_visitati: u64,
    pub tempo_limite: Instant,
    pub mossa_migliore: Option<Mossa>,
    pub profondita_massima: u8,
    pub stop_requested: bool,
    pub orderer: MoveOrderer,
    pub pv_table: Vec<Vec<Mossa>>,
    pub ply: usize,
    pub killer_moves: Vec<[Option<Mossa>; 2]>,
    pub history: Vec<Vec<i32>>,
    pub countermove: Vec<[Option<Mossa>; 64]>,
    pub start_time: Instant,
    pub max_time: Duration,
    pub node_limit: u64,
    pub seldepth: usize,
    pub stat_nullmoves: u64,
    pub stat_futility: u64,
    pub stat_lmr: u64,
    pub stat_tt_hits: u64,
    pub stat_tt_cutoffs: u64,
    pub stat_late_red: u64,
    pub stat_extensions: u64,
    pub use_opening_book: bool,
    pub generation: u8,
    pub search_stack: Vec<i32>,
    pub eval_stack: Vec<i32>,
}

impl Motore {
    pub fn nuovo() -> Self {
        let mut pv_table = Vec::with_capacity(MAX_PLY);
        for _ in 0..MAX_PLY {
            pv_table.push(Vec::with_capacity(MAX_PLY));
        }
        
        let mut history = Vec::with_capacity(64);
        for _ in 0..64 {
            history.push(vec![0; 64]);
        }
        
        Motore {
            transposition_table: TranspositionTable::new(256),  // 256 MB
            nodi_visitati: 0,
            tempo_limite: Instant::now(),
            mossa_migliore: None,
            profondita_massima: 20,
            stop_requested: false,
            orderer: MoveOrderer::new(MAX_PLY),
            pv_table,
            ply: 0,
            killer_moves: vec![[None; 2]; MAX_PLY],
            history,
            countermove: vec![[None; 64]; 12],
            start_time: Instant::now(),
            max_time: Duration::from_secs(10),
            node_limit: MAX_NODI,
            seldepth: 0,
            stat_nullmoves: 0,
            stat_futility: 0,
            stat_lmr: 0,
            stat_tt_hits: 0,
            stat_tt_cutoffs: 0,
            stat_late_red: 0,
            stat_extensions: 0,
            use_opening_book: false,
            generation: 0,
            search_stack: vec![-INFINITO; MAX_PLY],
            eval_stack: vec![0; MAX_PLY],
        }
    }
    
    pub fn imposta_profondita_massima(&mut self, profondita: u8) {
        self.profondita_massima = profondita.min(32);
    }
    
    pub fn stop(&mut self) {
        self.stop_requested = true;
    }
    
    pub fn attiva_opening_book(&mut self, use_book: bool) {
        self.use_opening_book = use_book;
    }
    
    pub fn set_contempt(&mut self, _contempt: i32) {
        // Implementa se necessario
    }
    
    pub fn set_multipv(&mut self, _multipv: u32) {
        // Implementa se necessario
    }
    
    pub fn reset_transposition_table(&mut self, size_mb: usize) {
        self.transposition_table = TranspositionTable::new(size_mb);
    }
    
    pub fn valuta_posizione(&self, scacchiera: &Scacchiera) -> i32 {
        valuta_posizione(scacchiera)
    }
    
    // Metodo utility per ottenere i nodi visitati
    pub fn nodi_visitati(&self) -> u64 {
        self.nodi_visitati
    }
    
    pub fn trova_mossa_migliore(&mut self, scacchiera: &Scacchiera, tempo: Duration) -> Option<Mossa> {
        // Reset dello stato
        self.nodi_visitati = 0;
        self.start_time = Instant::now();
        self.tempo_limite = self.start_time + tempo;
        self.max_time = tempo;
        self.mossa_migliore = None;
        self.stop_requested = false;
        self.ply = 0;
        self.seldepth = 0;
        self.clear_pv_table();
        self.clear_heuristics();
        self.generation = self.generation.wrapping_add(1);
        self.transposition_table.set_generation(self.generation);
        
        // Reset statistiche
        self.stat_nullmoves = 0;
        self.stat_futility = 0;
        self.stat_lmr = 0;
        self.stat_tt_hits = 0;
        self.stat_tt_cutoffs = 0;
        self.stat_late_red = 0;
        self.stat_extensions = 0;
        
        // Genera mosse iniziali
        let mosse = movegen::genera_mosse_legali(scacchiera);
        if mosse.is_empty() {
            return None;
        }
        
        if mosse.len() == 1 {
            return Some(mosse[0]);
        }
        
        // Inizializza iterative deepening
        let mut best_move = mosse[0];
        let mut best_value = -INFINITO;
        let mut depth = 1;
        
        // Iterative deepening con aspiration windows
        while depth <= self.profondita_massima && !self.stop_requested {
            if self.check_time_limit() {
                break;
            }
            
            let window = if depth <= 4 {
                (-INFINITO, INFINITO)
            } else {
                let window_size = ASPIRATION_INITIAL + (depth as i32 * 5);
                (
                    best_value - window_size,
                    best_value + window_size
                )
            };
            
            let mut alpha = window.0;
            let mut beta = window.1;
            
            loop {
                let search_result = self.negamax_root(
                    scacchiera,
                    depth as u8,
                    alpha,
                    beta,
                    0
                );
                
                if self.stop_requested {
                    break;
                }
                
                match search_result {
                    Some((score, move_found)) => {
                        if score <= alpha {
                            // Fail low
                            alpha = -INFINITO;
                            continue;
                        } else if score >= beta {
                            // Fail high
                            beta = INFINITO;
                            continue;
                        }
                        
                        // Successo
                        best_value = score;
                        best_move = move_found.unwrap_or(mosse[0]);
                        self.mossa_migliore = Some(best_move);
                        
                        // Aggiorna PV
                        self.update_pv_table(0, best_move);
                        
                        // Output informazioni
                        self.print_search_info(depth as u8, best_value);
                        
                        break;
                    }
                    None => {
                        // Timeout o errore
                        break;
                    }
                }
            }
            
            if self.should_stop_search(depth as u8) {
                break;
            }
            
            depth += 1;
        }
        
        self.print_stats();
        
        self.mossa_migliore
    }
    
    fn negamax_root(
        &mut self,
        scacchiera: &Scacchiera,
        depth: u8,
        mut alpha: i32,
        beta: i32,
        ply: usize
    ) -> Option<(i32, Option<Mossa>)> {
        if ply >= MAX_PLY {
            return Some((valuta_posizione(scacchiera), None));
        }
        
        if self.check_time_limit() {
            return None;
        }
        
        // TT lookup
        let tt_move = self.transposition_table.probe_move(scacchiera.hash());
        
        // Genera e ordina le mosse
        let mut moves = movegen::genera_mosse_legali(scacchiera);
        if moves.is_empty() {
            let in_check = scacchiera.re_in_scacco(scacchiera.colore_attivo());
            return Some((
                if in_check {
                    -VITTORIA + ply as i32
                } else {
                    0
                },
                None
            ));
        }
        
        // Ordina mosse
        self.order_moves_root(&mut moves, tt_move, ply);
        
        let mut best_value = -INFINITO;
        let mut best_move = None;
        
        for &mossa in &moves {
            if self.check_time_limit() {
                return None;
            }
            
            let mut nuova_scacchiera = scacchiera.clone();
            if !nuova_scacchiera.esegui_mossa(&mossa) {
                continue;
            }
            
            let score = if ply == 0 && depth >= 3 && moves.len() > 1 {
                // Principal Variation Search
                let mut score = -self.negamax(
                    &nuova_scacchiera,
                    depth - 1,
                    -alpha - 1,
                    -alpha,
                    ply + 1
                );
                
                if score > alpha && score < beta {
                    score = -self.negamax(
                        &nuova_scacchiera,
                        depth - 1,
                        -beta,
                        -alpha,
                        ply + 1
                    );
                }
                score
            } else {
                -self.negamax(
                    &nuova_scacchiera,
                    depth - 1,
                    -beta,
                    -alpha,
                    ply + 1
                )
            };
            
            if score > best_value {
                best_value = score;
                best_move = Some(mossa);
                
                if score > alpha {
                    alpha = score;
                    
                    // Aggiorna PV
                    self.update_pv_table(ply, mossa);
                }
            }
            
            if alpha >= beta {
                break;
            }
        }
        
        Some((best_value, best_move))
    }
    
    fn negamax(
        &mut self,
        scacchiera: &Scacchiera,
        depth: u8,
        mut alpha: i32,
        beta: i32,
        ply: usize
    ) -> i32 {
        // Aggiorna seldepth
        if ply > self.seldepth {
            self.seldepth = ply;
        }
        
        // Controlla limite tempo
        if self.nodi_visitati % 2048 == 0 && self.check_time_limit() {
            return 0;
        }
        
        // Incrementa conteggio nodi
        self.nodi_visitati += 1;
        
        // Controllo per ripetizione o regola delle 50 mosse
        // Implementa questi metodi in Scacchiera se necessario
        // if scacchiera.is_draw_by_repetition() || scacchiera.is_draw_by_50_moves() {
        //     return 0;
        // }
        
        // TT probe
        let hash = scacchiera.hash();
        let tt_move = self.transposition_table.probe_move(hash);
        
        if let Some(entry) = self.transposition_table.probe(hash) {
            self.stat_tt_hits += 1;
            
            if entry.depth() >= depth as u8 {
                match entry.bound() {
                    Bound::Exact => {
                        self.stat_tt_cutoffs += 1;
                        return entry.score() as i32;
                    }
                    Bound::Lower => {
                        if entry.score() as i32 >= beta {
                            self.stat_tt_cutoffs += 1;
                            return entry.score() as i32;
                        }
                        if entry.score() as i32 > alpha {
                            alpha = entry.score() as i32;
                        }
                    }
                    Bound::Upper => {
                        if entry.score() as i32 <= alpha {
                            self.stat_tt_cutoffs += 1;
                            return entry.score() as i32;
                        }
                    }
                }
            }
        }
        
        // Controlla se siamo a un nodo foglia
        if depth == 0 {
            return self.quiescence(scacchiera, alpha, beta, ply);
        }
        
        // Salva valutazione statica
        let static_eval = valuta_posizione(scacchiera);
        if ply < MAX_PLY {
            self.eval_stack[ply] = static_eval;
        }
        
        // Null Move Pruning (semplificato)
        if !scacchiera.re_in_scacco(scacchiera.colore_attivo()) 
            && depth >= 3 
            && static_eval >= beta 
            && ply > 0 {
            
            self.stat_nullmoves += 1;
            
            // Null move pruning semplificato
            if static_eval >= beta {
                return beta;
            }
        }
        
        // Razoring
        if depth == 1 && static_eval + 200 < alpha {
            let qscore = self.quiescence(scacchiera, alpha, beta, ply);
            if qscore < alpha {
                return alpha;
            }
        }
        
        // Futility Pruning
        if depth <= 3 
            && !scacchiera.re_in_scacco(scacchiera.colore_attivo()) 
            && static_eval + 150 * depth as i32 <= alpha {
            
            self.stat_futility += 1;
            return self.quiescence(scacchiera, alpha, beta, ply);
        }
        
        // Genera e ordina mosse
        let mut moves = movegen::genera_mosse_legali(scacchiera);
        if moves.is_empty() {
            if scacchiera.re_in_scacco(scacchiera.colore_attivo()) {
                return -VITTORIA + ply as i32;
            }
            return 0;
        }
        
        self.order_moves(&mut moves, tt_move, depth as i32, ply);
        
        let in_check = scacchiera.re_in_scacco(scacchiera.colore_attivo());
        let mut best_score = -INFINITO;
        let mut best_move = moves[0];
        let mut moves_searched = 0;
        
        for &mossa in &moves {
            let is_capture = self.is_capture(&mossa, scacchiera);
            let gives_check = self.gives_check(scacchiera, &mossa);
            
            // Futility Pruning (mosse tardive)
            if moves_searched >= 3 
                && depth <= 3 
                && !in_check 
                && !is_capture 
                && !gives_check
                && static_eval + 100 * depth as i32 <= alpha {
                
                self.stat_futility += 1;
                continue;
            }
            
            // Late Move Reduction
            let mut reduction = 0;
            if depth >= 3 
                && moves_searched >= 4 
                && !is_capture 
                && !gives_check 
                && !in_check
                && mossa.promozione().is_none() {
                
                reduction = self.late_move_reduction(depth as i32, moves_searched);
                self.stat_lmr += 1;
            }
            
            let mut nuova_scacchiera = scacchiera.clone();
            if !nuova_scacchiera.esegui_mossa(&mossa) {
                continue;
            }
            
            // Estensioni
            let mut extension = 0;
            if gives_check || (is_capture && self.see(scacchiera, &mossa) >= 0) {
                extension = 1;
                self.stat_extensions += 1;
            }
            
            let new_depth = if reduction > 0 {
                (depth as i32 - reduction).max(1) as u8
            } else {
                depth.saturating_sub(1).saturating_add(extension as u8)
            };
            
            let score = if moves_searched == 0 {
                -self.negamax(&nuova_scacchiera, new_depth, -beta, -alpha, ply + 1)
            } else {
                // Zero window search con LMR
                let mut score = -self.negamax(
                    &nuova_scacchiera, 
                    new_depth, 
                    -alpha - 1, 
                    -alpha, 
                    ply + 1
                );
                
                if score > alpha && score < beta && reduction > 0 {
                    // Re-search con finestra completa
                    score = -self.negamax(
                        &nuova_scacchiera, 
                        depth - 1, 
                        -beta, 
                        -alpha, 
                        ply + 1
                    );
                }
                score
            };
            
            moves_searched += 1;
            
            if score > best_score {
                best_score = score;
                best_move = mossa;
                
                if score > alpha {
                    alpha = score;
                    if ply < MAX_PLY {
                        self.update_pv_table(ply, mossa);
                    }
                    
                    // Aggiorna euristica history
                    if !is_capture {
                        self.update_history(mossa, depth as i32);
                    }
                }
            }
            
            if alpha >= beta {
                // Cutoff
                if !is_capture {
                    self.update_killers(mossa, ply);
                }
                break;
            }
        }
        
        // Salva in TT
        let bound = if best_score <= self.search_stack.get(ply).copied().unwrap_or(-INFINITO) {
            Bound::Upper
        } else if best_score >= beta {
            Bound::Lower
        } else {
            Bound::Exact
        };
        
        self.transposition_table.store(
            hash,
            best_score as i16,
            static_eval as i16,
            Some(best_move),
            depth as u8,
            bound
        );
        
        best_score
    }
    
    fn quiescence(&mut self, scacchiera: &Scacchiera, mut alpha: i32, beta: i32, ply: usize) -> i32 {
        if ply >= MAX_PLY {
            return valuta_posizione(scacchiera);
        }
        
        self.nodi_visitati += 1;
        
        // Stand pat
        let stand_pat = valuta_posizione(scacchiera);
        if stand_pat >= beta {
            return beta;
        }
        
        if alpha < stand_pat {
            alpha = stand_pat;
        }
        
        // Delta pruning
        let delta = 900;  // Valore della regina
        if stand_pat + delta <= alpha {
            return alpha;
        }
        
        // Genera catture e scacchi
        let mut moves = movegen::genera_mosse_legali(scacchiera);
        moves.retain(|m| {
            self.is_capture(m, scacchiera) || 
            self.gives_check(scacchiera, m) ||
            m.promozione().is_some()
        });
        
        if moves.is_empty() {
            return stand_pat;
        }
        
        // Ordina catture per MVV/LVA
        self.order_captures(&mut moves, scacchiera);
        
        for &mossa in &moves {
            // SEE pruning per catture svantaggiose
            if self.is_capture(&mossa, scacchiera) && self.see(scacchiera, &mossa) < 0 {
                continue;
            }
            
            let mut nuova_scacchiera = scacchiera.clone();
            if !nuova_scacchiera.esegui_mossa(&mossa) {
                continue;
            }
            
            let score = -self.quiescence(&nuova_scacchiera, -beta, -alpha, ply + 1);
            
            if score > alpha {
                alpha = score;
                
                if alpha >= beta {
                    return beta;
                }
            }
        }
        
        alpha
    }
    
    fn late_move_reduction(&self, depth: i32, moves_searched: usize) -> i32 {
        let mut reduction = 0;
        
        if depth >= 3 {
            reduction = 1;
            
            if depth >= 5 && moves_searched >= 8 {
                reduction = 2;
            }
            
            if depth >= 8 && moves_searched >= 16 {
                reduction = 3;
            }
        }
        
        reduction
    }
    
    fn order_moves_root(
        &mut self,
        moves: &mut Vec<Mossa>,
        tt_move: Option<Mossa>,
        ply: usize
    ) {
        moves.sort_by(|a, b| {
            let score_a = self.move_score(a, tt_move, ply);
            let score_b = self.move_score(b, tt_move, ply);
            score_b.cmp(&score_a)
        });
    }
    
    fn order_moves(
        &mut self,
        moves: &mut Vec<Mossa>,
        tt_move: Option<Mossa>,
        _depth: i32,
        ply: usize
    ) {
        moves.sort_by(|a, b| {
            let score_a = self.move_score(a, tt_move, ply);
            let score_b = self.move_score(b, tt_move, ply);
            score_b.cmp(&score_a)
        });
    }
    
    fn move_score(
        &self,
        mossa: &Mossa,
        tt_move: Option<Mossa>,
        ply: usize
    ) -> i32 {
        let mut score = 0;
        
        // TT move prima
        if tt_move == Some(*mossa) {
            score += 1_000_000;
        }
        
        // PV move dall'iterazione precedente
        if ply < MAX_PLY && !self.pv_table[ply].is_empty() && self.pv_table[ply][0] == *mossa {
            score += 900_000;
        }
        
        // Catture (MVV/LVA) - controlla flag per catture
        if mossa.flag().is_capture() {
            score += 100_000;
        }
        
        // Promozioni
        if let Some(promo) = mossa.promozione() {
            score += 80_000 + self.piece_value(promo);
        }
        
        // Killer moves
        if ply < MAX_PLY {
            if self.killer_moves[ply][0] == Some(*mossa) {
                score += 9000;
            } else if self.killer_moves[ply][1] == Some(*mossa) {
                score += 8000;
            }
        }
        
        // Counter move (semplificato)
        if ply > 0 && ply < MAX_PLY {
            if let Some(prev_move) = self.pv_table[ply - 1].first() {
                let from_idx = prev_move.da().indice();
                if from_idx < 64 {
                    let idx = from_idx % 12;
                    if idx < self.countermove.len() {
                        let col_idx = prev_move.a().indice() % 64;
                        if col_idx < self.countermove[idx].len() {
                            if self.countermove[idx][col_idx] == Some(*mossa) {
                                score += 7000;
                            }
                        }
                    }
                }
            }
        }
        
        // Euristica history
        let from = mossa.da().indice();
        let to = mossa.a().indice();
        if from < 64 && to < 64 && from < self.history.len() && to < self.history[from].len() {
            score += self.history[from][to].min(8000);
        }
        
        score
    }
    
    fn order_captures(&self, captures: &mut Vec<Mossa>, scacchiera: &Scacchiera) {
        captures.sort_by(|a, b| {
            let score_a = self.estimate_capture_score(a, scacchiera);
            let score_b = self.estimate_capture_score(b, scacchiera);
            score_b.cmp(&score_a)
        });
    }
    
    fn estimate_capture_score(&self, mossa: &Mossa, scacchiera: &Scacchiera) -> i32 {
        if let Some((victim, attacker)) = self.get_capture_info(mossa, scacchiera) {
            self.piece_value(victim) * 10 - self.piece_value(attacker)
        } else {
            0
        }
    }
    
    fn see(&self, scacchiera: &Scacchiera, mossa: &Mossa) -> i32 {
        if let Some((victim, _)) = self.get_capture_info(mossa, scacchiera) {
            let mut board_copy = scacchiera.clone();
            
            // Esegui la cattura
            if board_copy.esegui_mossa(mossa) {
                return self.piece_value(victim);
            }
        }
        
        0
    }
    
    fn get_capture_info(&self, mossa: &Mossa, scacchiera: &Scacchiera) -> Option<(Pezzo, Pezzo)> {
        let colore = scacchiera.colore_attivo();
        let avversario = colore.opposto();
        
        if let Some((victim, victim_color)) = scacchiera.pezzo_su_casella(mossa.a()) {
            if victim_color == avversario {
                if let Some((attacker, _)) = scacchiera.pezzo_su_casella(mossa.da()) {
                    return Some((victim, attacker));
                }
            }
        }
        
        // En passant
        if let Some(en_passant_idx) = scacchiera.en_passant() {
            if let Some(en_passant_sq) = Casella::da_indice(en_passant_idx) {
                if mossa.a() == en_passant_sq {
                    // Per en passant, controlla se è effettivamente una cattura en passant
                    if let Some((Pezzo::Pedone, col)) = scacchiera.pezzo_su_casella(mossa.da()) {
                        if col == colore {
                            return Some((Pezzo::Pedone, Pezzo::Pedone));
                        }
                    }
                }
            }
        }
        
        None
    }
    
    fn is_capture(&self, mossa: &Mossa, scacchiera: &Scacchiera) -> bool {
        self.get_capture_info(mossa, scacchiera).is_some()
    }
    
    fn gives_check(&self, scacchiera: &Scacchiera, mossa: &Mossa) -> bool {
        let mut nuova_scacchiera = scacchiera.clone();
        if nuova_scacchiera.esegui_mossa(mossa) {
            nuova_scacchiera.re_in_scacco(scacchiera.colore_attivo().opposto())
        } else {
            false
        }
    }
    
    fn piece_value(&self, pezzo: Pezzo) -> i32 {
        match pezzo {
            Pezzo::Pedone => 100,
            Pezzo::Cavallo => 320,
            Pezzo::Alfiere => 330,
            Pezzo::Torre => 500,
            Pezzo::Regina => 900,
            Pezzo::Re => 10000,
        }
    }
    
    fn update_killers(&mut self, mossa: Mossa, ply: usize) {
        if ply >= MAX_PLY {
            return;
        }
        
        let killers = &mut self.killer_moves[ply];
        
        if killers[0] != Some(mossa) {
            killers[1] = killers[0];
            killers[0] = Some(mossa);
        }
    }
    
    fn update_history(&mut self, mossa: Mossa, depth: i32) {
        let from = mossa.da().indice();
        let to = mossa.a().indice();
        
        if from < 64 && to < 64 && from < self.history.len() && to < self.history[from].len() {
            self.history[from][to] += depth * depth;
            
            // Invecchia la tabella history se i valori diventano troppo grandi
            if self.history[from][to] > 2_000_000 {
                for row in &mut self.history {
                    for val in row {
                        *val /= 2;
                    }
                }
            }
        }
    }
    
    fn update_pv_table(&mut self, ply: usize, mossa: Mossa) {
        if ply >= MAX_PLY {
            return;
        }
        
        // Pulisci PV del ply corrente
        self.pv_table[ply].clear();
        self.pv_table[ply].push(mossa);
        
        // Copia PV dal ply successivo
        if ply + 1 < MAX_PLY {
            let next_ply = self.pv_table[ply + 1].clone();
            self.pv_table[ply].extend(next_ply.iter());
        }
    }
    
    fn clear_pv_table(&mut self) {
        for pv in &mut self.pv_table {
            pv.clear();
        }
    }
    
    fn clear_heuristics(&mut self) {
        // Pulisci tabella history
        for row in &mut self.history {
            for val in row {
                *val = 0;
            }
        }
        
        // Pulisci killer moves
        for killers in &mut self.killer_moves {
            killers[0] = None;
            killers[1] = None;
        }
        
        // Pulisci tabella countermove
        for row in &mut self.countermove {
            for val in row {
                *val = None;
            }
        }
    }
    
    fn check_time_limit(&self) -> bool {
        self.stop_requested || Instant::now() > self.tempo_limite || self.nodi_visitati > self.node_limit
    }
    
    fn should_stop_search(&self, depth: u8) -> bool {
        // Fermati se stai usando troppo tempo
        if self.start_time.elapsed() > self.max_time.mul_f32(0.9) {
            return true;
        }
        
        // Fermati presto se hai trovato scacco matto (controllo semplificato)
        if depth >= 2 && self.mossa_migliore.is_some() {
            // Potremmo controllare TT per score di matto qui
            return false;
        }
        
        false
    }
    
    fn print_search_info(&self, depth: u8, score: i32) {
        let elapsed = self.start_time.elapsed();
        let nps = if elapsed.as_secs() > 0 {
            self.nodi_visitati / elapsed.as_secs() as u64
        } else {
            self.nodi_visitati * 1000 / elapsed.as_millis() as u64
        };
        
        print!("info depth {} seldepth {} score ", depth, self.seldepth);
        
        if score.abs() > MATE_BOUND {
            let mate_in = (VITTORIA - score.abs()) / 2 + 1;
            if score > 0 {
                print!("mate {}", mate_in);
            } else {
                print!("mate -{}", mate_in);
            }
        } else {
            print!("cp {}", score);
        }
        
        print!(" nodes {} nps {} time {} pv ", 
               self.nodi_visitati, nps, elapsed.as_millis());
        
        if !self.pv_table[0].is_empty() {
            for mossa in &self.pv_table[0] {
                print!("{} ", mossa);  // Usa il trait Display di Mossa
            }
        }
        
        println!();
    }
    
    fn print_stats(&self) {
        let elapsed = self.start_time.elapsed();
        let nps = if elapsed.as_secs() > 0 {
            self.nodi_visitati / elapsed.as_secs() as u64
        } else {
            self.nodi_visitati * 1000 / elapsed.as_millis() as u64
        };
        
        println!("\n=== STATISTICHE RICERCA ===");
        println!("Tempo totale: {:.3}s", elapsed.as_secs_f64());
        println!("Nodi visitati: {}", self.nodi_visitati);
        println!("Nodi/secondo: {}", nps);
        println!("Profondità massima raggiunta: {}", self.seldepth);
        println!("TT hit rate: {:.1}%", 
                 (self.stat_tt_hits as f64 / self.nodi_visitati.max(1) as f64) * 100.0);
        println!("TT cutoffs: {}", self.stat_tt_cutoffs);
        println!("Null moves: {}", self.stat_nullmoves);
        println!("Futility prunes: {}", self.stat_futility);
        println!("LMR reductions: {}", self.stat_lmr);
        println!("Extensions: {}", self.stat_extensions);
    }
}

impl Default for Motore {
    fn default() -> Self {
        Self::nuovo()
    }
}

impl Clone for Motore {
    fn clone(&self) -> Self {
        // Crea un nuovo motore con la stessa configurazione
        Self::nuovo()
    }
}