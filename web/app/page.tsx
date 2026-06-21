"use client";

import { useCallback, useMemo, useState } from "react";

import { BoardGrid } from "@/components/BoardGrid";
import { applyMove, createEmptyBoard, encodeBoard, type Board } from "@/lib/board";
import type { MoveScore } from "@/lib/game";
import { getOpponentMove } from "@/lib/opponent";
import {
  HUMAN_PLAYER,
  OPPONENT_PLAYER,
  playerLabel,
  type Player,
} from "@/lib/players";

export default function Home() {
  const [board, setBoard] = useState<Board>(() => createEmptyBoard());
  const [turn, setTurn] = useState<Player>(HUMAN_PLAYER);
  const [lastOpponentMove, setLastOpponentMove] = useState<MoveScore | null>(
    null,
  );
  const [captures, setCaptures] = useState<Record<Player, number>>({
    black: 0,
    white: 0,
  });
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const game = useMemo(() => encodeBoard(board), [board]);
  const canPlay = turn === HUMAN_PLAYER && !loading;

  const highlight = useMemo(() => {
    if (!lastOpponentMove) {
      return null;
    }
    return { row: lastOpponentMove.row, col: lastOpponentMove.col };
  }, [lastOpponentMove]);

  const runOpponentTurn = useCallback(async (boardAfterHuman: Board) => {
    setLoading(true);
    setError(null);

    try {
      const move = await getOpponentMove(
        encodeBoard(boardAfterHuman),
        OPPONENT_PLAYER,
      );
      setLastOpponentMove(move);
      const result = applyMove(
        boardAfterHuman,
        move.row,
        move.col,
        OPPONENT_PLAYER,
      );
      setBoard(result.board);
      if (result.captured.length > 0) {
        setCaptures((prev) => ({
          ...prev,
          [OPPONENT_PLAYER]: prev[OPPONENT_PLAYER] + result.captured.length,
        }));
      }
      setTurn(HUMAN_PLAYER);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Opponent move failed");
      setTurn(OPPONENT_PLAYER);
    } finally {
      setLoading(false);
    }
  }, []);

  const handlePlay = useCallback(
    async (row: number, col: number) => {
      if (!canPlay || board[row]?.[col] !== "empty") {
        return;
      }

      const result = applyMove(board, row, col, HUMAN_PLAYER);
      setBoard(result.board);
      if (result.captured.length > 0) {
        setCaptures((prev) => ({
          ...prev,
          [HUMAN_PLAYER]: prev[HUMAN_PLAYER] + result.captured.length,
        }));
      }
      setLastOpponentMove(null);
      setTurn(OPPONENT_PLAYER);
      await runOpponentTurn(result.board);
    },
    [board, canPlay, runOpponentTurn],
  );

  return (
    <main>
      <header className="header">
        <h1>Pente</h1>
        <p>
          You play Black. After each move, the opponent (White) responds via a
          stubbed search — wire in the Rust engine later.
        </p>
      </header>

      <div className="layout">
        <BoardGrid
          board={board}
          turn={turn}
          canPlay={canPlay}
          onPlay={handlePlay}
          highlight={highlight}
        />

        <aside className="sidebar">
          <p className="turn-status">
            <span
              className={`turn-dot turn-dot--${turn}`}
              aria-hidden
            />
            {loading
              ? `${playerLabel(OPPONENT_PLAYER)} is thinking…`
              : `${playerLabel(turn)} to move`}
          </p>

          <ul className="captures">
            <li>
              {playerLabel(HUMAN_PLAYER)} captures: {captures[HUMAN_PLAYER]}
            </li>
            <li>
              {playerLabel(OPPONENT_PLAYER)} captures: {captures[OPPONENT_PLAYER]}
            </li>
          </ul>

          <details className="encoded">
            <summary>Encoded position</summary>
            <code>{game}</code>
          </details>

          {lastOpponentMove && (
            <p className="last-move">
              Opponent played ({lastOpponentMove.row + 1},{" "}
              {lastOpponentMove.col + 1})
              {lastOpponentMove.score !== 0 && (
                <> · score {lastOpponentMove.score}</>
              )}
            </p>
          )}

          {error && <p className="error">{error}</p>}
        </aside>
      </div>
    </main>
  );
}
