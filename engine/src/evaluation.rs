//! Incremental and full-board pattern scoring using [`crate::aho_corasick::TileDfa`].

use std::collections::HashMap;

use crate::aho_corasick::TileDfa;
use crate::board::BoardState;
use crate::tile::TileType;

/// Overlay moves keyed by flat board index for O(1) lookup. Values are [`TileType::White`] or
/// [`TileType::Black`] (a stone played on top of the board grid).
pub type MoveMap = HashMap<usize, TileType>;

/// Pattern scores aligned with [`default_automaton`] insertion order.
pub type PatternWeights = Vec<i32>;

/// A board position plus overlay stones and the composed pattern score.
#[derive(Clone, Debug)]
pub struct EvaluatedMoveSet {
    pub board: BoardState,
    /// Stones played on top of `board`; key is [`BoardState::index`].
    pub moves: MoveMap,
    /// Play order; last entry is the most recent move (for incremental evaluation).
    pub move_order: Vec<(usize, TileType)>,
    pub score: i32,
    pub scorer: PatternScorer
}

impl EvaluatedMoveSet {
    // initalize an EvaluatedMoveSet from a BoardState, evaluating the full board
    pub fn from_board_state(
        board: BoardState,
        scorer: PatternScorer
    ) -> Self {
        let mut map = MoveMap::with_capacity(0);

        let mut this = Self {
            board,
            moves: map,
            move_order: vec![],
            score: 0,
            scorer,
        };
        this.score = this.evaluate_full();
        this
    }

    // Add a new move to a parent EvaluatedMoveSet. Evaluates boardd on the delta of the moves
    pub fn from_parent(
        parent: EvaluatedMoveSet,
        row: usize,
        col: usize,
        tile: TileType,
    ) -> Self {
        debug_assert!(
            matches!(tile, TileType::Black | TileType::White),
            "Move must not be empty"
        );

        let idx = parent.board.index(row, col);
        let mut moves = parent.moves.clone();
        moves.insert(idx, tile);
        let mut move_order = parent.move_order.clone();
        move_order.push((idx, tile));

        let mut this = Self {
            board: parent.board,
            moves,
            move_order,
            score: 0,
            scorer: parent.scorer
        };

        let before = |r: usize, c: usize| this.tile_at(r, c);
        let after = |r: usize, c: usize| if r == row && c == col { TileType::Empty } else { this.tile_at(r, c) };

        let delta = this.local_score(row, col, after) - this.local_score(row, col, before);

        this.score = parent.score + delta;

        this
    }

    fn tile_at(&self, row: usize, col: usize) -> TileType {
        let i = self.board.index(row, col);
        self.moves
            .get(&i)
            .copied()
            .unwrap_or_else(|| self.board.get_tile(row, col))
    }

    fn evaluate_full(&self) -> i32 {
        let mut total = 0i32;

        for row in 0..self.board.height {
            let line: Vec<TileType> = (0..self.board.width).map(|c| self.tile_at(row, c)).collect();
            total += self.scorer.score_line(&line);
        }

        for col in 0..self.board.width {
            let line: Vec<TileType> = (0..self.board.height).map(|r| self.tile_at(r, col)).collect();
            total += self.scorer.score_line(&line);
        }

        // Down-right (\): starts along top row then left column.
        for start_col in 0..self.board.width {
            let mut line = Vec::new();
            let mut r = 0usize;
            let mut c = start_col;
            while r < self.board.height && c < self.board.width {
                line.push(self.tile_at(r, c));
                r += 1;
                c += 1;
            }
            total += self.scorer.score_line(&line);
        }
        for start_row in 1..self.board.height {
            let mut line = Vec::new();
            let mut r = start_row;
            let mut c = 0usize;
            while r < self.board.height && c < self.board.width {
                line.push(self.tile_at(r, c));
                r += 1;
                c += 1;
            }
            total += self.scorer.score_line(&line);
        }

        // Down-left (/): starts along top row then right column.
        for start_col in 0..self.board.width {
            let mut line = Vec::new();
            let mut r = 0usize;
            let mut c = start_col as isize;
            while r < self.board.height && c >= 0 {
                line.push(self.tile_at(r, c as usize));
                r += 1;
                c -= 1;
            }
            total += self.scorer.score_line(&line);
        }
        for start_row in 1..self.board.height {
            let mut line = Vec::new();
            let mut r = start_row;
            let mut c = (self.board.width - 1) as isize;
            while r < self.board.height && c >= 0 {
                line.push(self.tile_at(r, c as usize));
                r += 1;
                c -= 1;
            }
            total += self.scorer.score_line(&line);
        }

        total
    }

    fn local_score(
        &self,
        row: usize,
        col: usize,
        get: impl Fn(usize, usize) -> TileType,
    ) -> i32 {
        let mut total = 0i32;

        // Horizontal
        let c0 = col.saturating_sub(5);
        let c1 = (col + 5).min(self.board.width - 1);
        let row_line: Vec<TileType> = (c0..=c1).map(|c| get(row, c)).collect();
        total += self.scorer.score_line(&row_line);

        // Vertical
        let r0 = row.saturating_sub(5);
        let r1 = (row + 5).min(self.board.height - 1);
        let col_line: Vec<TileType> = (r0..=r1).map(|r| get(r, col)).collect();
        total += self.scorer.score_line(&col_line);

        // Diagonal \ -> Rn = Cn - col + row
        let r0 = row.saturating_sub(5)
          .max(col.saturating_sub(5) + row - col);
        let r1 = (row + 5)
          .min(self.board.height - 1)
          .min(self.board.width - 1 + row - col);
        let diagonal_line: Vec<TileType> = (r0..=r1).map(|r| get(r, r + col - row)).collect();
        total += self.scorer.score_line(&diagonal_line);

        // Diagonal / -> Rn = -Cn + col + row
        let r0 = row.saturating_sub(5)
          .max(row + col - col.saturating_sub(5));
        let r1 = (row + 5)
          .min(self.board.height - 1)
          .min(row + col - (self.board.width - 1));
        let diagonal_line: Vec<TileType> = (r0..=r1).map(|r| get(r, row + col - r)).collect();
        total += self.scorer.score_line(&diagonal_line);

        total
    }
}


// uses a DFA to calculate a score given a pattern of inputs
#[derive(Debug, Clone)]
pub struct PatternScorer {
    dfa: TileDfa,
    weights: PatternWeights
}
impl PatternScorer {
    fn score_line(&self, line: &[TileType]) -> i32 {
        let mut sum = 0i32;
        for m in self.dfa.find_matches(line) {
            sum = sum.saturating_add(self.weights[m.pattern_id]);
        }
        sum
    }
}

/// Pattern strings use `0` = empty, `1` = white, `2` = black (see `patterns.toml`).
fn parse_pattern(s: &str) -> Vec<TileType> {
    s.chars()
        .map(|ch| match ch {
            '0' => TileType::Empty,
            '1' => TileType::White,
            '2' => TileType::Black,
            _ => panic!("bad pattern char {ch:?} in {s:?}"),
        })
        .collect()
}

/// Builds the automaton and weight table from the engine pattern set.
pub fn default_automaton() -> (TileDfa, PatternWeights) {
    let specs: &[(&str, i32)] = &[
        ("120", 5_i32.pow(0)),
        ("210", -(5_i32.pow(0))),
        ("010", 5_i32.pow(1)),
        ("020", -(5_i32.pow(1))),
        ("2110", -(5_i32.pow(3)) + 5),
        ("1220", (5_i32.pow(3)) - (5_i32.pow(2))),
        ("0110", 5_i32.pow(2)),
        ("0220", -(5_i32.pow(2))),
        ("21110", 5_i32.pow(2)),
        ("12220", -(5_i32.pow(2))),
        ("21010", 5_i32.pow(0)),
        ("12020", -(5_i32.pow(0))),
        ("01110", 5_i32.pow(3)),
        ("02220", -(5_i32.pow(5))),
        ("01010", 5_i32.pow(1)),
        ("02020", -(5_i32.pow(1))),
        ("211110", 5_i32.pow(3)),
        ("122220", -(5_i32.pow(7))),
        ("210110", 5_i32.pow(2)),
        ("120220", -(5_i32.pow(2))),
        ("211010", 5_i32.pow(2)),
        ("122020", -(5_i32.pow(2))),
        ("011110", 5_i32.pow(6)),
        ("022220", -(5_i32.pow(7))),
        ("010110", 5_i32.pow(3)),
        ("020220", -(5_i32.pow(5))),
        ("11011", 5_i32.pow(1)),
        ("22022", -(5_i32.pow(7))),
        ("11101", 5_i32.pow(3)),
        ("22202", -(5_i32.pow(7))),
    ];

    let mut ac = TileDfa::new();
    let mut weights = Vec::with_capacity(specs.len());
    for (pat, w) in specs {
        ac.add_pattern(&parse_pattern(pat));
        weights.push(*w);
    }
    ac.build();
    (ac, weights)
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::tile::TileType::{Black, Empty, White};

//     #[test]
//     fn incremental_matches_full_one_stone() {
//         let (ac, w) = default_automaton();
//         let mut board = BoardState::new(15, 15);
//         board.set_tile(7, 7, Black);
//         refresh_board_score(&mut board, &ac, &w);

//         let idx = board.index(7, 8);
//         let full = EvaluatedMoveSet::from_board_state(board.clone(), vec![(idx, White)], &ac, &w);

//         let parent = EvaluatedMoveSet::from_board_state(board, vec![], &ac, &w);
//         let inc = EvaluatedMoveSet::from_parent(&parent, 7, 8, White, &ac, &w);

//         assert_eq!(full.score, inc.score);
//     }

//     #[test]
//     fn seeded_from_board_matches_full_two_moves() {
//         let (ac, w) = default_automaton();
//         let mut board = BoardState::new(9, 9);
//         board.set_tile(4, 4, Black);
//         refresh_board_score(&mut board, &ac, &w);

//         let moves = vec![
//             (board.index(4, 5), White),
//             (board.index(4, 6), Black),
//         ];
//         let mut map = MoveMap::new();
//         for &(idx, t) in &moves {
//             map.insert(idx, t);
//         }
//         let expected = evaluate_full(&board, &map, &ac, &w);
//         let got = EvaluatedMoveSet::from_board_state(board, moves, &ac, &w).score;
//         assert_eq!(got, expected);
//     }

//     #[test]
//     fn move_map_lookup_o1() {
//         let (ac, w) = default_automaton();
//         let board = BoardState::new(5, 5);
//         let idx = board.index(2, 3);
//         let ems = EvaluatedMoveSet::from_board_state(board, vec![(idx, Black)], &ac, &w);
//         assert_eq!(ems.move_at(2, 3), Some(Black));
//         assert_eq!(ems.move_at(0, 0), None);
//     }
// }
