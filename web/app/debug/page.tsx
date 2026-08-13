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
 * Plies of opponent lookahead baked into the returned score. At 0 the score
 * matches a static (no-search) evaluation of the resulting board and lines
 * up exactly with the debug panel's position score; at 2 it additionally
 * accounts for the opponent's best anticipated reply (and your own reply to
 * that), so the search score no longer matches "the score of this exact
 * move" shown in the debug panel.
 */
const DEBUG_SEARCH_DEPTH = 1;

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

  /** Board cell currently under the pointer, for the debug coordinate readout. */
  const [hoverCell, setHoverCell] = useState<{ row: number; col: number } | null>(
    null,
  );
  const [copied, setCopied] = useState(false);

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

  const handleCopyPosition = useCallback(() => {
    navigator.clipboard
      .writeText(game)
      .then(() => {
        setCopied(true);
        setTimeout(() => setCopied(false), 1500);
      })
      .catch(() => setCopied(false));
  }, [game]);

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
          onHoverCell={setHoverCell}
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

            <p className="debug-eval__row">
              <span>Hovering:</span>{" "}
              {hoverCell ? `(${hoverCell.row}, ${hoverCell.col})` : "—"}
            </p>

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
                  <span>Current Board Scores:</span>{" "}
                  {humanMoveEval
                    ? `white ${humanMoveEval.scoreWhite} · black ${humanMoveEval.scoreBlack}`
                    : "—"}
                </p>
              </>
            )}
          </section>

          <div className="encoded-row">
            <details className="encoded">
              <summary>Encoded position</summary>
              <code>{game}</code>
            </details>
            <button
              type="button"
              className="icon-button"
              onClick={handleCopyPosition}
              aria-label="Copy encoded position to clipboard"
              title="Copy encoded position to clipboard"
            >
              {copied ? (
                <svg
                  viewBox="0 0 24 24"
                  width="16"
                  height="16"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  aria-hidden
                >
                  <polyline points="20 6 9 17 4 12" />
                </svg>
              ) : (
                <svg
                  viewBox="0 0 24 24"
                  width="16"
                  height="16"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  aria-hidden
                >
                  <rect x="9" y="9" width="12" height="12" rx="2" />
                  <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
                </svg>
              )}
            </button>
          </div>

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
