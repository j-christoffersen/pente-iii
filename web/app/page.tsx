"use client";

import { useCallback, useMemo, useState } from "react";

import { BoardGrid } from "@/components/BoardGrid";
import {
  applyMove,
  createEmptyBoard,
  encodeBoard,
  hasFiveInARow,
  type Board,
  type Cell,
} from "@/lib/board";
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
 * Zero plies of opponent lookahead: the returned score is the move's own
 * immediate impact only, matching a static (no-search) evaluation of the
 * resulting board. Depth >= 1 would bake the opponent's best anticipated
 * reply into the score, which no longer matches "the score of this exact
 * move."
 */
const DEBUG_SEARCH_DEPTH = 0;

/** Standard Pente win condition: 5 captured pairs. */
const CAPTURE_PAIRS_TO_WIN = 5;

type WinReason = "five-in-a-row" | "captures";

function checkWin(
  board: Board,
  row: number,
  col: number,
  player: Player,
  capturePairs: number,
): WinReason | null {
  if (capturePairs >= CAPTURE_PAIRS_TO_WIN) {
    return "captures";
  }
  if (hasFiveInARow(board, row, col, player)) {
    return "five-in-a-row";
  }
  return null;
}

function winMessage(player: Player, reason: WinReason): string {
  const subject = playerLabel(player);
  return reason === "captures"
    ? `${subject} wins by capturing ${CAPTURE_PAIRS_TO_WIN} pairs!`
    : `${subject} wins with five in a row!`;
}

export default function Home() {
  const [board, setBoard] = useState<Board>(() => createEmptyBoard());
  const [turn, setTurn] = useState<Player>(HUMAN_PLAYER);
  const [lastOpponentMove, setLastOpponentMove] = useState<MoveScore | null>(
    null,
  );
  /** Pairs captured, by capturing player (5 pairs wins the game). */
  const [captures, setCaptures] = useState<Record<Player, number>>({
    black: 0,
    white: 0,
  });
  const [winner, setWinner] = useState<{ player: Player; reason: WinReason } | null>(
    null,
  );
  const [humanMoveEval, setHumanMoveEval] = useState<EvaluateResponse | null>(
    null,
  );
  const [opponentMoveEval, setOpponentMoveEval] =
    useState<EvaluateResponse | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  /** Debug mode: freely add/remove stones of either color to see the score. */
  const [debugMode, setDebugMode] = useState(false);
  const [debugColor, setDebugColor] = useState<Player>(HUMAN_PLAYER);
  const [debugBoardEval, setDebugBoardEval] =
    useState<EvaluateResponse | null>(null);

  const game = useMemo(() => encodeBoard(board), [board]);
  const canPlay = turn === HUMAN_PLAYER && !loading && !winner && !debugMode;

  const highlight = useMemo(() => {
    if (!lastOpponentMove) {
      return null;
    }
    return { row: lastOpponentMove.row, col: lastOpponentMove.col };
  }, [lastOpponentMove]);

  const resetGame = useCallback(() => {
    setBoard(createEmptyBoard());
    setTurn(HUMAN_PLAYER);
    setLastOpponentMove(null);
    setCaptures({ black: 0, white: 0 });
    setWinner(null);
    setHumanMoveEval(null);
    setOpponentMoveEval(null);
    setError(null);
  }, []);

  const toggleDebugMode = useCallback(() => {
    setDebugMode((prev) => {
      const next = !prev;
      if (next) {
        evaluatePosition(encodeBoard(board))
          .then(setDebugBoardEval)
          .catch(() => setDebugBoardEval(null));
      }
      return next;
    });
  }, [board]);

  const handleDebugCellClick = useCallback(
    (row: number, col: number) => {
      const current = board[row]?.[col];
      if (current === undefined) {
        return;
      }
      const nextCell: Cell = current === "empty" ? debugColor : "empty";
      const nextBoard = board.map((r, ri) =>
        ri === row ? r.map((c, ci) => (ci === col ? nextCell : c)) : r,
      );
      setBoard(nextBoard);
      evaluatePosition(encodeBoard(nextBoard))
        .then(setDebugBoardEval)
        .catch(() => setDebugBoardEval(null));
    },
    [board, debugColor],
  );

  const clearDebugBoard = useCallback(() => {
    const empty = createEmptyBoard();
    setBoard(empty);
    evaluatePosition(encodeBoard(empty))
      .then(setDebugBoardEval)
      .catch(() => setDebugBoardEval(null));
  }, []);

  const runOpponentTurn = useCallback(
    async (boardAfterHuman: Board) => {
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

        const pairs = result.captured.length / 2;
        const totalCaptures = captures[OPPONENT_PLAYER] + pairs;
        if (pairs > 0) {
          setCaptures((prev) => ({
            ...prev,
            [OPPONENT_PLAYER]: prev[OPPONENT_PLAYER] + pairs,
          }));
        }

        const win = checkWin(
          result.board,
          move.row,
          move.col,
          OPPONENT_PLAYER,
          totalCaptures,
        );
        if (win) {
          setWinner({ player: OPPONENT_PLAYER, reason: win });
        } else {
          setTurn(HUMAN_PLAYER);
        }

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
    },
    [captures],
  );

  const handlePlay = useCallback(
    async (row: number, col: number) => {
      if (!canPlay || board[row]?.[col] !== "empty") {
        return;
      }

      const result = applyMove(board, row, col, HUMAN_PLAYER);
      setBoard(result.board);

      const pairs = result.captured.length / 2;
      const totalCaptures = captures[HUMAN_PLAYER] + pairs;
      if (pairs > 0) {
        setCaptures((prev) => ({
          ...prev,
          [HUMAN_PLAYER]: prev[HUMAN_PLAYER] + pairs,
        }));
      }
      setLastOpponentMove(null);
      setOpponentMoveEval(null);

      try {
        setHumanMoveEval(await evaluatePosition(encodeBoard(result.board)));
      } catch {
        setHumanMoveEval(null);
      }

      const win = checkWin(result.board, row, col, HUMAN_PLAYER, totalCaptures);
      if (win) {
        setWinner({ player: HUMAN_PLAYER, reason: win });
        return;
      }

      setTurn(OPPONENT_PLAYER);
      await runOpponentTurn(result.board);
    },
    [board, canPlay, captures, runOpponentTurn],
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
          onPlay={debugMode ? handleDebugCellClick : handlePlay}
          highlight={highlight}
          debugMode={debugMode}
        />

        <aside className="sidebar">
          {winner ? (
            <p className="winner-banner">{winMessage(winner.player, winner.reason)}</p>
          ) : debugMode ? (
            <p className="turn-status">Debug mode — editing the board</p>
          ) : (
            <p className="turn-status">
              <span
                className={`turn-dot turn-dot--${turn}`}
                aria-hidden
              />
              {loading
                ? `${playerLabel(OPPONENT_PLAYER)} is thinking…`
                : `${playerLabel(turn)} to move`}
            </p>
          )}

          {winner && (
            <button type="button" className="new-game" onClick={resetGame}>
              New game
            </button>
          )}

          <ul className="captures">
            <li>
              {playerLabel(HUMAN_PLAYER)} captures: {captures[HUMAN_PLAYER]}/
              {CAPTURE_PAIRS_TO_WIN}
            </li>
            <li>
              {playerLabel(OPPONENT_PLAYER)} captures: {captures[OPPONENT_PLAYER]}/
              {CAPTURE_PAIRS_TO_WIN}
            </li>
          </ul>

          <section className="debug-eval">
            <div className="debug-eval__header">
              <h2>Debug: position score</h2>
              <button
                type="button"
                className="debug-toggle"
                onClick={toggleDebugMode}
                disabled={loading}
              >
                {debugMode ? "Exit debug mode" : "Edit board"}
              </button>
            </div>

            {debugMode ? (
              <>
                <div className="debug-color-picker">
                  <span>Placing:</span>
                  <button
                    type="button"
                    className={debugColor === "black" ? "active" : ""}
                    onClick={() => setDebugColor("black")}
                  >
                    Black
                  </button>
                  <button
                    type="button"
                    className={debugColor === "white" ? "active" : ""}
                    onClick={() => setDebugColor("white")}
                  >
                    White
                  </button>
                </div>
                <p className="debug-eval__row">
                  <span>Score:</span>{" "}
                  {debugBoardEval
                    ? `white ${debugBoardEval.scoreWhite} · black ${debugBoardEval.scoreBlack}`
                    : "—"}
                </p>
                <button type="button" className="clear-board" onClick={clearDebugBoard}>
                  Clear board
                </button>
              </>
            ) : (
              <>
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
              </>
            )}
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
                <>
                  {" "}
                  · search score {lastOpponentMove.score} (depth{" "}
                  {lastOpponentMove.depth})
                </>
              )}
            </p>
          )}

          {error && <p className="error">{error}</p>}
        </aside>
      </div>
    </main>
  );
}
