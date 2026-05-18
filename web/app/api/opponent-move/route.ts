import { NextRequest, NextResponse } from "next/server";

import {
  decodeGameParam,
  type EncodedGame,
  type OpponentMoveResponse,
} from "@/lib/game";

function parseGame(request: NextRequest, body?: unknown): EncodedGame | null {
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

/**
 * GET /api/opponent-move?game=<encoded>
 * POST /api/opponent-move  { "game": "<encoded>" }
 *
 * Stub: validates encoded game is present; Rust engine integration TBD.
 */
export async function GET(request: NextRequest) {
  const game = parseGame(request);
  if (!game) {
    return NextResponse.json(
      {
        error:
          "Missing game. Pass encoded position as ?game=... or JSON body { game }.",
      },
      { status: 400 },
    );
  }

  const response: OpponentMoveResponse = {
    game,
    moves: [],
    duration_ms: 0,
  };

  return NextResponse.json(response);
}

export async function POST(request: NextRequest) {
  let body: unknown;
  try {
    body = await request.json();
  } catch {
    body = undefined;
  }

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

  const response: OpponentMoveResponse = {
    game,
    moves: [],
    duration_ms: 0,
  };

  return NextResponse.json(response);
}
