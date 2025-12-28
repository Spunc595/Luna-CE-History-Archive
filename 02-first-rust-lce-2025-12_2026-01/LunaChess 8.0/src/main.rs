// src/main.rs

// Dichiarazione dei moduli del progetto
// Assicurati che tutti questi file esistano nella cartella src/
mod board;
mod attacks;
mod movegen;
mod evaluation;
mod transposition;
mod search;
mod uci;

use crate::uci::UciEngine;
use std::env;

fn main() {
    // Gestione degli argomenti da riga di comando (opzionale)
    let args: Vec<String> = env::args().collect();

    // Se passiamo "bench" come argomento, potremmo eseguire un test di velocità
    if args.len() > 1 && args[1] == "bench" {
        esegui_benchmark();
        return;
    }

    // Avvio del motore in modalità UCI
    // Questa è la modalità standard utilizzata dalle GUI (Arena, GUI di scacchi, ecc.)
    let mut engine = UciEngine::nuova();
    engine.run();
}

/// Una semplice funzione di benchmark per testare i nodi al secondo (NPS)
/// dopo le ottimizzazioni (Zobrist, Make/Unmake, Lazy SMP)
fn esegui_benchmark() {
    println!("Avvio Benchmark RustChess PRO...");
    
    use crate::board::Scacchiera;
    use crate::search::Motore;
    use crate::transposition::TranspositionTable;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    use std::time::Instant;

    let mut board = Scacchiera::nuova();
    let tt = Arc::new(TranspositionTable::new(64)); // 64MB TT
    let mut motore = Motore::nuovo(tt);
    motore.set_stop_flag(Arc::new(AtomicBool::new(false)));

    let start = Instant::now();
    
    // Eseguiamo una ricerca a profondità fissa per il test
    let profondita_test = 8;
    println!("Ricerca profondità {}...", profondita_test);
    
    motore.trova_mossa_migliore(&mut board, 10000, true);
    
    let durata = start.elapsed();
    let nodi = motore.nodi_visitati;
    let nps = (nodi as f64 / durata.as_secs_f64()) as u64;

    println!("---------------------------");
    println!("Benchmark Completato:");
    println!("Nodi totali: {}", nodi);
    println!("Tempo: {:?}", durata);
    println!("NPS: {} (Nodi al secondo)", nps);
    println!("---------------------------");
}