//! Incremental and full-board pattern scoring using [`crate::aho_corasick::TileDfa`].

use std::collections::HashMap;

use crate::aho_corasick::{TileDfa, TurnTileType};
use crate::board::BoardState;
use crate::tile::{PlayerType, TileType};


/// Overlay moves keyed by flat board index for O(1) lookup. Values are [`TileType::White`] or
/// [`TileType::Black`] (a stone played on top of the board grid).
pub type MoveMap = HashMap<usize, TileType>;

/// Pattern scores aligned with [`default_automaton`] insertion order.
pub type PatternWeights = Vec<i32>;

/// A board position plus overlay stones and the composed pattern score.
#[derive(Clone, Debug)]
pub struct EvaluatedMoveSet<'a> {
    pub board: &'a BoardState,
    /// Stones played on top of `board`; key is [`BoardState::index`].
    pub moves: MoveMap,
    /// The coordinates of the move, if any
    pub move_coords: Option<(usize, usize)>,
    /// bounding box of all played moves thus far
    pub min_row: usize,
    pub max_row: usize,
    pub min_col: usize,
    pub max_col: usize,

    pub score_white: i32,
    pub score_black: i32,

    pub player_to_play: PlayerType,
    pub score: i32,
}

impl<'a> EvaluatedMoveSet<'a> {
    // initalize an EvaluatedMoveSet from a BoardState, evaluating the full board
    pub fn from_board_state(
        board: &'a BoardState,
        scorer: &PatternScorer,
        player_to_play: PlayerType,
    ) -> Self {
        let map = MoveMap::with_capacity(0);

        let  (mut min_row, mut max_row, mut min_col, mut max_col): (usize, usize, usize, usize) = (
            board.height / 2,
            board.height / 2,
            board.width / 2,
            board.width / 2,
        );
        let mut is_empty = true;

        for row in 0..board.height {
            for col in 0..board.width {
                if board.get_tile(row, col) != TileType::Empty {
                    if is_empty {
                        min_row = row;
                        max_row = row;
                        min_col = col;
                        max_col = col;
                        is_empty = false;
                    } else {
                        min_row = min_row.min(row);
                        max_row = max_row.max(row);
                        min_col = min_col.min(col);
                    }
                    min_row = min_row.min(row);
                    max_row = max_row.max(row);
                    min_col = min_col.min(col);
                    max_col = max_col.max(col);
                }
            }
        }

        let mut this = Self {
            board,
            moves: map,
            move_coords: None,
            min_row,
            max_row,
            min_col,
            max_col,
            player_to_play,
            score_white: 0,
            score_black: 0,
            score: 0,
        };
        this.score_white = evaluate_full(&board, scorer, &this.moves, PlayerType::White);
        this.score_black = evaluate_full(&board, scorer, &this.moves, PlayerType::Black);
        this.score = if player_to_play == PlayerType::White {
            this.score_white
        } else {
            this.score_black
        };
        this
    }

    // Add a new move to a parent EvaluatedMoveSet. Evaluates boardd on the delta of the moves
    pub fn from_parent(
        parent: &'a EvaluatedMoveSet,
        scorer: &PatternScorer,
        board: &'a BoardState,
        row: usize,
        col: usize,
    ) -> Self {

        let idx = board.index(row, col);
        let mut moves = parent.moves.clone();
        let player_to_play = parent.player_to_play.next();
        let tile = TileType::from_player_type(player_to_play);
        moves.insert(idx, tile);

        let mut this = Self {
            board,
            moves,
            move_coords: Some((row, col)),
            min_row: parent.min_row.min(row),
            max_row: parent.max_row.max(row),
            min_col: parent.min_col.min(col),
            max_col: parent.max_col.max(col),
            player_to_play,
            score_white: 0,
            score_black: 0,
            score: 0,
        };

        let before = |r: usize, c: usize| if r == row && c == col { TileType::Empty } else { effective_tile_at(r, c, &board, &parent.moves) };
        let after = |r: usize, c: usize| effective_tile_at(r, c, &board, &this.moves);
        
        let delta_white = local_score(row, col, after, board, scorer, PlayerType::White) - local_score(row, col, before, board, scorer, PlayerType::White);
        this.score_white = parent.score_white + delta_white;

        let delta_black = local_score(row, col, after, board, scorer, PlayerType::Black) - local_score(row, col, before, board, scorer, PlayerType::Black);
        this.score_black = parent.score_black + delta_black;

        this.score = if player_to_play == PlayerType::White {
            this.score_white
        } else {
            this.score_black
        };
        
        this
    }

    
}

// gets the tile at a position given an initial board and set of moves made
fn effective_tile_at(row: usize, col: usize, board: &BoardState, moves: &MoveMap) -> TileType {
    let i = board.index(row, col);
    moves
        .get(&i)
        .copied()
        .unwrap_or_else(|| board.get_tile(row, col))
}

// evalautes the score of a board
fn evaluate_full(board: &BoardState, scorer: &PatternScorer, moves: &MoveMap, player_to_play: PlayerType) -> i32 {
    let mut total = 0i32;
    let convert_to_turn_tile_type = |tile: TileType| TurnTileType::from_tile_type(player_to_play, tile);

    for row in 0..board.height {
        let line: Vec<TurnTileType> = (0..board.width).map(|c| effective_tile_at(row, c, board, moves))
        .map(convert_to_turn_tile_type)
        .collect();
        total += scorer.score_line(&line);
    }

    for col in 0..board.width {
        let line: Vec<TurnTileType> = (0..board.height).map(|r| effective_tile_at(r, col, board, moves))
        .map(convert_to_turn_tile_type)
        .collect();
        total += scorer.score_line(&line);
    }

    // TODO hanlde non-square boards
    // Down-right (\): starts along top row then left column.
    for start_col in 0..board.width {
        let line: Vec<TurnTileType> = (start_col..board.width).map(|c| effective_tile_at(c - start_col, c, board, moves))
        .map(convert_to_turn_tile_type)
        .collect();
        total += scorer.score_line(&line);
    }
    for start_row in 1..board.height {
        let line: Vec<TurnTileType> = (start_row..board.height).map(|r| effective_tile_at(r, r - start_row, board, moves))
        .map(convert_to_turn_tile_type)
        .collect();
        total += scorer.score_line(&line);
    }

    // Down-left (/): starts along top row then right column.
    for start_col in 0..board.width {
        let line: Vec<TurnTileType> = (0..start_col).map(|c| effective_tile_at(start_col - c, c, board, moves))
        .map(convert_to_turn_tile_type)
        .collect();
        total += scorer.score_line(&line);
    }
    for start_row in 1..board.height {
        let line: Vec<TurnTileType> = (start_row..board.height).map(|r| effective_tile_at(r, board.width - 1 + start_row - r, board, moves))
        .map(convert_to_turn_tile_type)
        .collect();
        total += scorer.score_line(&line);
    }

    total
}

// gets the score associated with a single tile. Used for calculating deltas between moves.
fn local_score(
    row: usize,
    col: usize,
    get: impl Fn(usize, usize) -> TileType,
    board: &BoardState,
    scorer: &PatternScorer,
    player_to_play: PlayerType,
) -> i32 {
    let mut total = 0i32;
    let convert_to_turn_tile_type = |tile: TileType| TurnTileType::from_tile_type(player_to_play, tile);
    
    // Horizontal
    let c0 = col.saturating_sub(5);
    let c1 = (col + 5).min(board.width - 1);
    let row_line: Vec<TurnTileType> = (c0..=c1).map(|c| get(row, c)).map(convert_to_turn_tile_type).collect();
    total += scorer.score_line(&row_line);

    // Vertical
    let r0 = row.saturating_sub(5);
    let r1 = (row + 5).min(board.height - 1);
    let col_line: Vec<TurnTileType> = (r0..=r1).map(|r| get(r, col)).map(convert_to_turn_tile_type).collect();
    total += scorer.score_line(&col_line);

    // Diagonal \ -> Rn = Cn - col + row
    let r0 = row.saturating_sub(5)
      .max(col.saturating_sub(5).saturating_add(row).saturating_sub(col));
    let r1 = (row + 5)
      .min(board.height - 1)
      .min(board.width.saturating_sub(1).saturating_add(row).saturating_sub(col));
    let diagonal_line: Vec<TurnTileType> = (r0..=r1).map(|r| get(r, r + col - row)).map(convert_to_turn_tile_type).collect();
    total += scorer.score_line(&diagonal_line);

    // Diagonal / -> Rn = -Cn + col + row
    let r0 = row.saturating_sub(5)
      .max(row.saturating_add(col).saturating_sub(board.width - 1));
    let r1 = (row + 5)
      .min(board.height - 1)
      .min(row.saturating_add(col).saturating_sub(col.saturating_sub(5)));
    let diagonal_line: Vec<TurnTileType> = (r0..=r1).map(|r| get(r, row + col - r)).map(convert_to_turn_tile_type).collect();
    total += scorer.score_line(&diagonal_line);

    total
}

// uses a DFA to calculate a score given a pattern of inputs
#[derive(Debug, Clone)]
pub struct PatternScorer {
    dfa: TileDfa,
    weights: PatternWeights
}
impl PatternScorer {
    pub fn new(dfa: TileDfa, weights: PatternWeights) -> Self {
        Self { dfa, weights }
    }

    fn score_line(&self, line: &[TurnTileType]) -> i32 {
        let mut sum = 0i32;
        for m in self.dfa.find_matches(line) {
            sum = sum.saturating_add(self.weights[m.pattern_id]);
        }
        sum
    }
}

/// Pattern strings use `0` = empty, `1` = white, `2` = black (see `patterns.toml`).
fn parse_pattern(s: &str) -> Vec<TurnTileType> {
    s.chars()
        .map(|ch| match ch {
            '0' => TurnTileType::Empty,
            '1' => TurnTileType::One,
            '2' => TurnTileType::Two,
            _ => panic!("bad pattern char {ch:?} in {s:?}"),
        })
        .collect()
}

/// Builds the automaton and weight table from the engine pattern set.
/// 1 inidicates the currently to move player's piece, 2 indicates their opponent's piece.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tile::TileType::{Black, Empty, White};

    #[test]
    fn incremental_matches_full_one_stone() {
        let emptyBoard = BoardState::new(15, 15);
        let mut board = emptyBoard.clone();
        board.set_tile(7, 7, Black);
        let (dfa, weights) = default_automaton();
        let scorer = PatternScorer::new(dfa, weights);

        let full = EvaluatedMoveSet::from_board_state(&board, &scorer, PlayerType::Black);

        let parent = EvaluatedMoveSet::from_board_state(&emptyBoard, &scorer, PlayerType::White);
        let inc = EvaluatedMoveSet::from_parent(&parent, &scorer, &emptyBoard, 7, 7);

        assert!(full.score != 0);
        assert_eq!(full.score, inc.score);
    }

    // #[test]
    // fn seeded_from_board_matches_full_two_moves() {
    //     let (ac, w) = default_automaton();
    //     let mut board = BoardState::new(9, 9);
    //     board.set_tile(4, 4, Black);

    //     let moves = vec![
    //         (board.index(4, 5), White),
    //         (board.index(4, 6), Black),
    //     ];
    //     let mut map = MoveMap::new();
    //     for &(idx, t) in &moves {
    //         map.insert(idx, t);
    //     }
    //     let expected = evaluate_full(&board, &map, &ac, &w);
    //     let got = EvaluatedMoveSet::from_board_state(board, moves, &ac, &w).score;
    //     assert_eq!(got, expected);
    // }

    // #[test]
    // fn move_map_lookup_o1() {
    //     let (ac, w) = default_automaton();
    //     let board = BoardState::new(5, 5);
    //     let idx = board.index(2, 3);
    //     let ems = EvaluatedMoveSet::from_board_state(board, vec![(idx, Black)], &ac, &w);
    //     assert_eq!(ems.move_at(2, 3), Some(Black));
    //     assert_eq!(ems.move_at(0, 0), None);
    // }
}
