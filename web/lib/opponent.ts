import type { EncodedGame, MoveScore, OpponentMoveResponse } from "@/lib/game";
import type { Player } from "@/lib/players";

/** Client-side call to the engine via the /api/opponent-move route. */
export async function getOpponentMove(
  game: EncodedGame,
  player: Player,
  depth: number,
): Promise<MoveScore> {
  const res = await fetch("/api/opponent-move", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ game, player, depth }),
  });

  if (!res.ok) {
    const body = await res.json().catch(() => null);
    throw new Error(body?.error ?? `Opponent move failed (${res.status})`);
  }

  const data = (await res.json()) as OpponentMoveResponse;
  const move = data.moves[0];
  if (!move) {
    throw new Error("Engine returned no candidate moves");
  }
  return move;
}
