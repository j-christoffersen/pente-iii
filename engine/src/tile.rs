use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TileType {
    Black,
    White,
    Empty,
}

impl TileType {
    pub fn from_player_type(player_type: PlayerType) -> Self {
        match player_type {
            PlayerType::Black => TileType::Black,
            PlayerType::White => TileType::White,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlayerType {
    Black,
    White,
}

impl PlayerType {
    pub fn next(self) -> Self {
        match self {
            PlayerType::Black => PlayerType::White,
            PlayerType::White => PlayerType::Black,
        }
    }
}
