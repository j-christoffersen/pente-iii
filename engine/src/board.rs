//! Grid state for Pente-style evaluation.

use serde::{Deserialize, Serialize};

use crate::tile::TileType;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoardState {
    pub width: usize,
    pub height: usize,
    pub tiles: Vec<TileType>,
}

impl BoardState {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            tiles: vec![TileType::Empty; width * height],
        }
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
}
