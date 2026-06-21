export type Player = "black" | "white";

export const HUMAN_PLAYER: Player = "black";
export const OPPONENT_PLAYER: Player = "white";

export function nextPlayer(player: Player): Player {
  return player === "black" ? "white" : "black";
}

export function playerLabel(player: Player): string {
  return player === "black" ? "Black" : "White";
}
