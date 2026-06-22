//! CLI entry: reads a board position, emits ranked next moves (or a static
//! evaluation) as JSON.

use std::time::Instant;

use anyhow::{Context, Result};
use clap::Parser;
use serde::Serialize;

use pente_engine::board::BoardState;
use pente_engine::evaluation::{default_automaton, EvaluatedMoveSet, PatternScorer};
use pente_engine::search::Search;
use pente_engine::tile::PlayerType;

#[derive(Parser, Debug)]
#[command(name = "pente-engine")]
#[command(about = "Evaluate a Pente board position and list candidate moves with scores.")]
struct Args {
    /// Board position: JSON `BoardState` or compact `WxH:` grid (`.` empty, `b` black, `w` white).
    position: String,

    /// Side to find a move for / evaluate from: "black" or "white".
    #[arg(long, default_value = "black")]
    player: String,

    /// Search depth in plies. Ignored when `--evaluate` is set.
    #[arg(long, default_value_t = 3)]
    depth: usize,

    /// Statically score the position (score_white/score_black) instead of
    /// searching for a move.
    #[arg(long)]
    evaluate: bool,
}

#[derive(Serialize)]
struct MoveScore {
    row: usize,
    col: usize,
    score: i32,
}

#[derive(Serialize)]
struct MoveOutput {
    /// Echo of the input position for callers that pipe JSON.
    position: String,
    moves: Vec<MoveScore>,
    /// Wall-clock time for parse + search, in milliseconds.
    duration_ms: u64,
}

#[derive(Serialize)]
struct EvaluateOutput {
    position: String,
    score_white: i32,
    score_black: i32,
    duration_ms: u64,
}

fn parse_player(s: &str) -> Result<PlayerType> {
    match s.to_ascii_lowercase().as_str() {
        "black" => Ok(PlayerType::Black),
        "white" => Ok(PlayerType::White),
        other => Err(anyhow::anyhow!(
            "invalid --player {other:?}; expected \"black\" or \"white\""
        )),
    }
}

fn main() -> Result<()> {
    let args = Args::parse();
    let player = parse_player(&args.player)?;
    let board = BoardState::from_position_str(&args.position)
        .map_err(|e| anyhow::anyhow!("parse board position: {e}"))?;

    let started = Instant::now();
    let (dfa, weights) = default_automaton();
    let scorer = PatternScorer::new(dfa, weights);

    if args.evaluate {
        let evaluated = EvaluatedMoveSet::from_board_state(&board, &scorer, player);
        let duration_ms = started.elapsed().as_millis() as u64;
        let out = EvaluateOutput {
            position: args.position,
            score_white: evaluated.score_white,
            score_black: evaluated.score_black,
            duration_ms,
        };
        println!("{}", serde_json::to_string_pretty(&out).context("serialize output")?);
        return Ok(());
    }

    let search = Search::new(scorer);
    let ((row, col), score) = search.find_best_move(&board, player, args.depth);
    let duration_ms = started.elapsed().as_millis() as u64;

    let out = MoveOutput {
        position: args.position,
        moves: vec![MoveScore { row, col, score }],
        duration_ms,
    };
    println!("{}", serde_json::to_string_pretty(&out).context("serialize output")?);
    Ok(())
}
