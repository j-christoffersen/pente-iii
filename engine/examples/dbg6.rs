use pente_engine::board::BoardState;
use pente_engine::evaluation::{default_automaton, EvaluatedMoveSet, PatternScorer};
use pente_engine::tile::{PlayerType, TileType};

fn main() {
    let mut board = BoardState::new(19, 19);
    for c in [8, 9, 10] {
        board.set_tile(9, c, TileType::White);
    }

    let (dfa, weights) = default_automaton();
    let scorer = PatternScorer::new(dfa, weights);
    let base = EvaluatedMoveSet::from_board_state(&board, &scorer, PlayerType::White);

    let mut scored: Vec<((usize, usize), i32)> = Vec::new();
    for row in 3..16 {
        for col in 3..16 {
            if board.get_tile(row, col) == TileType::Empty {
                let ems = EvaluatedMoveSet::from_parent(&base, &scorer, &board, row, col);
                // depth=0 ranking criterion used by find_best_move/evaluate_round_moves.
                scored.push(((row, col), -ems.score));
            }
        }
    }
    scored.sort_by_key(|(_, s)| std::cmp::Reverse(*s));
    for (mv, s) in scored.iter().take(10) {
        println!("{:?} value={}", mv, s);
    }

    println!("---");
    let at_11 = EvaluatedMoveSet::from_parent(&base, &scorer, &board, 9, 11);
    println!(
        "(9,11): score_white={} score_black={} value(-score)={}",
        at_11.score_white, at_11.score_black, -at_11.score
    );
    let at_12 = EvaluatedMoveSet::from_parent(&base, &scorer, &board, 9, 12);
    println!(
        "(9,12): score_white={} score_black={} value(-score)={}",
        at_12.score_white, at_12.score_black, -at_12.score
    );
}
