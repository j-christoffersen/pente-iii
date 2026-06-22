"use client";

import { useCallback, useMemo, useState } from "react";

import { BoardGrid } from "@/components/BoardGrid";
import { applyMove, createEmptyBoard, encodeBoard, type Board } from "@/lib/board";
import { evaluatePosition } from "@/lib/evaluate";
import type { EvaluateResponse, MoveScore } from "@/lib/game";
import { getOpponentMove } from "@/lib/opponent";
import {
  HUMAN_PLAYER,
  OPPONENT_PLAYER,
  playerLabel,
  type Player,
} from "@/lib/players";

/**
 * Shallow on purpose: lets the debug panel show the score of this exact
 * opponent move rather than a multi-ply lookahead result.
 */
const DEBUG_SEARCH_DEPTH = 1;

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
  const [humanMoveEval, setHumanMoveEval] = useState<EvaluateResponse | null>(
    null,
  );
  const [opponentMoveEval, setOpponentMoveEval] =
    useState<EvaluateResponse | null>(null);
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
        DEBUG_SEARCH_DEPTH,
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

      try {
        setOpponentMoveEval(await evaluatePosition(encodeBoard(result.board)));
      } catch {
        setOpponentMoveEval(null);
      }
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
      setOpponentMoveEval(null);
      setTurn(OPPONENT_PLAYER);

      try {
        setHumanMoveEval(await evaluatePosition(encodeBoard(result.board)));
      } catch {
        setHumanMoveEval(null);
      }

      await runOpponentTurn(result.board);
    },
    [board, canPlay, runOpponentTurn],
  );

  return (
    <main>
      <header className="header">
        <h1>Pente</h1>
        <p>
          You play Black. After each move, the opponent (White) responds via
          the Rust engine, searched at depth {DEBUG_SEARCH_DEPTH} for
          debugging.
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

          <section className="debug-eval">
            <h2>Debug: position score</h2>
            <p className="debug-eval__row">
              <span>After your move:</span>{" "}
              {humanMoveEval
                ? `white ${humanMoveEval.scoreWhite} · black ${humanMoveEval.scoreBlack}`
                : "—"}
            </p>
            <p className="debug-eval__row">
              <span>After opponent move:</span>{" "}
              {opponentMoveEval
                ? `white ${opponentMoveEval.scoreWhite} · black ${opponentMoveEval.scoreBlack}`
                : "—"}
            </p>
          </section>

          <details className="encoded">
            <summary>Encoded position</summary>
            <code>{game}</code>
          </details>

          {lastOpponentMove && (
            <p className="last-move">
              Opponent played ({lastOpponentMove.row + 1},{" "}
              {lastOpponentMove.col + 1})
              {lastOpponentMove.score !== 0 && (
                <> · search score {lastOpponentMove.score}</>
              )}
            </p>
          )}

          {error && <p className="error">{error}</p>}
        </aside>
      </div>
    </main>
  );
}
