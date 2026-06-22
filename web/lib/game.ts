/** Encoded board position (compact `WxH:...` or JSON `BoardState`). */
export type EncodedGame = string;

export interface MoveScore {
  row: number;
  col: number;
  score: number;
}

export interface OpponentMoveResponse {
  game: EncodedGame;
  moves: MoveScore[];
  duration_ms: number;
}

/** Static (no-search) position evaluation from the engine. */
export interface EvaluateResponse {
  scoreWhite: number;
  scoreBlack: number;
}

export function decodeGameParam(raw: string | null | undefined): EncodedGame | null {
  if (!raw) {
    return null;
  }
  try {
    return decodeURIComponent(raw);
  } catch {
    return raw;
  }
}
