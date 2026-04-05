use crate::board::Bitboard;
use std::sync::OnceLock;

// -----------------------------------------------------
//              MAGIC BITBOARDS PER SLIDER
// -----------------------------------------------------

#[derive(Clone, Debug)]
pub struct MagicEntry {
    pub mask: Bitboard,
    pub magic: u64,
    pub shift: u32,
    pub attacks: Vec<Bitboard>,
}

pub struct MagicTables {
    pub bishop: [MagicEntry; 64],
    pub rook: [MagicEntry; 64],
}

static MAGIC: OnceLock<MagicTables> = OnceLock::new();

#[inline(always)]
pub fn bishop_attacks(sq: usize, occ: Bitboard) -> Bitboard {
    let entry = &MAGIC.get().expect("Magic tables non inizializzate").bishop[sq];
    let idx = ((occ & entry.mask).wrapping_mul(entry.magic)) >> entry.shift;
    entry.attacks[idx as usize]
}

#[inline(always)]
pub fn rook_attacks(sq: usize, occ: Bitboard) -> Bitboard {
    let entry = &MAGIC.get().expect("Magic tables non inizializzate").rook[sq];
    let idx = ((occ & entry.mask).wrapping_mul(entry.magic)) >> entry.shift;
    entry.attacks[idx as usize]
}

#[inline(always)]
pub fn queen_attacks(sq: usize, occ: Bitboard) -> Bitboard {
    bishop_attacks(sq, occ) | rook_attacks(sq, occ)
}

pub fn init_magic() {
    MAGIC.get_or_init(|| {
        MagicTables {
            bishop: init_bishop_tables(),
            rook: init_rook_tables(),
        }
    });
}

// -----------------------------------------------------
//              CALCOLO MASCHERE (CORRETTO)
// -----------------------------------------------------

fn bishop_mask(sq: usize) -> Bitboard {
    let r = (sq / 8) as i32;
    let f = (sq % 8) as i32;
    let mut mask = 0u64;

    // Diagonali: escludiamo i bordi esterni per la maschera (standard Magic)
    for &(dr, df) in &[(1, 1), (1, -1), (-1, 1), (-1, -1)] {
        let mut nr = r + dr;
        let mut nf = f + df;
        while nr > 0 && nr < 7 && nf > 0 && nf < 7 {
            mask |= 1u64 << (nr * 8 + nf);
            nr += dr;
            nf += df;
        }
    }
    mask
}

fn rook_mask(sq: usize) -> Bitboard {
    let r = (sq / 8) as i32;
    let f = (sq % 8) as i32;
    let mut mask = 0u64;

    for &(dr, df) in &[(1, 0), (-1, 0), (0, 1), (0, -1)] {
        let mut nr = r + dr;
        let mut nf = f + df;
        // Correzione: usiamo i valori dereferenziati dr e df
        while (nr > 0 && nr < 7 && df == 0) || (nf > 0 && nf < 7 && dr == 0) {
            mask |= 1u64 << (nr * 8 + nf);
            nr += dr;
            nf += df;
        }
    }
    mask
}

// -----------------------------------------------------
//          ATTACCHI ON-THE-FLY (RILEVAMENTO REALE)
// -----------------------------------------------------

fn bishop_attacks_on_the_fly(sq: usize, occ: Bitboard) -> Bitboard {
    let r = (sq / 8) as i32;
    let f = (sq % 8) as i32;
    let mut attacks = 0u64;

    for &(dr, df) in &[(1, 1), (1, -1), (-1, 1), (-1, -1)] {
        let mut nr = r + dr;
        let mut nf = f + df;
        while nr >= 0 && nr < 8 && nf >= 0 && nf < 8 {
            let bit = 1u64 << (nr * 8 + nf);
            attacks |= bit;
            if (occ & bit) != 0 { break; }
            nr += dr;
            nf += df;
        }
    }
    attacks
}

fn rook_attacks_on_the_fly(sq: usize, occ: Bitboard) -> Bitboard {
    let r = (sq / 8) as i32;
    let f = (sq % 8) as i32;
    let mut attacks = 0u64;

    for &(dr, df) in &[(1, 0), (-1, 0), (0, 1), (0, -1)] {
        let mut nr = r + dr;
        let mut nf = f + df;
        while nr >= 0 && nr < 8 && nf >= 0 && nf < 8 {
            let bit = 1u64 << (nr * 8 + nf);
            attacks |= bit;
            if (occ & bit) != 0 { break; }
            nr += dr;
            nf += df;
        }
    }
    attacks
}

// -----------------------------------------------------
//          GENERAZIONE TABELLE E MAGIC NUMBERS
// -----------------------------------------------------

fn index_to_occupancy(index: usize, mask: Bitboard) -> Bitboard {
    let mut occ = 0;
    let mut m = mask;
    let mut i = index;
    while m != 0 {
        let lsb = m.trailing_zeros();
        m &= m - 1;
        if (i & 1) != 0 {
            occ |= 1 << lsb;
        }
        i >>= 1;
    }
    occ
}

fn random_u64(seed: &mut u64) -> u64 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    *seed
}

fn random_magic(seed: &mut u64) -> u64 {
    random_u64(seed) & random_u64(seed) & random_u64(seed)
}

fn init_bishop_tables() -> [MagicEntry; 64] {
    let mut tables: Vec<MagicEntry> = Vec::with_capacity(64);
    let mut seed = 123456789;
    
    for sq in 0..64 {
        let mask = bishop_mask(sq);
        let bits = mask.count_ones();
        let size = 1 << bits;
        let shift = 64 - bits;
        
        let mut occs = vec![0; size];
        let mut real_attacks = vec![0; size];
        for i in 0..size {
            occs[i] = index_to_occupancy(i, mask);
            real_attacks[i] = bishop_attacks_on_the_fly(sq, occs[i]);
        }

        let magic = loop {
            let cand = random_magic(&mut seed);
            let mut attacks_table = vec![0; size];
            let mut used = vec![false; size];
            let mut fail = false;

            for i in 0..size {
                let idx = ((occs[i].wrapping_mul(cand)) >> shift) as usize;
                if used[idx] {
                    if attacks_table[idx] != real_attacks[i] {
                        fail = true;
                        break;
                    }
                } else {
                    used[idx] = true;
                    attacks_table[idx] = real_attacks[i];
                }
            }
            if !fail {
                break MagicEntry { mask, magic: cand, shift, attacks: attacks_table };
            }
        };
        tables.push(magic);
    }
    tables.try_into().unwrap_or_else(|_| panic!("Errore conversione tabelle"))
}

fn init_rook_tables() -> [MagicEntry; 64] {
    let mut tables: Vec<MagicEntry> = Vec::with_capacity(64);
    let mut seed = 987654321;
    
    for sq in 0..64 {
        let mask = rook_mask(sq);
        let bits = mask.count_ones();
        let size = 1 << bits;
        let shift = 64 - bits;
        
        let mut occs = vec![0; size];
        let mut real_attacks = vec![0; size];
        for i in 0..size {
            occs[i] = index_to_occupancy(i, mask);
            real_attacks[i] = rook_attacks_on_the_fly(sq, occs[i]);
        }

        let magic = loop {
            let cand = random_magic(&mut seed);
            let mut attacks_table = vec![0; size];
            let mut used = vec![false; size];
            let mut fail = false;

            for i in 0..size {
                let idx = ((occs[i].wrapping_mul(cand)) >> shift) as usize;
                if used[idx] {
                    if attacks_table[idx] != real_attacks[i] {
                        fail = true;
                        break;
                    }
                } else {
                    used[idx] = true;
                    attacks_table[idx] = real_attacks[i];
                }
            }
            if !fail {
                break MagicEntry { mask, magic: cand, shift, attacks: attacks_table };
            }
        };
        tables.push(magic);
    }
    tables.try_into().unwrap_or_else(|_| panic!("Errore conversione tabelle"))
}