pub struct EvalParams {
    pub piece_values_mg: [i32; 6],
    pub piece_values_eg: [i32; 6],
    pub passed_pawn_mg: [i32; 8],
    pub passed_pawn_eg: [i32; 8],
    pub mobility_mg: [i32; 6], // Bonus per ogni casa controllata
    pub mobility_eg: [i32; 6],
    pub king_shield_bonus: i32,
    pub king_open_file_penalty: i32,
}

pub const PARAMS: EvalParams = EvalParams {
    piece_values_mg: [82, 337, 365, 477, 1025, 0],
    piece_values_eg: [94, 281, 297, 512, 936, 0],
    passed_pawn_mg: [0, 5, 10, 20, 35, 60, 100, 0],
    passed_pawn_eg: [0, 10, 20, 40, 70, 130, 200, 0],
    mobility_mg: [0, 4, 4, 3, 2, 0], // I pezzi leggeri beneficiano di più della mobilità in MG
    mobility_eg: [0, 5, 5, 5, 5, 0],
    king_shield_bonus: 25,
    king_open_file_penalty: 40,
};