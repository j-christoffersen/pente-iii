"use client";

import { useState } from "react";

import type { OpponentMoveResponse } from "@/lib/game";

export default function Home() {
  const [game, setGame] = useState("15x15:" + ".".repeat(225));
  const [result, setResult] = useState<OpponentMoveResponse | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  async function fetchOpponentMove() {
    setLoading(true);
    setError(null);
    setResult(null);

    try {
      const res = await fetch("/api/opponent-move", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ game }),
      });

      const data = await res.json();
      if (!res.ok) {
        setError(data.error ?? "Request failed");
        return;
      }

      setResult(data as OpponentMoveResponse);
    } catch {
      setError("Network error");
    } finally {
      setLoading(false);
    }
  }

  return (
    <main>
      <h1>Pente</h1>
      <p>
        Encoded board position (compact <code>WxH:grid</code> or JSON). Opponent
        move search is stubbed until the Rust engine is wired in.
      </p>

      <label htmlFor="game">Encoded game</label>
      <textarea
        id="game"
        value={game}
        onChange={(e) => setGame(e.target.value)}
        spellCheck={false}
      />
      <button type="button" onClick={fetchOpponentMove} disabled={loading}>
        {loading ? "Searching…" : "Get opponent move"}
      </button>

      {error && <pre className="error">{error}</pre>}
      {result && <pre>{JSON.stringify(result, null, 2)}</pre>}
    </main>
  );
}
