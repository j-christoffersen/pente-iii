// TODO handle turn change

use std::fs::OpenOptions;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use crate::board::BoardState;
use crate::evaluation::{EvaluatedMoveSet, PatternScorer, WIN_SCORE};
use crate::tile::{TileType, PlayerType};

/// TEMPORARY: debug log of every move the AI considers during search, for
/// inspecting search behavior. Gitignored; delete `open_debug_log` and its
/// call sites (also used from `evaluation.rs`) to remove this instrumentation.
pub(crate) fn debug_log_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("ai_debug.log")
}

pub(crate) fn open_debug_log() -> BufWriter<std::fs::File> {
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(debug_log_path())
        .expect("failed to open AI debug log file");
    BufWriter::new(file)
}

/// After move ordering, only the top this many continuations are expanded each ply.
// pub const MOVE_SET_SIZE: usize = 16;
pub const MOVE_SET_SIZE: usize = 5;

#[cfg(test)]
pub const SEARCH_DEPTH: usize = 2;
#[cfg(not(test))]
pub const SEARCH_DEPTH: usize = 4;

// pub const BOUNDING_BOX_PADDING: usize = 6;
pub const BOUNDING_BOX_PADDING: usize = 3;


/// Engine entrypoint: holds a reusable pattern scorer for search.
#[derive(Clone, Debug)]
pub struct Search {
    scorer: PatternScorer,
}

impl Search {
    pub fn new(scorer: PatternScorer) -> Self {
        Self { scorer }
    }

    /// `depth` is plies of *opponent* lookahead baked into the returned
    /// score, on top of the move itself. `depth == 0` ranks candidates by
    /// their own immediate impact only, with no anticipated reply — that's
    /// the score that matches a static (no-search) evaluation of the
    /// resulting position. Each additional depth adds one more ply of the
    /// opponent's (then your own, etc.) best anticipated response.
    pub fn find_best_move(self, board: &BoardState, color: PlayerType, depth: usize) -> ((usize, usize), i32) {
        let base_evaluation = EvaluatedMoveSet::from_board_state(board, &self.scorer, color);

        let mut debug_log = open_debug_log();
        let _ = writeln!(debug_log, ">>>>>>>>>>>>>> find_best_move depth {depth}");
        debug_log.flush();
        self.evaluate_round_moves(&base_evaluation, depth)
    }

    fn evaluate_round_moves(&self, parent_ems: &EvaluatedMoveSet, depth: usize) -> ((usize, usize), i32) {
        // // TEMPORARY: search the entire board instead of the padded bounding box.
        // // To revert, uncomment the block below and delete the full-board block above it.
        // let (min_row, max_row, min_col, max_col) = (
        //     0,
        //     parent_ems.board.height - 1,
        //     0,
        //     parent_ems.board.width - 1,
        // );
        let (min_row, max_row, min_col, max_col) = (
            parent_ems.min_row.saturating_sub(BOUNDING_BOX_PADDING),
            (parent_ems.max_row + BOUNDING_BOX_PADDING).min(parent_ems.board.height - 1),
            parent_ems.min_col.saturating_sub(BOUNDING_BOX_PADDING),
            (parent_ems.max_col + BOUNDING_BOX_PADDING).min(parent_ems.board.width - 1),
        );

        let mut moves = Vec::new();
        for row in min_row..=max_row {
            for col in min_col..=max_col {
                if parent_ems.effective_tile_at(row, col) == TileType::Empty {
                    moves.push((row, col));
                }
            }
        }

        let turn_number = parent_ems.stone_count() + 1;
        let mut debug_log = open_debug_log();
        let last_move = parent_ems.move_coords.unwrap_or((999, 999));
        let _ = writeln!(debug_log, "evaluate_round_moves turn {turn_number} depth {depth} from last move ({}, {})", last_move.0, last_move.1);


        let mut ems_list: Vec<((usize, usize), EvaluatedMoveSet)> = vec![];
        for (r, c) in moves {
            let ems = EvaluatedMoveSet::from_parent(parent_ems, &self.scorer, parent_ems.board, r, c);
            let _ = writeln!(
                debug_log,
                "turn={turn_number} depth={depth} move=({r},{c}) score={} captures_white={} captures_black={} score_white_to_play={} score_black_to_play={}",
                ems.score, ems.captures_white, ems.captures_black, ems.score_white_to_play, ems.score_black_to_play
            );
            // ems.score is already the mover's own perspective (see comment
            // below), so a score at or above WIN_SCORE means this candidate
            // wins outright. Nothing can beat a win, so stop immediately
            // instead of evaluating remaining candidates or recursing.
            if ems.score >= WIN_SCORE {
                let _ = writeln!(debug_log, "turn={turn_number} depth={depth} move=({r},{c}) is a winning move, short-circuiting");
                return ((r, c), ems.score);
            }
            ems_list.push(((r, c), ems));
        }

        // Descending: `score` is from the *next* mover's (opponent's)
        // perspective, so the moves best for the current mover are the ones
        // where the opponent is left worst off — i.e. the lowest scores.
        let _ =writeln!(debug_log, "ems_list top 5 scores turn {turn_number} depth {depth}: {:?}", ems_list.iter().take(5).map(|(_, ems)| ems.score).collect::<Vec<i32>>());
        ems_list
        // .sort_by(|a, b| a.1.score.cmp(&b.1.score));
        .sort_by(|a, b| b.1.score.cmp(&a.1.score));
        let _ = writeln!(debug_log, "ems_list top 5 scores post sort turn {turn_number} depth {depth}: {:?}", ems_list.iter().take(5).map(|(_, ems)| ems.score).collect::<Vec<i32>>());
        debug_log.flush();

        // if depth = 0, scores are final. Otherwise, keep iterating recursively until depth is reached.
        let moves_with_scores: Vec<((usize, usize), i32)> = if depth == 0 {
            // ems.score is the *opponent's* (next mover's) perspective right
            // after this candidate move, same as every other ply here — negate
            // it to get the value for the player actually choosing this move.
            ems_list.iter().map(|(_, ems)| (ems.move_coords.unwrap(), ems.score)).collect()
        } else {
            // prune to the top MOVE_SET_SIZE moves and continue recursively
            let top_moves_from_current_round_eval: Vec<&((usize, usize), EvaluatedMoveSet)> = ems_list.iter()
            .take(MOVE_SET_SIZE)
            .collect();
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
        // TODO add randomness
        let best = *moves_with_scores.iter().max_by_key(|(_, score)| score).unwrap();
        let _ = writeln!(debug_log, "returning best move: {:?}", best);
        best
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