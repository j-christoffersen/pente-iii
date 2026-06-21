export const BOARD_SIZE = 19;

import type { Player } from "@/lib/players";

export type Cell = "empty" | "black" | "white";

export type Board = Cell[][];

/** Standard 19×19 star points (0-indexed). */
export const STAR_POINTS: ReadonlyArray<readonly [number, number]> = [
  [3, 3],
  [3, 9],
  [3, 15],
  [9, 3],
  [9, 9],
  [9, 15],
  [15, 3],
  [15, 9],
  [15, 15],
];

export function createEmptyBoard(): Board {
  return Array.from({ length: BOARD_SIZE }, () =>
    Array.from({ length: BOARD_SIZE }, () => "empty" as Cell),
  );
}

export function encodeBoard(board: Board): string {
  const cells = board
    .flat()
    .map((c) => (c === "black" ? "b" : c === "white" ? "w" : "."))
    .join("");
  return `${BOARD_SIZE}x${BOARD_SIZE}:${cells}`;
}

export function parseBoard(encoded: string): Board | null {
  const trimmed = trimmedEncoded(encoded);
  if (!trimmed) {
    return null;
  }

  if (trimmed.startsWith("{")) {
    return parseJsonBoard(trimmed);
  }

  return parseCompactBoard(trimmed);
}

function trimmedEncoded(encoded: string): string {
  return encoded.trim();
}

function parseCompactBoard(s: string): Board | null {
  const splitAt = s.includes(":") ? s.indexOf(":") : s.indexOf("\n");
  if (splitAt === -1) {
    return null;
  }

  const header = s.slice(0, splitAt).trim();
  const body = s
    .slice(splitAt + 1)
    .replace(/\s/g, "")
    .toLowerCase();

  const match = header.match(/^(\d+)x(\d+)$/);
  if (!match) {
    return null;
  }

  const width = Number(match[1]);
  const height = Number(match[2]);
  if (width !== BOARD_SIZE || height !== BOARD_SIZE) {
    return null;
  }

  if (body.length !== BOARD_SIZE * BOARD_SIZE) {
    return null;
  }

  const board = createEmptyBoard();
  for (let row = 0; row < BOARD_SIZE; row++) {
    for (let col = 0; col < BOARD_SIZE; col++) {
      const ch = body[row * BOARD_SIZE + col];
      if (ch === "b") {
        board[row]![col] = "black";
      } else if (ch === "w") {
        board[row]![col] = "white";
      } else if (ch !== ".") {
        return null;
      }
    }
  }

  return board;
}

function parseJsonBoard(s: string): Board | null {
  try {
    const data = JSON.parse(s) as {
      width?: number;
      height?: number;
      tiles?: string[];
    };
    if (data.width !== BOARD_SIZE || data.height !== BOARD_SIZE) {
      return null;
    }
    if (!Array.isArray(data.tiles) || data.tiles.length !== BOARD_SIZE * BOARD_SIZE) {
      return null;
    }

    const board = createEmptyBoard();
    for (let row = 0; row < BOARD_SIZE; row++) {
      for (let col = 0; col < BOARD_SIZE; col++) {
        const tile = data.tiles[row * BOARD_SIZE + col];
        if (tile === "black") {
          board[row]![col] = "black";
        } else if (tile === "white") {
          board[row]![col] = "white";
        } else if (tile !== "empty") {
          return null;
        }
      }
    }
    return board;
  } catch {
    return null;
  }
}

export function playerToCell(player: Player): Exclude<Cell, "empty"> {
  return player;
}

function opponentCell(player: Player): Exclude<Cell, "empty"> {
  return player === "black" ? "white" : "black";
}

function isInBounds(row: number, col: number): boolean {
  return row >= 0 && row < BOARD_SIZE && col >= 0 && col < BOARD_SIZE;
}

const DIRECTIONS: ReadonlyArray<readonly [number, number]> = [
  [-1, -1],
  [-1, 0],
  [-1, 1],
  [0, -1],
  [0, 1],
  [1, -1],
  [1, 0],
  [1, 1],
];

/**
 * Pente capture rule: placing a stone that brackets exactly two opponent
 * stones (own–opp–opp–own along a line) captures that pair. Checks all 8
 * directions from the just-placed stone.
 */
export function findCaptures(
  board: Board,
  row: number,
  col: number,
  player: Player,
): Array<[number, number]> {
  const opponent = opponentCell(player);
  const own = playerToCell(player);
  const captured: Array<[number, number]> = [];

  for (const [dr, dc] of DIRECTIONS) {
    const r1 = row + dr;
    const c1 = col + dc;
    const r2 = row + 2 * dr;
    const c2 = col + 2 * dc;
    const r3 = row + 3 * dr;
    const c3 = col + 3 * dc;

    if (
      isInBounds(r3, c3) &&
      board[r1]![c1] === opponent &&
      board[r2]![c2] === opponent &&
      board[r3]![c3] === own
    ) {
      captured.push([r1, c1], [r2, c2]);
    }
  }

  return captured;
}

export interface MoveResult {
  board: Board;
  captured: Array<[number, number]>;
}

export function applyMove(
  board: Board,
  row: number,
  col: number,
  player: Player,
): MoveResult {
  if (board[row]?.[col] !== "empty") {
    return { board, captured: [] };
  }

  const placed = board.map((r, ri) =>
    ri === row ? r.map((c, ci) => (ci === col ? playerToCell(player) : c)) : r.slice(),
  );

  const captured = findCaptures(placed, row, col, player);
  if (captured.length === 0) {
    return { board: placed, captured };
  }

  const capturedSet = new Set(captured.map(([r, c]) => `${r},${c}`));
  const board2 = placed.map((r, ri) =>
    r.map((c, ci) => (capturedSet.has(`${ri},${ci}`) ? "empty" : c)),
  );

  return { board: board2, captured };
}

export function isStarPoint(row: number, col: number): boolean {
  return STAR_POINTS.some(([r, c]) => r === row && c === col);
}
