// NOTA: NON usare "mod" qui, ma "use crate::"
use crate::board::{Scacchiera, Colore};
use crate::movegen::{self, Mossa};
use crate::search::Motore;
use crate::evaluation;

pub fn test_basic_moves() {
    println!("=== TEST MOSSE BASE ===");
    
    let board = Scacchiera::nuova();
    println!("{}", board);
    
    let moves = movegen::genera_e_filtra_mosse(&board);
    println!("Mosse legali nella posizione iniziale: {}", moves.len());
    
    // Verifica che ci siano 20 mosse legali (16 pedoni + 4 cavalli)
    assert_eq!(moves.len(), 20, "Dovrebbero esserci 20 mosse legali!");
    println!("[OK] Test mosse base superato!");
}

pub fn test_fen_parsing() {
    println!("\n=== TEST PARSING FEN ===");
    
    let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    let board = Scacchiera::da_fen(fen).expect("FEN parsing failed");
    println!("Posizione iniziale da FEN:");
    println!("{}", board);
    
    // Verifica che il colore attivo sia bianco
    assert!(matches!(board.colore_attivo(), Colore::Bianco));
    println!("[OK] Test FEN parsing superato!");
}

pub fn test_make_move() {
    println!("\n=== TEST ESEGUIRE MOSSE ===");
    
    let mut board = Scacchiera::nuova();
    
    // Trova la mossa e2-e4
    let moves = movegen::genera_e_filtra_mosse(&board);
    let e2_e4 = moves.iter().find(|m| {
        m.to_string() == "e2e4"
    }).expect("Mossa e2e4 non trovata!");
    
    println!("Prima della mossa e2-e4:");
    println!("{}", board);
    
    if movegen::esegui_mossa(&mut board, e2_e4) {
        println!("Dopo la mossa e2-e4:");
        println!("{}", board);
        
        // Verifica che il colore attivo sia nero
        assert!(matches!(board.colore_attivo(), Colore::Nero));
        println!("[OK] Test eseguire mosse superato!");
    } else {
        panic!("Fallito nell'eseguire la mossa e2e4");
    }
}

pub fn test_check_detection() {
    println!("\n=== TEST SCACCO ===");
    
    // Posizione di scacco (scacco al re nero)
    let fen = "rnbqkbnr/pppp1ppp/8/4p3/5P2/5N2/PPPPP1PP/RNBQKB1R b KQkq - 1 2";
    let board = Scacchiera::da_fen(fen).expect("FEN parsing failed");
    
    println!("Posizione di test:");
    println!("{}", board);
    
    // Il re nero dovrebbe essere in scacco
    assert!(board.re_in_scacco(Colore::Nero), "Il re nero dovrebbe essere in scacco!");
    println!("[OK] Test rilevamento scacco superato!");
}

pub fn test_search() {
    println!("\n=== TEST RICERCA ===");
    
    let board = Scacchiera::nuova();
    
    println!("Posizione iniziale:");
    println!("{}", board);
    
    // Cerca la migliore mossa con profondità 2
    let mut engine = Motore::nuovo();
    let tempo = std::time::Duration::from_secs(1);
    let best_move = engine.trova_mossa_migliore(&board, tempo);
    
    println!("Miglior mossa trovata: {:?}", best_move);
    println!("Nodi esplorati: {}", engine.nodi_visitati);
    
    if let Some(best_move) = best_move {
        println!("Mossa in notazione: {}", best_move);
    }
    
    println!("[OK] Test ricerca base superato!");
}

pub fn run_all_tests() {
    println!("AVVIO TEST MOTORE SCACCHISTICO");
    println!("{}", "=".repeat(50));
    
    test_fen_parsing();
    test_basic_moves();
    test_make_move();
    test_check_detection();
    test_search();
    
    println!("\n\n");
    println!("TUTTI I TEST SUPERATI!");
    println!("{}", "=".repeat(50));
}