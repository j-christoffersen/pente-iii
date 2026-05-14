use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TileType {
    Black,
    White,
    Empty,
}

impl TileType {
    pub const ALPHABET_LEN: usize = 3;

    #[inline]
    pub fn alphabet_index(self) -> usize {
        match self {
            TileType::Black => 0,
            TileType::White => 1,
            TileType::Empty => 2,
        }
    }
}
