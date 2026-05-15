//! Aho–Corasick automaton over [`TurnTileType`]
//! Find pattern matches in a sequence of tiles in O(n) time.

use std::collections::VecDeque;
use serde::{Deserialize, Serialize};
use crate::tile::{PlayerType, TileType};

/// One occurrence of a pattern in the scanned sequence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Match {
    /// Inclusive start index in the input slice.
    pub start: usize,
    /// Exclusive end index (`start..end` spans the matched tiles).
    pub end: usize,
    /// Id returned by [`TileDfa::add_pattern`].
    pub pattern_id: usize,
}

// One is the currently to move player's piece, Two is their opponent's piece, Empty is an empty tile.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TurnTileType {
    One,
    Two,
    Empty,
}

impl TurnTileType {
    pub const ALPHABET_LEN: usize = 3;

    #[inline]
    pub fn alphabet_index(self) -> usize {
        match self {
            TurnTileType::One => 1,
            TurnTileType::Two => 2,
            TurnTileType::Empty => 0,
        }
    }

    #[inline]
    pub fn from_tile_type(player_to_play: PlayerType, tile_type: TileType) -> Self {
        match (player_to_play, tile_type) {
            (PlayerType::White, TileType::White) => TurnTileType::One,
            (PlayerType::White, TileType::Black) => TurnTileType::Two,
            (PlayerType::White, TileType::Empty) => TurnTileType::Empty,
            (PlayerType::Black, TileType::White) => TurnTileType::Two,
            (PlayerType::Black, TileType::Black) => TurnTileType::One,
            (PlayerType::Black, TileType::Empty) => TurnTileType::Empty,
        }
    }
}

#[derive(Clone, Debug)]
struct Node {
    /// Trie edges by `TileType::alphabet_index()`.
    next: [Option<usize>; TurnTileType::ALPHABET_LEN],
    fail: usize,
    /// Pattern ids whose bytes end exactly at this trie node.
    output: Vec<usize>,
}

impl Node {
    fn new() -> Self {
        Self {
            next: [None; TurnTileType::ALPHABET_LEN],
            fail: 0,
            output: Vec::new(),
        }
    }
}

/// Multi-pattern matcher: build with [`TileDfa::new`], add patterns, call [`TileDfa::build`], then [`TileDfa::find_matches`].
#[derive(Clone, Debug)]
pub struct TileDfa {
    nodes: Vec<Node>,
    pattern_lens: Vec<usize>,
    built: bool,
}

impl TileDfa {
    pub fn new() -> Self {
        Self {
            nodes: vec![Node::new()],
            pattern_lens: Vec::new(),
            built: false,
        }
    }

    /// Insert a pattern; returns its id. Call [`build`](Self::build) after all patterns are added.
    /// Empty patterns are rejected.
    pub fn add_pattern(&mut self, pattern: &[TurnTileType]) -> usize {
        assert!(!self.built, "cannot add_pattern after build");
        assert!(!pattern.is_empty(), "empty pattern is not supported");

        let id = self.pattern_lens.len();
        self.pattern_lens.push(pattern.len());

        let mut state = 0;
        for &tile in pattern {
            let c = tile.alphabet_index();
            if self.nodes[state].next[c].is_none() {
                self.nodes[state].next[c] = Some(self.nodes.len());
                self.nodes.push(Node::new());
            }
            state = self.nodes[state].next[c].expect("edge just inserted");
        }
        self.nodes[state].output.push(id);
        id
    }

    /// Build failure links (BFS). Required before matching.
    pub fn build(&mut self) {
        assert!(!self.built, "build called twice");
        self.built = true;

        let mut q = VecDeque::new();
        let root = 0;

        for c in 0..TurnTileType::ALPHABET_LEN {
            if let Some(u) = self.nodes[root].next[c] {
                self.nodes[u].fail = root;
                q.push_back(u);
            }
        }

        while let Some(r) = q.pop_front() {
            for c in 0..TurnTileType::ALPHABET_LEN {
                let Some(u) = self.nodes[r].next[c] else {
                    continue;
                };
                q.push_back(u);
                let mut f = self.nodes[r].fail;
                loop {
                    if let Some(t) = self.nodes[f].next[c] {
                        self.nodes[u].fail = t;
                        break;
                    }
                    if f == root {
                        self.nodes[u].fail = root;
                        break;
                    }
                    f = self.nodes[f].fail;
                }
            }
        }
    }

    /// Advance from `state` on `tile` (after `build`).
    pub fn transition(&self, mut state: usize, tile: TurnTileType) -> usize {
        debug_assert!(self.built);
        let c = tile.alphabet_index();
        loop {
            if let Some(next) = self.nodes[state].next[c] {
                return next;
            }
            if state == 0 {
                return 0;
            }
            state = self.nodes[state].fail;
        }
    }

    /// All pattern occurrences in `text` (possibly overlapping). End index is `i + 1` after consuming `text[i]`.
    pub fn find_matches(&self, text: &[TurnTileType]) -> Vec<Match> {
        debug_assert!(self.built);
        let mut out = Vec::new();
        let mut state = 0usize;

        for (i, &tile) in text.iter().enumerate() {
            state = self.transition(state, tile);
            let mut f = state;
            loop {
                for &pid in &self.nodes[f].output {
                    let len = self.pattern_lens[pid];
                    let end = i + 1;
                    let start = end.saturating_sub(len);
                    out.push(Match {
                        start,
                        end,
                        pattern_id: pid,
                    });
                }
                if f == 0 {
                    break;
                }
                f = self.nodes[f].fail;
            }
        }
        out
    }

    /// Returns true if some pattern occurs as a contiguous subsequence of `text`.
    pub fn is_match(&self, text: &[TurnTileType]) -> bool {
        !self.find_matches(text).is_empty()
    }
}

impl Default for TileDfa {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_pattern_finds_substring() {
        let mut ac = TileDfa::new();
        let p = [TurnTileType::One, TurnTileType::Two, TurnTileType::One];
        ac.add_pattern(&p);
        ac.build();

        let text = [TurnTileType::Empty, TurnTileType::One, TurnTileType::Two, TurnTileType::One, TurnTileType::Empty];
        let m = ac.find_matches(&text);
        assert_eq!(
            m,
            vec![Match {
                start: 1,
                end: 4,
                pattern_id: 0
            }]
        );
    }

    #[test]
    fn multiple_patterns_overlapping() {
        let mut ac = TileDfa::new();
        let id_a = ac.add_pattern(&[TurnTileType::One, TurnTileType::Two]);
        let id_b = ac.add_pattern(&[TurnTileType::Two, TurnTileType::One]);
        ac.build();

        let text = [TurnTileType::One, TurnTileType::Two, TurnTileType::One];
        let mut m = ac.find_matches(&text);
        m.sort_by_key(|x| (x.start, x.pattern_id));
        assert_eq!(
            m,
            vec![
                Match {
                    start: 0,
                    end: 2,
                    pattern_id: id_a
                },
                Match {
                    start: 1,
                    end: 3,
                    pattern_id: id_b
                },
            ]
        );
    }

    #[test]
    fn failure_link_finds_suffix_pattern() {
        let mut ac = TileDfa::new();
        ac.add_pattern(&[TurnTileType::Two, TurnTileType::Two]);
        ac.build();

        let text = [TurnTileType::One, TurnTileType::Two, TurnTileType::Two, TurnTileType::Two];
        let m = ac.find_matches(&text);
        assert_eq!(m.len(), 2);
        assert!(m.iter().any(|x| x.start == 1 && x.end == 3));
        assert!(m.iter().any(|x| x.start == 2 && x.end == 4));
    }
}
