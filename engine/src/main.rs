//! CLI entry: reads a board position, emits ranked next moves as JSON.

use anyhow::{Context, Result};
use clap::Parser;
use serde::Serialize;

#[derive(Parser, Debug)]
#[command(name = "pente-engine")]
#[command(about = "Evaluate a Pente board position and list candidate moves with scores.")]
struct Args {
    /// Board position encoding (format is project-defined; pass opaque state for the evaluator).
    position: String,
}

#[derive(Serialize)]
struct MoveScore {
    row: u8,
    col: u8,
    score: f64,
}

#[derive(Serialize)]
struct EngineOutput {
    /// Echo of the input position for callers that pipe JSON.
    position: String,
    moves: Vec<MoveScore>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let moves = get_top_moves(&args.position);

    let out = EngineOutput {
        position: args.position.clone(),
        moves,
    };

    let json = serde_json::to_string_pretty(&out).context("serialize output")?;
    println!("{json}");
    Ok(())
}

/// Placeholder move list until the evaluator is wired up.
fn get_top_moves(position: &str) -> Vec<MoveScore> {
    let _ = position;
    let search = Search::new(PatternScorer::new(default_automaton()));
    vec![]
}
