//! CLI entry: reads a board position, emits ranked next moves as JSON.

use std::time::Instant;

use anyhow::{Context, Result};
use clap::Parser;
use serde::Serialize;

use pente_engine::board::BoardState;
use pente_engine::evaluation::{default_automaton, PatternScorer};
use pente_engine::search::Search;
use pente_engine::tile::PlayerType;

/// Search depth for `find_best_move` (plies to explore).
const SEARCH_DEPTH: usize = 3;

#[derive(Parser, Debug)]
#[command(name = "pente-engine")]
#[command(about = "Evaluate a Pente board position and list candidate moves with scores.")]
struct Args {
    /// Board position: JSON `BoardState` or compact `WxH:` grid (`.` empty, `b` black, `w` white).
    position: String,
}

#[derive(Serialize)]
struct MoveScore {
    row: usize,
    col: usize,
    score: i32,
}

#[derive(Serialize)]
struct EngineOutput {
    /// Echo of the input position for callers that pipe JSON.
    position: String,
    moves: Vec<MoveScore>,
    /// Wall-clock time for parse + search, in milliseconds.
    duration_ms: u64,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let started = Instant::now();
    let moves = get_top_moves(&args.position)?;
    let duration_ms = started.elapsed().as_millis() as u64;

    let out = EngineOutput {
        position: args.position.clone(),
        moves,
        duration_ms,
    };

    let json = serde_json::to_string_pretty(&out).context("serialize output")?;
    println!("{json}");
    Ok(())
}

fn get_top_moves(position: &str) -> Result<Vec<MoveScore>> {
    let board = BoardState::from_position_str(position)
        .map_err(|e| anyhow::anyhow!("parse board position: {e}"))?;
    let (tile_dfa, pattern_weights) = default_automaton();
    let search = Search::new(PatternScorer::new(tile_dfa, pattern_weights));
    let ((row, col), score) = search.find_best_move(&board, PlayerType::Black, SEARCH_DEPTH);
    Ok(vec![MoveScore { row, col, score }])
}
