import { execFile } from "node:child_process";
import { existsSync } from "node:fs";
import path from "node:path";
import { promisify } from "node:util";

import type { EncodedGame, MoveScore } from "@/lib/game";
import type { Player } from "@/lib/players";

/**
 * Server-only bridge to the compiled Rust engine binary. Do not import this
 * from client components — it shells out via `node:child_process`.
 */

const execFileAsync = promisify(execFile);

const ENGINE_DIR = path.resolve(process.cwd(), "..", "engine");
const RELEASE_BINARY = path.join(ENGINE_DIR, "target", "release", "pente-engine");
const DEBUG_BINARY = path.join(ENGINE_DIR, "target", "debug", "pente-engine");

function resolveEngineBinary(): string {
  if (existsSync(RELEASE_BINARY)) {
    return RELEASE_BINARY;
  }
  if (existsSync(DEBUG_BINARY)) {
    return DEBUG_BINARY;
  }
  throw new Error(
    `Engine binary not found. Build it with: cargo build --release --manifest-path ${path.join(ENGINE_DIR, "Cargo.toml")}`,
  );
}

async function runEngine(args: string[]): Promise<string> {
  const { stdout } = await execFileAsync(resolveEngineBinary(), args, {
    maxBuffer: 10 * 1024 * 1024,
  });
  return stdout;
}

interface MoveOutput {
  position: string;
  moves: MoveScore[];
  duration_ms: number;
}

export async function findBestMove(
  game: EncodedGame,
  player: Player,
  depth: number,
): Promise<MoveScore> {
  const stdout = await runEngine([
    game,
    "--player",
    player,
    "--depth",
    String(depth),
  ]);
  const parsed = JSON.parse(stdout) as MoveOutput;
  const move = parsed.moves[0];
  if (!move) {
    throw new Error("Engine returned no candidate moves");
  }
  return move;
}

interface EvaluateOutput {
  position: string;
  score_white: number;
  score_black: number;
  duration_ms: number;
}

export interface EngineEvaluation {
  scoreWhite: number;
  scoreBlack: number;
}

export async function evaluatePosition(game: EncodedGame): Promise<EngineEvaluation> {
  const stdout = await runEngine([game, "--evaluate"]);
  const parsed = JSON.parse(stdout) as EvaluateOutput;
  return { scoreWhite: parsed.score_white, scoreBlack: parsed.score_black };
}
