"use client";

import {
  BOARD_SIZE,
  type Board,
  isStarPoint,
} from "@/lib/board";
import type { Player } from "@/lib/players";

interface BoardGridProps {
  board: Board;
  turn: Player;
  canPlay: boolean;
  onPlay: (row: number, col: number) => void;
  highlight?: { row: number; col: number } | null;
}

export function BoardGrid({
  board,
  turn,
  canPlay,
  onPlay,
  highlight,
}: BoardGridProps) {
  return (
    <div className="board-wrap">
      <div
        className="board"
        role="grid"
        aria-label={`${BOARD_SIZE} by ${BOARD_SIZE} Pente board`}
      >
        {board.map((row, rowIndex) =>
          row.map((cell, colIndex) => {
            const isHighlighted =
              highlight?.row === rowIndex && highlight?.col === colIndex;
            const star = cell === "empty" && isStarPoint(rowIndex, colIndex);
            const playable = canPlay && cell === "empty";

            return (
              <button
                key={`${rowIndex}-${colIndex}`}
                type="button"
                role="gridcell"
                className={[
                  "cell",
                  cell !== "empty" && `cell--${cell}`,
                  isHighlighted && "cell--highlight",
                  playable && "cell--playable",
                ]
                  .filter(Boolean)
                  .join(" ")}
                aria-label={`Row ${rowIndex + 1}, column ${colIndex + 1}, ${cell}`}
                disabled={!playable}
                onClick={() => onPlay(rowIndex, colIndex)}
              >
                {star && <span className="cell__star" aria-hidden />}
                {cell !== "empty" && (
                  <span className={`stone stone--${cell}`} aria-hidden />
                )}
              </button>
            );
          }),
        )}
      </div>
      <p className="board-turn" aria-live="polite">
        {canPlay ? "Your turn — Black" : `Opponent (${turn}) is moving…`}
      </p>
    </div>
  );
}
