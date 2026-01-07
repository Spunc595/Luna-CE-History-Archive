use std::time::{SystemTime, UNIX_EPOCH};

// Funzione principale: riceve la stringa delle mosse e restituisce la mossa da libro
pub fn get_book_move_str(history_str: &str) -> Option<String> {
    let history = history_str.trim();

    match history {
        // --- 1. PRIMA MOSSA ---
        "" => choose(&["e2e4", "d2d4"]),

        // --- RISPOSTE A 1. e4 ---
        "e2e4" => choose(&["e7e5", "c7c5", "e7e6", "c7c6"]), 

        // --- PARTITA DI RE ---
        "e2e4 e7e5" => Some("g1f3"),
        "e2e4 e7e5 g1f3" => choose(&["b8c6", "g8f6", "d7d6"]),
        "e2e4 e7e5 g1f3 b8c6" => choose(&["f1b5", "f1c4", "d2d4"]), 
        
        // Spagnola
        "e2e4 e7e5 g1f3 b8c6 f1b5" => choose(&["a7a6", "g8f6"]),
        "e2e4 e7e5 g1f3 b8c6 f1b5 a7a6" => Some("b5a4"),
        "e2e4 e7e5 g1f3 b8c6 f1b5 a7a6 b5a4" => Some("g8f6"),
        "e2e4 e7e5 g1f3 b8c6 f1b5 a7a6 b5a4 g8f6" => Some("e1g1"),

        // Italiana
        "e2e4 e7e5 g1f3 b8c6 f1c4" => choose(&["f8c5", "g8f6"]),
        "e2e4 e7e5 g1f3 b8c6 f1c4 f8c5" => choose(&["c2c3", "e1g1", "d2d3"]),

        // --- SICILIANA ---
        "e2e4 c7c5" => choose(&["g1f3", "b1c3"]),
        "e2e4 c7c5 g1f3" => choose(&["d7d6", "b8c6", "e7e6"]),
        "e2e4 c7c5 g1f3 d7d6" => Some("d2d4"),
        "e2e4 c7c5 g1f3 d7d6 d2d4" => Some("c5d4"),
        "e2e4 c7c5 g1f3 d7d6 d2d4 c5d4 f3d4" => Some("g8f6"),
        "e2e4 c7c5 g1f3 d7d6 d2d4 c5d4 f3d4 g8f6" => Some("b1c3"),
        "e2e4 c7c5 g1f3 d7d6 d2d4 c5d4 f3d4 g8f6 b1c3" => choose(&["a7a6", "g7g6", "b8c6"]),

        // --- FRANCESE ---
        "e2e4 e7e6" => Some("d2d4"),
        "e2e4 e7e6 d2d4" => Some("d7d5"),
        "e2e4 e7e6 d2d4 d7d5" => choose(&["b1c3", "e4e5", "e4d5"]),

        // --- PARTITA DI DONNA ---
        "d2d4" => choose(&["g8f6", "d7d5"]),
        "d2d4 d7d5" => choose(&["c2c4", "g1f3", "c1f4"]), 
        "d2d4 d7d5 c2c4" => choose(&["e7e6", "c7c6"]),
        "d2d4 d7d5 c2c4 e7e6" => Some("b1c3"),
        "d2d4 d7d5 c2c4 c7c6" => Some("g1f3"),

        // Indiane
        "d2d4 g8f6" => choose(&["c2c4", "g1f3"]),
        "d2d4 g8f6 c2c4" => choose(&["e7e6", "g7g6"]),
        "d2d4 g8f6 c2c4 g7g6" => Some("b1c3"),
        "d2d4 g8f6 c2c4 g7g6 b1c3" => Some("f8g7"),

        _ => None,
    }
    .map(|s| s.to_string())
}

// CORREZIONE QUI: usa `&'static str`
fn choose(moves: &[&'static str]) -> Option<&'static str> {
    if moves.is_empty() { return None; }
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().subsec_nanos();
    let idx = (nanos as usize) % moves.len();
    Some(moves[idx])
}