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
}

#[cfg(test)]
mod tests {
    use super::*;

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
