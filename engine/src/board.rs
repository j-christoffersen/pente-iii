//! Grid state for Pente-style evaluation.

use serde::{Deserialize, Serialize};

use crate::tile::{PlayerType, TileType};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoardState {
    pub width: usize,
    pub height: usize,
    pub tiles: Vec<TileType>,
    /// Pairs of opponent stones captured so far, by capturing player.
    #[serde(default)]
    pub captures_white: u32,
    #[serde(default)]
    pub captures_black: u32,
}

/// The 8 line directions (4 axes) checked for captures from a placed stone.
const DIRECTIONS: [(isize, isize); 8] = [
    (-1, -1),
    (-1, 0),
    (-1, 1),
    (0, -1),
    (0, 1),
    (1, -1),
    (1, 0),
    (1, 1),
];

/// Pente capture rule: a stone placed at (row, col) by `player` captures any
/// opponent pair immediately bracketed (own-opp-opp-own) along a line. `get`
/// supplies the effective tile at a coordinate *after* the new stone is placed,
/// so this works against both a materialized board and a search overlay.
pub fn find_captures(
    width: usize,
    height: usize,
    row: usize,
    col: usize,
    player: PlayerType,
    get: impl Fn(usize, usize) -> TileType,
) -> Vec<(usize, usize)> {
    let opponent = TileType::from_player_type(player.next());
    let own = TileType::from_player_type(player);
    let mut captured = Vec::new();

    for &(dr, dc) in DIRECTIONS.iter() {
        let r3 = row as isize + 3 * dr;
        let c3 = col as isize + 3 * dc;
        if r3 < 0 || r3 >= height as isize || c3 < 0 || c3 >= width as isize {
            continue;
        }

        let r1 = (row as isize + dr) as usize;
        let c1 = (col as isize + dc) as usize;
        let r2 = (row as isize + 2 * dr) as usize;
        let c2 = (col as isize + 2 * dc) as usize;
        let (r3, c3) = (r3 as usize, c3 as usize);

        if get(r1, c1) == opponent && get(r2, c2) == opponent && get(r3, c3) == own {
            captured.push((r1, c1));
            captured.push((r2, c2));
        }
    }

    captured
}

impl BoardState {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            tiles: vec![TileType::Empty; width * height],
            captures_white: 0,
            captures_black: 0,
        }
    }

    /// Captures `player` would make by placing a stone at (row, col), without
    /// mutating the board.
    pub fn find_captures(&self, row: usize, col: usize, player: PlayerType) -> Vec<(usize, usize)> {
        find_captures(self.width, self.height, row, col, player, |r, c| {
            self.get_tile(r, c)
        })
    }

    /// Places `player`'s stone at (row, col), removing any captured opponent
    /// pairs and updating that player's capture count. Returns the captured
    /// coordinates (empty if none).
    pub fn apply_move(&mut self, row: usize, col: usize, player: PlayerType) -> Vec<(usize, usize)> {
        self.set_tile(row, col, TileType::from_player_type(player));

        let captured = self.find_captures(row, col, player);
        for &(r, c) in &captured {
            self.set_tile(r, c, TileType::Empty);
        }

        let pairs = (captured.len() / 2) as u32;
        match player {
            PlayerType::White => self.captures_white += pairs,
            PlayerType::Black => self.captures_black += pairs,
        }

        captured
    }

    #[inline]
    pub fn index(&self, row: usize, col: usize) -> usize {
        row * self.width + col
    }

    #[inline]
    pub fn get_tile(&self, row: usize, col: usize) -> TileType {
        self.tiles[self.index(row, col)]
    }

    pub fn set_tile(&mut self, row: usize, col: usize, tile: TileType) {
        let idx = self.index(row, col);
        self.tiles[idx] = tile;
    }

    /// Parse a board from JSON or a compact `WxH` grid (`.`, `b`, `w`).
    pub fn from_position_str(s: &str) -> Result<Self, String> {
        let s = s.trim();
        if s.starts_with('{') {
            return serde_json::from_str(s).map_err(|e| e.to_string());
        }
        parse_compact_grid(s)
    }
}

fn parse_dims(header: &str) -> Result<(usize, usize), String> {
    let (w, h) = header
        .trim()
        .split_once('x')
        .ok_or("expected WxH dimensions (e.g. 15x15)")?;
    let width = w.parse().map_err(|_| format!("invalid width: {w}"))?;
    let height = h.parse().map_err(|_| format!("invalid height: {h}"))?;
    Ok((width, height))
}

fn tile_from_char(c: char) -> Result<TileType, String> {
    match c.to_ascii_lowercase() {
        '.' | '-' => Ok(TileType::Empty),
        'b' => Ok(TileType::Black),
        'w' => Ok(TileType::White),
        _ => Err(format!("invalid cell character: {c:?}")),
    }
}

fn parse_compact_grid(s: &str) -> Result<BoardState, String> {
    let (header, rest) = s
        .split_once(|c| c == ':' || c == '\n')
        .ok_or("expected WxH:grid or WxH newline grid")?;
    let (width, height) = parse_dims(header)?;
    let cells: Vec<char> = rest.chars().filter(|c| !c.is_whitespace()).collect();
    let expected = width
        .checked_mul(height)
        .ok_or("board dimensions overflow")?;
    if cells.len() != expected {
        return Err(format!(
            "expected {expected} cells, got {}",
            cells.len()
        ));
    }
    let tiles = cells
        .into_iter()
        .map(tile_from_char)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(BoardState {
        width,
        height,
        tiles,
        captures_white: 0,
        captures_black: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_position_str_parses_compact_grid() {
        let board = BoardState::from_position_str(
            "3x3:\
             ...\
             .b.\
             ...",
        )
        .expect("parse");
        assert_eq!(board.width, 3);
        assert_eq!(board.get_tile(1, 1), TileType::Black);
    }

    #[test]
    fn board_state_json_roundtrip() {
        let mut b = BoardState::new(3, 2);
        b.set_tile(0, 1, TileType::Black);
        b.set_tile(1, 0, TileType::White);

        let json = serde_json::to_string(&b).expect("serialize");
        let back: BoardState = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(b, back);
    }

    #[test]
    fn json_without_captures_defaults_to_zero() {
        let board: BoardState =
            serde_json::from_str(r#"{"width":2,"height":1,"tiles":["empty","empty"]}"#)
                .expect("deserialize");
        assert_eq!(board.captures_white, 0);
        assert_eq!(board.captures_black, 0);
    }

    #[test]
    fn apply_move_captures_bracketed_pair() {
        let mut board = BoardState::new(9, 9);
        board.set_tile(5, 5, TileType::White);
        board.set_tile(5, 6, TileType::White);
        board.set_tile(5, 4, TileType::Black);

        let captured = board.apply_move(5, 7, PlayerType::Black);

        assert_eq!(captured.len(), 2);
        assert!(captured.contains(&(5, 5)));
        assert!(captured.contains(&(5, 6)));
        assert_eq!(board.get_tile(5, 5), TileType::Empty);
        assert_eq!(board.get_tile(5, 6), TileType::Empty);
        assert_eq!(board.get_tile(5, 7), TileType::Black);
        assert_eq!(board.captures_black, 1);
        assert_eq!(board.captures_white, 0);
    }

    #[test]
    fn apply_move_does_not_capture_three_in_a_row() {
        let mut board = BoardState::new(9, 9);
        board.set_tile(5, 5, TileType::White);
        board.set_tile(5, 6, TileType::White);
        board.set_tile(5, 7, TileType::White);
        board.set_tile(5, 4, TileType::Black);

        let captured = board.apply_move(5, 8, PlayerType::Black);

        assert!(captured.is_empty());
        assert_eq!(board.get_tile(5, 5), TileType::White);
        assert_eq!(board.captures_black, 0);
    }

    #[test]
    fn apply_move_can_capture_in_multiple_directions_at_once() {
        // Black plays at (5,5); it brackets a white pair to the east and another
        // white pair to the south in the same move.
        let mut board = BoardState::new(9, 9);
        board.set_tile(5, 6, TileType::White);
        board.set_tile(5, 7, TileType::White);
        board.set_tile(5, 8, TileType::Black);

        board.set_tile(6, 5, TileType::White);
        board.set_tile(7, 5, TileType::White);
        board.set_tile(8, 5, TileType::Black);

        let captured = board.apply_move(5, 5, PlayerType::Black);

        assert_eq!(captured.len(), 4);
        assert_eq!(board.captures_black, 2);
    }
}
