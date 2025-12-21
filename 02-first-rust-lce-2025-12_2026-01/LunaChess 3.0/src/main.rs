mod board;
mod movegen;
mod search;
mod evaluation;
mod uci;
mod transposition;
mod zobrist;
mod opening_book;

fn main() {
    println!("Motore di Scacchi Rust v1.0 con Aperture");
    
    // TEST DI DEBUG - Aggiungi questa sezione
    {
        use crate::board::Scacchiera;
        use crate::movegen::genera_mosse;
        use crate::board::{Pezzo, Colore};
        
        println!("\n=== DEBUG TEST ===");
        let scacchiera = Scacchiera::nuova();
        scacchiera.stampa();
        
        let mosse = genera_mosse(&scacchiera);
        println!("\nMosse totali generate: {}", mosse.len());
        
        // Conta mosse per tipo di pezzo
        let mut conteggio_per_pezzo = std::collections::HashMap::new();
        for mossa in &mosse {
            if let Some((pezzo, _)) = scacchiera.ottieni_pezzo(mossa.da) {
                *conteggio_per_pezzo.entry(pezzo).or_insert(0) += 1;
            }
        }
        
        println!("\nConteggio mosse per tipo di pezzo:");
        for (pezzo, count) in &conteggio_per_pezzo {
            println!("  {:?}: {} mosse", pezzo, count);
        }
        
        // Mosse dei pedoni bianchi
        println!("\nMosso dei pedoni bianchi:");
        for mossa in &mosse {
            if let Some((pezzo, colore)) = scacchiera.ottieni_pezzo(mossa.da) {
                if pezzo == Pezzo::Pedone && colore == Colore::Bianco {
                    println!("  {} -> {}", mossa.da, mossa.a);
                }
            }
        }
    }
    
    // Continua con UCI normale
    let mut uci_handler = uci::UciHandler::new();
    uci_handler.run();
}