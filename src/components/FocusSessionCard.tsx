import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { fmtClock } from "../format";
import type { FocusSessionApi } from "../hooks/useFocusSession";

// Card do Modo Foco (Pomodoro) na aba Hoje. Inicia um bloco ligado ao foco atual.
export function FocusSessionCard({ session }: { session: FocusSessionApi }) {
  const { phase, remaining, pomodoros, goal, blockPaused, start, stop, toggleBlock } =
    session;
  const [custom, setCustom] = useState("");

  async function begin(minutes: number) {
    let f = "";
    try {
      f = (await invoke<string | null>("get_focus")) ?? "";
    } catch {
      /* ignore */
    }
    start(minutes, f);
  }

  function beginCustom() {
    const m = parseInt(custom, 10);
    if (!isNaN(m) && m > 0 && m <= 180) {
      begin(m);
      setCustom("");
    }
  }

  if (phase === "idle") {
    return (
      <div className="session-card">
        <div className="session-card-top">
          <span className="session-card-title">⏱️ Modo Foco</span>
          {pomodoros > 0 && <span className="pomos">🍅 {pomodoros} hoje</span>}
        </div>
        <p className="session-card-sub">
          Um bloco de foco protegido — eu cuido do tempo e te lembro da pausa.
        </p>
        <div className="session-card-actions">
          <button className="grant-btn" onClick={() => begin(25)}>
            ▶ Focar 25 min
          </button>
          <button className="link-btn" onClick={() => begin(15)}>
            15 min
          </button>
          <button className="link-btn" onClick={() => begin(50)}>
            50 min
          </button>
        </div>
        <div className="session-custom">
          <input
            className="session-custom-input"
            type="number"
            min={1}
            max={180}
            placeholder="min"
            value={custom}
            onChange={(e) => setCustom(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && beginCustom()}
          />
          <button className="link-btn" onClick={beginCustom} disabled={!custom}>
            ▶ tempo personalizado
          </button>
        </div>
      </div>
    );
  }

  const isBreak = phase === "break";
  return (
    <div
      className={`session-card running ${isBreak ? "brk" : "foc"}${
        blockPaused ? " frozen" : ""
      }`}
    >
      <div className="session-card-top">
        <span className="session-card-title">
          {blockPaused ? "⏸ Pausado" : isBreak ? "☕ Pausa" : "🟢 Em foco"}
        </span>
        {pomodoros > 0 && <span className="pomos">🍅 {pomodoros}</span>}
      </div>
      {!isBreak && goal && <div className="session-goal">🎯 {goal}</div>}
      <div className="session-clock">{fmtClock(remaining)}</div>
      <div className="session-card-actions">
        <button className="link-btn" onClick={toggleBlock}>
          {blockPaused ? "retomar" : "pausar"}
        </button>
        <button className="link-btn" onClick={stop}>
          {isBreak ? "pular pausa" : "parar bloco"}
        </button>
      </div>
    </div>
  );
}
