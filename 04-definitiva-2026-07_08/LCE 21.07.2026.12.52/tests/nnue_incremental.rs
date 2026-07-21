// Verifica che l'accumulatore NNUE incrementale (board.rs::esegui_mossa/
// annulla_mossa + nnue.rs::Accumulator::add_piece/remove_piece) produca
// esattamente lo stesso risultato di un ricalcolo completo (LunaNNUE::refresh)
// in ogni posizione raggiunta, sia in avanti (make) che all'indietro (unmake).
// Copre mosse silenziose, catture, en passant, arrocco e promozione
// (inclusa promozione con cattura).

use luna::board::Scacchiera;
use luna::nnue::LunaNNUE;
use luna::zobrist::ZobristKeys;
use std::io::Write;

const INPUT_SIZE: usize = 768;
const L1_SIZE: usize = 256;
const L2_SIZE: usize = 32;

/// Crea una rete con pesi pseudo-randomici (deterministici) scritta su un
/// file temporaneo e caricata tramite LunaNNUE::load, l'unico costruttore
/// pubblico disponibile. I valori concreti dei pesi sono irrilevanti per
/// questo test: interessa solo che add_piece/remove_piece siano l'esatto
/// inverso l'uno dell'altro e coerenti con refresh().
fn load_test_net(seed: u64) -> LunaNNUE {
    let total = INPUT_SIZE * L1_SIZE + L1_SIZE + L1_SIZE * L2_SIZE + L2_SIZE + L2_SIZE + 1;
    let mut state = seed ^ 0x9E3779B97F4A7C15;
    let mut bytes = Vec::with_capacity(total * 2);
    for _ in 0..total {
        // xorshift64 minimale, solo per generare pesi deterministici e
        // variati (positivi e negativi) senza dipendere dal crate `rand`.
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let val = (state as i64 % 2000 - 1000) as i16;
        bytes.extend_from_slice(&val.to_le_bytes());
    }

    let path = std::env::temp_dir().join(format!("luna_test_weights_{}.nnue", seed));
    let mut f = std::fs::File::create(&path).unwrap();
    f.write_all(&bytes).unwrap();
    drop(f);

    let net = LunaNNUE::load(path.to_str().unwrap()).expect("caricamento rete di test fallito");
    let _ = std::fs::remove_file(&path);
    net
}

/// L'accumulatore incrementale della board deve sempre coincidere,
/// elemento per elemento, con un ricalcolo completo da zero.
fn assert_acc_consistent(net: &LunaNNUE, board: &Scacchiera, label: &str) {
    let full = net.refresh(board);
    assert_eq!(board.nnue_acc.v, full.v, "accumulatore incrementale disallineato: {}", label);

    let incremental_score = net.evaluate_from_accumulator(&board.nnue_acc, board.turno);
    let full_score = net.evaluate(board);
    assert_eq!(incremental_score, full_score, "punteggio disallineato: {}", label);
}

fn play_and_check(net: &LunaNNUE, board: &mut Scacchiera, z: &ZobristKeys, uci_move: &str) {
    let legali = board.genera_mosse_legali(z, Some(net));
    let m = legali.iter().find(|m| m.to_uci() == uci_move)
        .unwrap_or_else(|| panic!("mossa {} non legale in {}", uci_move, board.to_fen()));
    let applied = board.esegui_mossa(m, z, Some(net));
    assert!(applied, "la mossa {} avrebbe dovuto essere applicabile", uci_move);
    assert_acc_consistent(net, board, &format!("dopo {}", uci_move));
}

#[test]
fn incremental_accumulator_matches_full_refresh_quiet_and_capture() {
    let z = ZobristKeys::default();
    let net = load_test_net(1);
    let mut board = Scacchiera::new_iniziale(&z);
    board.refresh_nnue(Some(&net));
    assert_acc_consistent(&net, &board, "posizione iniziale");

    // Sequenza con mosse silenziose e una cattura.
    let mosse = ["e2e4", "e7e5", "g1f3", "b8c6", "f1b5", "a7a6", "b5c6", "d7c6"];

    let mut undo_stack: Vec<luna::board::Mossa> = Vec::new();
    for &mv in mosse.iter() {
        let legali = board.genera_mosse_legali(&z, Some(&net));
        let m = *legali.iter().find(|m| m.to_uci() == mv)
            .unwrap_or_else(|| panic!("mossa {} non legale in {}", mv, board.to_fen()));
        assert!(board.esegui_mossa(&m, &z, Some(&net)));
        assert_acc_consistent(&net, &board, &format!("dopo {}", mv));
        undo_stack.push(m);
    }

    // Annulliamo tutto: l'accumulatore deve tornare esattamente a quello
    // della posizione iniziale ad ogni passo.
    while let Some(m) = undo_stack.pop() {
        board.annulla_mossa(&m, &z, Some(&net));
        assert_acc_consistent(&net, &board, "durante l'unmake");
    }

    let mut start = Scacchiera::new_iniziale(&z);
    start.refresh_nnue(Some(&net));
    assert_eq!(board.nnue_acc.v, start.nnue_acc.v, "unmake completo non riporta alla posizione iniziale");
}

#[test]
fn incremental_accumulator_matches_full_refresh_en_passant() {
    let z = ZobristKeys::default();
    let net = load_test_net(2);
    let mut board = Scacchiera::new_iniziale(&z);
    board.refresh_nnue(Some(&net));

    for mv in ["e2e4", "a7a6", "e4e5", "d7d5", "e5d6"] {
        play_and_check(&net, &mut board, &z, mv);
    }
}

#[test]
fn incremental_accumulator_matches_full_refresh_castling() {
    let z = ZobristKeys::default();
    let net = load_test_net(3);
    // Posizione con arrocco corto immediatamente disponibile per il bianco.
    let mut board = Scacchiera::from_fen(
        "r1bqkbnr/pppp1ppp/2n5/4p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 4 4",
        &z,
    );
    board.refresh_nnue(Some(&net));
    assert_acc_consistent(&net, &board, "posizione pre-arrocco");

    play_and_check(&net, &mut board, &z, "e1g1");
}

#[test]
fn incremental_accumulator_matches_full_refresh_promotion_capture() {
    let z = ZobristKeys::default();
    let net = load_test_net(4);
    // Pedone bianco su b7 pronto a promuovere catturando la torre su a8.
    let mut board = Scacchiera::from_fen("r3k3/1P6/8/8/8/8/8/4K3 w - - 0 1", &z);
    board.refresh_nnue(Some(&net));
    assert_acc_consistent(&net, &board, "posizione pre-promozione");

    play_and_check(&net, &mut board, &z, "b7a8q");
}
