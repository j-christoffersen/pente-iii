import { NextRequest, NextResponse } from "next/server";

import { findBestMove } from "@/lib/engine";
import { decodeGameParam, type OpponentMoveResponse } from "@/lib/game";
import { OPPONENT_PLAYER, type Player } from "@/lib/players";

const DEFAULT_SEARCH_DEPTH = 3;

function parseGame(request: NextRequest, body?: unknown): string | null {
  const fromQuery = decodeGameParam(request.nextUrl.searchParams.get("game"));
  if (fromQuery) {
    return fromQuery;
  }

  if (body && typeof body === "object" && "game" in body) {
    const game = (body as { game: unknown }).game;
    return decodeGameParam(typeof game === "string" ? game : null);
  }

  return null;
}

function parsePlayer(body: unknown): Player {
  if (
    body &&
    typeof body === "object" &&
    "player" in body &&
    ((body as { player: unknown }).player === "black" ||
      (body as { player: unknown }).player === "white")
  ) {
    return (body as { player: Player }).player;
  }
  return OPPONENT_PLAYER;
}

function parseDepth(body: unknown): number {
  if (body && typeof body === "object" && "depth" in body) {
    const depth = (body as { depth: unknown }).depth;
    if (typeof depth === "number" && Number.isInteger(depth) && depth >= 1) {
      return depth;
    }
  }
  return DEFAULT_SEARCH_DEPTH;
}

/**
 * GET /api/opponent-move?game=<encoded>
 * POST /api/opponent-move  { "game": "<encoded>", "player": "white", "depth": 1 }
 */
async function handleOpponentMove(request: NextRequest, body?: unknown) {
  const started = performance.now();
  const game = parseGame(request, body);

  if (!game) {
    return NextResponse.json(
      {
        error:
          "Missing game. Pass encoded position as ?game=... or JSON body { game }.",
      },
      { status: 400 },
    );
  }

  const player = parsePlayer(body);
  const depth = parseDepth(body);

  try {
    const move = await findBestMove(game, player, depth);
    const response: OpponentMoveResponse = {
      game,
      moves: [move],
      duration_ms: Math.round(performance.now() - started),
    };
    return NextResponse.json(response);
  } catch (err) {
    return NextResponse.json(
      { error: err instanceof Error ? err.message : "Search failed" },
      { status: 422 },
    );
  }
}

export async function POST(request: NextRequest) {
  let body: unknown;
  try {
    body = await request.json();
  } catch {
    body = undefined;
  }
  return handleOpponentMove(request, body);
}
