use std::fs;

fn main() {
    let input = fs::read_to_string("polyglot_raw.txt").expect("impossibile leggere input");
    let mut output = String::from("// Generato automaticamente da polyglot_raw.txt\n");
    output.push_str("pub const RANDOM64: [u64; 781] = [\n");

    for token in input.split(|c| c == ',' || c == '{' || c == '}') {
        let t = token.trim();
        if t.is_empty() { continue; }
        // Rimuove eventuali "ULL" o spazi
        let cleaned = t.replace("ULL", "").replace("ulL", "").replace("UL", "");
        output.push_str("    ");
        output.push_str(&cleaned);
        output.push_str(",\n");
    }

    output.push_str("];\n\n");
    output.push_str("pub struct PolyglotZobrist;\n\n");
    output.push_str("impl PolyglotZobrist {\n");
    output.push_str("    #[inline(always)] pub fn piece_key(piece_index: usize, square: usize) -> u64 { RANDOM64[64 * piece_index + square] }\n");
    output.push_str("    #[inline(always)] pub fn castle_key(index: usize) -> u64 { RANDOM64[768 + index] }\n");
    output.push_str("    #[inline(always)] pub fn ep_file_key(file: usize) -> u64 { RANDOM64[772 + file] }\n");
    output.push_str("    #[inline(always)] pub fn side_to_move() -> u64 { RANDOM64[780] }\n");
    output.push_str("}\n");

    fs::write("polyglot_zobrist.rs", output).expect("errore salvataggio output");
}
