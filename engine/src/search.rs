// TODO handle turn change

use crate::board::BoardState;
use crate::evaluation::{EvaluatedMoveSet, PatternScorer};
use crate::tile::{TileType, PlayerType};

/// After move ordering, only the top this many continuations are expanded each ply.
pub const MOVE_SET_SIZE: usize = 16;

#[cfg(test)]
pub const SEARCH_DEPTH: usize = 2;
#[cfg(not(test))]
pub const SEARCH_DEPTH: usize = 4;

pub const BOUNDING_BOX_PADDING: usize = 6;

/// Engine entrypoint: holds a reusable pattern scorer for search.
#[derive(Clone, Debug)]
pub struct Search {
    scorer: PatternScorer,
}

impl Search {
    pub fn new(scorer: PatternScorer) -> Self {
        Self { scorer }
    }

    pub fn find_best_move(self, board: &BoardState, color: PlayerType, depth: usize) -> ((usize, usize), i32) {
        debug_assert!(depth >= 1, "Depth must be at least 1");

        let base_evaluation = EvaluatedMoveSet::from_board_state(board, &self.scorer, color);
        
        self.evaluate_round_moves(&base_evaluation, depth)
    }

    fn evaluate_round_moves(&self, parent_ems: &EvaluatedMoveSet, depth: usize) -> ((usize, usize), i32) {
        let (min_row, max_row, min_col, max_col) = (
            parent_ems.min_row.saturating_sub(BOUNDING_BOX_PADDING),
            (parent_ems.max_row + BOUNDING_BOX_PADDING).min(parent_ems.board.height - 1),
            parent_ems.min_col.saturating_sub(BOUNDING_BOX_PADDING),
            (parent_ems.max_col + BOUNDING_BOX_PADDING).min(parent_ems.board.width - 1),
        );

        let mut moves = Vec::new();
        for row in min_row..=max_row {
            for col in min_col..=max_col {
                if parent_ems.board.get_tile(row, col) == TileType::Empty {
                    moves.push((row, col));
                }
            }
        }
        
        let mut ems_list: Vec<((usize, usize), EvaluatedMoveSet)> = vec![];
        for (r, c) in moves {
            let ems = EvaluatedMoveSet::from_parent(parent_ems, &self.scorer, parent_ems.board, r, c);
            ems_list.push(((r, c), ems));
        }

        ems_list.sort_by(|a, b| a.1.score.cmp(&b.1.score));

        // if depth = 0, scores are final. Otherwise, keep iterating recursively until depth is reached.
        let moves_with_scores: Vec<((usize, usize), i32)> = if depth == 0 {
            ems_list.iter().map(|(_, ems)| (ems.move_coords.unwrap(), ems.score)).collect()
        } else {
            // prune to the top MOVE_SET_SIZE moves and continue recursively
            let top_moves_from_current_round_eval: Vec<&((usize, usize), EvaluatedMoveSet)> = ems_list.iter().take(MOVE_SET_SIZE).collect();
            let mut moves_with_scores = Vec::new();
            for (mv, ems) in top_moves_from_current_round_eval {
                let (r, c) = *mv;
                let (_, score) = self.evaluate_round_moves(ems, depth - 1);

                // score returned will be the best score for the opponent, so we need to multiply by -1 to get the best score for the current player
                moves_with_scores.push(((r, c), -score));
            }
            moves_with_scores
        };

        // return the "top" move and its score
        *moves_with_scores.iter().max_by_key(|(_, score)| score).unwrap()
    }
}

mod tests {
    use super::*;
    use crate::evaluation::default_automaton;
    use crate::tile::TileType;

    fn test_search() -> Search {
        let (dfa, weights) = default_automaton();
        Search::new(PatternScorer::new(dfa, weights))
    }

    #[test]
    fn find_best_move_on_empty_board_returns_empty_cell() {
        let board = BoardState::new(15, 15);
        let search = test_search();

        let ((row, col), _score) =
            search.find_best_move(&board, PlayerType::Black, 1);

        assert!(row < board.height);
        assert!(col < board.width);
        assert_eq!(board.get_tile(row, col), TileType::Empty);
    }

    #[test]
    fn find_best_move_near_existing_stone() {
        let mut board = BoardState::new(15, 15);
        board.set_tile(7, 7, TileType::Black);
        let search = test_search();

        let ((row, col), _score) =
            search.find_best_move(&board, PlayerType::White, 1);

        assert_eq!(board.get_tile(row, col), TileType::Empty);
        assert!(
            row.abs_diff(7) <= BOUNDING_BOX_PADDING + 1
                && col.abs_diff(7) <= BOUNDING_BOX_PADDING + 1,
            "expected move near (7, 7), got ({row}, {col})"
        );
    }

    #[test]
    fn find_best_move_completes_at_search_depth() {
        let board = BoardState::new(15, 15);
        let search = test_search();

        let ((row, col), _score) =
            search.find_best_move(&board, PlayerType::Black, SEARCH_DEPTH);

        assert!(row < board.height);
        assert!(col < board.width);
        assert_eq!(board.get_tile(row, col), TileType::Empty);
    }
}