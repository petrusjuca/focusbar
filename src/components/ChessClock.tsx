import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { CategoryTotal } from "../types";
import { fmtDuration } from "../format";

// ♟️ Relógio Xadrez (FLOWMODE, ideia do Petrus): dois relógios com a mesma
// meta — produtividade × distração. O julgamento vem das categorias do dia
// (as mesmas do resto do app). Meta: bater as N horas produzindo ANTES do
// relógio da distração alcançar as dele. "Outro" não pontua pra ninguém.
const PRODUCTIVE = new Set(["Trabalho", "Estudo", "Ferramenta"]);
const UNPRODUCTIVE = new Set(["Procrastinação"]);

export function ChessClock({ categories }: { categories: CategoryTotal[] }) {
  const [goalH, setGoalH] = useState(4);
  useEffect(() => {
    invoke<string | null>("get_setting", { key: "chess_goal_hours" })
      .then((v) => {
        const n = v ? parseInt(v, 10) : 4;
        if (!isNaN(n) && n > 0) setGoalH(n);
      })
      .catch(() => {});
  }, []);
  async function changeGoal(h: number) {
    if (isNaN(h) || h < 1 || h > 16) return;
    setGoalH(h);
    try {
      await invoke("set_setting", { key: "chess_goal_hours", value: String(h) });
    } catch {
      /* ignore */
    }
  }

  const prod = categories
    .filter((c) => PRODUCTIVE.has(c.category))
    .reduce((a, c) => a + c.total_secs, 0);
  const bad = categories
    .filter((c) => UNPRODUCTIVE.has(c.category))
    .reduce((a, c) => a + c.total_secs, 0);
  if (prod + bad === 0) return null;

  const goal = goalH * 3600;
  const prodPct = Math.min(100, (prod * 100) / goal);
  const badPct = Math.min(100, (bad * 100) / goal);
  const prodWon = prod >= goal;
  const badWon = bad >= goal;

  let verdict = "";
  if (prodWon && !badWon) verdict = "🏆 Você venceu o relógio hoje!";
  else if (badWon && !prodWon) verdict = "🌧️ A distração cravou primeiro — amanhã tem revanche.";
  else if (prodWon && badWon) verdict = "⚖️ Os dois cravaram… dia intenso.";
  else if (prod > bad) verdict = "♟️ Você está na frente — segura a vantagem.";
  else if (bad > prod) verdict = "♟️ A distração está na frente — um pomodoro vira o jogo.";
  else verdict = "♟️ Empate técnico — o próximo lance decide.";

  return (
    <section className="chess-clock">
      <div className="daily-header">
        <span>♟️ RELÓGIO XADREZ</span>
        <span className="daily-day">
          meta:{" "}
          <input
            className="eod-hour"
            type="number"
            min={1}
            max={16}
            value={goalH}
            onChange={(e) => changeGoal(parseInt(e.target.value, 10))}
          />
          h
        </span>
      </div>
      <div className="chess-row prod">
        <span className="chess-label">✅ produzindo</span>
        <div className="chess-track">
          <div className="chess-fill" style={{ width: `${prodPct}%` }} />
        </div>
        <span className="chess-time">{fmtDuration(prod)}</span>
      </div>
      <div className="chess-row bad">
        <span className="chess-label">🍿 distração</span>
        <div className="chess-track">
          <div className="chess-fill" style={{ width: `${badPct}%` }} />
        </div>
        <span className="chess-time">{fmtDuration(bad)}</span>
      </div>
      <p className="chess-verdict">{verdict}</p>
    </section>
  );
}
