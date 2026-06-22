import { NextRequest, NextResponse } from "next/server";

import { evaluatePosition } from "@/lib/engine";
import { decodeGameParam, type EvaluateResponse } from "@/lib/game";

/**
 * POST /api/evaluate  { "game": "<encoded>" }
 * Static (no-search) score_white/score_black for the given position.
 */
export async function POST(request: NextRequest) {
  let body: unknown;
  try {
    body = await request.json();
  } catch {
    body = undefined;
  }

  const rawGame =
    body && typeof body === "object" && "game" in body
      ? (body as { game: unknown }).game
      : null;
  const game = decodeGameParam(typeof rawGame === "string" ? rawGame : null);

  if (!game) {
    return NextResponse.json(
      { error: "Missing game. Pass JSON body { game }." },
      { status: 400 },
    );
  }

  try {
    const evaluation = await evaluatePosition(game);
    const response: EvaluateResponse = evaluation;
    return NextResponse.json(response);
  } catch (err) {
    return NextResponse.json(
      { error: err instanceof Error ? err.message : "Evaluation failed" },
      { status: 422 },
    );
  }
}
