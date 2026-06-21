import {
  BOARD_SIZE,
  createEmptyBoard,
  parseBoard,
  type Board,
} from "@/lib/board";
import type { EncodedGame, MoveScore } from "@/lib/game";
import type { Player } from "@/lib/players";

const STUB_DELAY_MS = 250;

function distToCenter(row: number, col: number): number {
  const center = (BOARD_SIZE - 1) / 2;
  return Math.abs(row - center) + Math.abs(col - center);
}

function findEmptyMoves(board: Board): Array<{ row: number; col: number }> {
  const moves: Array<{ row: number; col: number }> = [];
  for (let row = 0; row < BOARD_SIZE; row++) {
    for (let col = 0; col < BOARD_SIZE; col++) {
      if (board[row]![col] === "empty") {
        moves.push({ row, col });
      }
    }
  }
  return moves;
}

/**
 * Stub opponent search — picks the empty intersection closest to center.
 * Replace with a call to the Rust engine / API when ready.
 */
export async function getOpponentMove(
  game: EncodedGame,
  player: Player,
): Promise<MoveScore> {
  await new Promise((resolve) => setTimeout(resolve, STUB_DELAY_MS));

  const board = parseBoard(game) ?? createEmptyBoard();
  const candidates = findEmptyMoves(board);

  if (candidates.length === 0) {
    throw new Error("No legal moves available");
  }

  candidates.sort(
    (a, b) => distToCenter(a.row, a.col) - distToCenter(b.row, b.col),
  );

  const best = candidates[0]!;

  return {
    row: best.row,
    col: best.col,
    score: player === "white" ? 1 : 0,
  };
}
