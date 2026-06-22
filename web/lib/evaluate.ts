import type { EncodedGame, EvaluateResponse } from "@/lib/game";

/** Client-side call to the engine's static (no-search) position evaluation. */
export async function evaluatePosition(
  game: EncodedGame,
): Promise<EvaluateResponse> {
  const res = await fetch("/api/evaluate", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ game }),
  });

  if (!res.ok) {
    const body = await res.json().catch(() => null);
    throw new Error(body?.error ?? `Evaluation failed (${res.status})`);
  }

  return (await res.json()) as EvaluateResponse;
}
