//! Incremental and full-board pattern scoring using [`crate::aho_corasick::TileDfa`].

use std::collections::HashMap;

use crate::aho_corasick::{TileDfa, TurnTileType};
use crate::board::{find_captures, BoardState};
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

    /// Cumulative pairs captured so far, by capturing player.
    pub captures_white: u32,
    pub captures_black: u32,

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
            captures_white: board.captures_white,
            captures_black: board.captures_black,
            score_white: 0,
            score_black: 0,
            score: 0,
        };
        this.score_white = evaluate_full(&board, scorer, &this.moves, PlayerType::White)
            + net_capture_score(this.captures_white, this.captures_black);
        this.score_black = evaluate_full(&board, scorer, &this.moves, PlayerType::Black)
            + net_capture_score(this.captures_black, this.captures_white);
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

        let mut moves = parent.moves.clone();
        // The mover for this transition is whoever parent says is about to
        // play; the new state then hands the turn to their opponent.
        let mover = parent.player_to_play;
        let next_to_play = mover.next();
        let tile = TileType::from_player_type(mover);

        let (mut delta_white, mut delta_black) =
            apply_cell_change(&mut moves, board, scorer, row, col, tile);

        // A move can capture bracketed opponent pairs along any of the 8
        // directions; each captured cell is cleared and re-scored too, since
        // its removal can open or close lines that don't pass through (row, col).
        let captured = find_captures(board.width, board.height, row, col, mover, |r, c| {
            effective_tile_at(r, c, board, &moves)
        });
        for &(cr, cc) in &captured {
            let (dw, db) = apply_cell_change(&mut moves, board, scorer, cr, cc, TileType::Empty);
            delta_white += dw;
            delta_black += db;
        }

        let captured_pairs = (captured.len() / 2) as u32;
        let mut captures_white = parent.captures_white;
        let mut captures_black = parent.captures_black;
        match mover {
            PlayerType::White => captures_white += captured_pairs,
            PlayerType::Black => captures_black += captured_pairs,
        }

        let capture_delta_white = net_capture_score(captures_white, captures_black)
            - net_capture_score(parent.captures_white, parent.captures_black);
        let capture_delta_black = net_capture_score(captures_black, captures_white)
            - net_capture_score(parent.captures_black, parent.captures_white);

        let mut this = Self {
            board,
            moves,
            move_coords: Some((row, col)),
            min_row: parent.min_row.min(row),
            max_row: parent.max_row.max(row),
            min_col: parent.min_col.min(col),
            max_col: parent.max_col.max(col),
            captures_white,
            captures_black,
            score_white: parent.score_white + delta_white + capture_delta_white,
            score_black: parent.score_black + delta_black + capture_delta_black,
            player_to_play: next_to_play,
            score: 0,
        };

        this.score = if next_to_play == PlayerType::White {
            this.score_white
        } else {
            this.score_black
        };

        this
    }
}

/// Inserts `new_tile` at (row, col) in `moves` and returns the (white, black)
/// score delta caused by that single change, scoped to the lines through it.
fn apply_cell_change(
    moves: &mut MoveMap,
    board: &BoardState,
    scorer: &PatternScorer,
    row: usize,
    col: usize,
    new_tile: TileType,
) -> (i32, i32) {
    let before = |r: usize, c: usize| effective_tile_at(r, c, board, moves);
    let before_white = local_score(row, col, before, board, scorer, PlayerType::White);
    let before_black = local_score(row, col, before, board, scorer, PlayerType::Black);

    moves.insert(board.index(row, col), new_tile);

    let after = |r: usize, c: usize| effective_tile_at(r, c, board, moves);
    let after_white = local_score(row, col, after, board, scorer, PlayerType::White);
    let after_black = local_score(row, col, after, board, scorer, PlayerType::Black);

    (after_white - before_white, after_black - before_black)
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

/// Shared ceiling for either way of winning Pente outright (5 captured pairs
/// or 5 in a row). Far above any other pattern weight (the largest is 5^7 =
/// 78125 for a live four), so a winning position always dominates the score
/// regardless of what else is on the board.
const WIN_SCORE: i32 = 5_i32.pow(9);

/// Score contributed by one player's captured pairs, in isolation. Each pair
/// below the win threshold (5^4 = 625) sits between the weight of making your
/// own open three ("01110" = 5^3 = 125) and the weight of an opponent's open
/// three going unanswered ("02220" = 5^5 = 3125): a capture is a better move
/// than just extending your own three, but it should never be taken in place
/// of blocking a real threat from the opponent. The 5th pair (the standard
/// Pente win-by-capture threshold) hits `WIN_SCORE`, same as five-in-a-row.
const CAPTURE_PAIR_VALUE: i32 = 5_i32.pow(4);
const CAPTURE_WIN_PAIRS: u32 = 5;

fn capture_score(pairs: u32) -> i32 {
    if pairs >= CAPTURE_WIN_PAIRS {
        WIN_SCORE
    } else {
        CAPTURE_PAIR_VALUE * pairs as i32
    }
}

/// Capture score from one player's perspective: their own captures are good,
/// captures made against them are equally bad.
fn net_capture_score(own_pairs: u32, opponent_pairs: u32) -> i32 {
    capture_score(own_pairs) - capture_score(opponent_pairs)
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
        // Five (or more) in a row is an outright win, so it gets the same
        // ceiling as the capture win condition (see `WIN_SCORE`). A run of
        // 6+ in a row contains multiple overlapping "11111" matches, which
        // just multiplies an already-dominant score — still unmistakably a
        // win, not a problem worth guarding against.
        ("11111", WIN_SCORE),
        ("22222", -WIN_SCORE),
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
    let mut weights = Vec::with_capacity(specs.len() * 2);
    for (pat, w) in specs {
        ac.add_pattern(&parse_pattern(pat));
        weights.push(*w);

        // Also match the mirrored arrangement, since a line is always
        // scanned in one fixed direction but the same tactical shape can
        // occur built from either side. Skip palindromes (e.g. "0110") —
        // registering them twice would double-count every match.
        let reversed: String = pat.chars().rev().collect();
        if reversed != *pat {
            ac.add_pattern(&parse_pattern(&reversed));
            weights.push(*w);
        }
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
        let empty_booard = BoardState::new(15, 15);
        let mut board = empty_booard.clone();
        board.set_tile(7, 7, Black);
        let (dfa, weights) = default_automaton();
        let scorer = PatternScorer::new(dfa, weights);

        let full = EvaluatedMoveSet::from_board_state(&board, &scorer, PlayerType::Black);

        // parent.player_to_play is the mover, so Black is set to play here.
        let parent = EvaluatedMoveSet::from_board_state(&empty_booard, &scorer, PlayerType::Black);
        let inc = EvaluatedMoveSet::from_parent(&parent, &scorer, &empty_booard, 7, 7);

        // Compare score_white/score_black directly rather than `.score`: full
        // and inc select `.score` from different player_to_play values (full
        // uses the param passed in; inc uses the mover's opponent, since a
        // move was just made), so `.score` alone isn't an apples-to-apples
        // comparison here.
        assert!(full.score_black != 0);
        assert_eq!(full.score_white, inc.score_white);
        assert_eq!(full.score_black, inc.score_black);
    }

    #[test]
    fn incremental_matches_full_after_capture() {
        let (dfa, weights) = default_automaton();
        let scorer = PatternScorer::new(dfa, weights);

        // Black plays at (7, 9), bracketing White's pair at (7, 7)-(7, 8)
        // against Black's existing stone at (7, 6).
        let mut pre_board = BoardState::new(15, 15);
        pre_board.set_tile(7, 6, Black);
        pre_board.set_tile(7, 7, White);
        pre_board.set_tile(7, 8, White);

        let mut full_board = pre_board.clone();
        let captured = full_board.apply_move(7, 9, PlayerType::Black);
        assert_eq!(captured.len(), 2);
        assert_eq!(full_board.captures_black, 1);

        let full = EvaluatedMoveSet::from_board_state(&full_board, &scorer, PlayerType::Black);

        // parent.player_to_play is the mover, so Black is set to play here.
        let parent = EvaluatedMoveSet::from_board_state(&pre_board, &scorer, PlayerType::Black);
        let inc = EvaluatedMoveSet::from_parent(&parent, &scorer, &pre_board, 7, 9);

        assert_eq!(inc.captures_black, 1);
        assert_eq!(inc.captures_white, 0);
        assert_eq!(full.score_white, inc.score_white);
        assert_eq!(full.score_black, inc.score_black);
    }

    #[test]
    fn capture_value_beats_self_open_three_but_loses_to_opponent_open_three() {
        // "01110" (your own open three) is weighted at 5^3; "02220" (the
        // opponent's open three) is weighted at -5^5. A single capture should
        // beat the former (capturing is a stronger move than just extending
        // your own three) but lose to the latter (never grab a capture in
        // place of blocking the opponent's real threat).
        let self_open_three_weight = 5_i32.pow(3);
        let opponent_open_three_weight = 5_i32.pow(5);
        assert!(capture_score(1) > self_open_three_weight);
        assert!(capture_score(1) < opponent_open_three_weight);
    }

    #[test]
    fn fifth_capture_dominates_every_pattern_weight() {
        // The largest weight in `default_automaton` is 5^7 (an open four).
        // Reaching 5 captured pairs is a standard Pente win, so it must score
        // higher than any achievable pattern weight.
        let largest_pattern_weight = 5_i32.pow(7);
        assert!(capture_score(5) > largest_pattern_weight);
        assert!(net_capture_score(5, 0) > largest_pattern_weight);
    }

    #[test]
    fn five_in_a_row_hits_win_score() {
        let (dfa, weights) = default_automaton();
        let scorer = PatternScorer::new(dfa, weights);

        let mut board = BoardState::new(15, 15);
        for col in 3..8 {
            board.set_tile(7, col, Black);
        }

        let evaluated = EvaluatedMoveSet::from_board_state(&board, &scorer, PlayerType::Black);

        assert!(evaluated.score_black >= WIN_SCORE);
        assert!(evaluated.score_white <= -WIN_SCORE);
    }

    #[test]
    fn four_in_a_row_does_not_reach_win_score() {
        let (dfa, weights) = default_automaton();
        let scorer = PatternScorer::new(dfa, weights);

        let mut board = BoardState::new(15, 15);
        for col in 3..7 {
            board.set_tile(7, col, Black);
        }

        let evaluated = EvaluatedMoveSet::from_board_state(&board, &scorer, PlayerType::Black);

        assert!(evaluated.score_black < WIN_SCORE);
    }
}
