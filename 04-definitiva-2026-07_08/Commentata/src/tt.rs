use crate::board::Mossa;

/// Rappresenta il tipo di confidenza/limite associato al punteggio memorizzato[cite: 6].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Bound {
    None = 0,
    Exact = 1, // Punteggio esatto (valore compreso perfettamente tra Alpha e Beta)[cite: 6]
    Alpha = 2, // Limite superiore (Upper Bound): la ricerca ha fallito in basso (score <= alpha)[cite: 6]
    Beta = 3,  // Limite inferiore (Lower Bound): la ricerca ha generato un beta-cutoff (score >= beta)[cite: 6]
}

/// Singola voce (slot) all'interno della Transposition Table[cite: 6].
/// Ottimizzata nelle dimensioni per massimizzare l'efficienza della cache CPU[cite: 6].
#[derive(Clone, Copy, Debug)]
pub struct TTEntry {
    pub key: u64,        // Chiave Zobrist univoca della posizione[cite: 6]
    pub score: i32,      // Punteggio di valutazione associato[cite: 6]
    pub move_data: u16,  // Dati grezzi (raw) della mossa migliore trovata in questa posizione[cite: 6]
    pub depth: u8,       // Profondità residua della ricerca al momento del salvataggio[cite: 6]
    pub bound: u8,       // Tipo di Bound convertito in u8 per risparmiare spazio in memoria[cite: 6]
    pub generation: u8,  // Identificativo della ricerca corrente per l'invecchiamento delle voci[cite: 6]
}

/// Tabella di trasposizione principale del motore[cite: 6].
/// Implementa una disposizione ad indirizzamento diretto basata su maschera bitwise[cite: 6].
pub struct TranspositionTable {
    entries: Vec<TTEntry>,
    mask: usize,
    generation: u8,
}

impl TranspositionTable {
    /// Alloca una nuova Transposition Table specificando la dimensione desiderata in Megabyte[cite: 6].
    /// La dimensione effettiva dei vettori viene forzata alla potenza di 2 inferiore o uguale più vicina[cite: 6].
    pub fn new(mb_size: usize) -> Self {
        // Calcola quante istanze di TTEntry possono risiedere nello spazio espresso in MB[cite: 6].
        let size = (mb_size * 1024 * 1024) / std::mem::size_of::<TTEntry>();
        let mut real_size = 1;
        
        // Forza la dimensione ad essere una potenza di 2 per ottimizzare l'operazione di modulo[cite: 6].
        while real_size <= size { 
            real_size *= 2; 
        }
        
        TranspositionTable {
            entries: vec![TTEntry { key: 0, score: 0, move_data: 0, depth: 0, bound: 0, generation: 0 }; real_size],
            mask: real_size - 1, // Maschera bitwise per calcolare l'indice istantaneamente senza divisioni[cite: 6]
            generation: 1,
        }
    }

    /// Svuota completamente la tabella azzerando le chiavi e reimpostando la generazione iniziale[cite: 6].
    pub fn clear(&mut self) {
        for entry in &mut self.entries {
            entry.key = 0;
            entry.generation = 0;
        }
        self.generation = 1;
    }

    /// Incrementa l'identificatore della generazione corrente ad ogni inizio di una nuova ricerca iterativa[cite: 6].
    /// Utilizza il wrapping nativo (`wrapping_add`) per evitare overflow del registro a 8 bit[cite: 6].
    pub fn new_search(&mut self) {
        self.generation = self.generation.wrapping_add(1);
    }

    /// Ispeziona la tabella alla ricerca di una chiave Zobrist corrispondente[cite: 6].
    /// Se i criteri di profondità e i limiti di Alpha/Beta sono soddisfatti, restituisce il punteggio per un cutoff immediato[cite: 6].
    pub fn probe(&self, key: u64, depth: i32, alpha: i32, beta: i32) -> Option<i32> {
        let idx = (key as usize) & self.mask;
        let entry = &self.entries[idx];

        // Verifica se la voce memorizzata corrisponde effettivamente alla posizione richiesta[cite: 6].
        if entry.key == key {
            // Un cutoff è matematicamente valido solo se la profondità salvata è maggiore o uguale a quella corrente[cite: 6].
            if entry.depth as i32 >= depth {
                let score = entry.score; 
                let bound = entry.bound;

                // 1. Valore Esatto: Restituisce direttamente il punteggio[cite: 6].
                if bound == Bound::Exact as u8 {
                    return Some(score);
                }
                // 2. Upper Bound (Alpha): Valido solo se il punteggio non migliora la nostra barriera minima Alpha[cite: 6].
                if bound == Bound::Alpha as u8 && score <= alpha {
                    return Some(score);
                }
                // 3. Lower Bound (Beta): Valido se il punteggio causa una potatura sopra la barriera Beta del nemico[cite: 6].
                if bound == Bound::Beta as u8 && score >= beta {
                    return Some(score);
                }
            }
        }
        None
    }

    /// Recupera la mossa memorizzata associata a una data posizione[cite: 6].
    /// Viene sfruttata nella fase di ordinamento delle mosse (Move Ordering) per analizzare prima la mossa migliore precedente[cite: 6].
    pub fn get_move(&self, key: u64) -> Mossa {
        let idx = (key as usize) & self.mask;
        let entry = &self.entries[idx];
        
        if entry.key == key {
            Mossa::from_data(entry.move_data)
        } else {
            Mossa::null() // Restituisce una mossa vuota/invalida se non trova corrispondenze[cite: 6]
        }
    }

    /// Memorizza o aggiorna una voce all'interno della tabella basandosi su una politica combinata di rimpiazzo[cite: 6].
    pub fn store(&mut self, key: u64, depth: i32, score: i32, bound: Bound, best_move: Mossa) {
        let idx = (key as usize) & self.mask;
        let entry = &mut self.entries[idx];

        // Politica di Sostituzione: Scrive sopra se lo slot è vuoto/diverso, 
        // se la nuova mossa deriva da una ricerca più profonda, o se la voce appartiene a una ricerca passata (invecchiamento)[cite: 6].
        if entry.key != key || depth as u8 >= entry.depth || entry.generation != self.generation {
            entry.key = key;
            entry.score = score;
            entry.depth = depth as u8;
            entry.bound = bound as u8;
            entry.generation = self.generation;
            
            // Salva i dati binari compressi della mossa se questa non è nulla[cite: 6].
            if !best_move.is_null() {
                entry.move_data = best_move.data;
            }
        }
    }
}