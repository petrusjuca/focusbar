import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { fmtClock, parseDuration } from "../format";
import type { FocusSessionApi } from "../hooks/useFocusSession";
import { TimerRing } from "./TimerRing";
import { EditableClock } from "./EditableClock";

// Card do Modo Foco — fluxo do mundo real: foco → overtime (conta pra tarefa) →
// pausa → pós-pausa. Cada transição é confirmada por você, nada automático.
export function FocusSessionCard({ session }: { session: FocusSessionApi }) {
  const {
    phase,
    remaining,
    over,
    goal,
    blockPaused,
    pomodoros,
    breaksSkipped,
    start,
    stop,
    toggleBlock,
    startBreak,
    finishTask,
    skipBreak,
    startNext,
    extend,
  } = session;
  const [custom, setCustom] = useState("");
  const customSecs = parseDuration(custom);

  async function focusGoal(): Promise<string> {
    try {
      return (await invoke<string | null>("get_focus")) ?? "";
    } catch {
      return "";
    }
  }
  async function begin(seconds: number) {
    start(seconds, await focusGoal());
  }
  async function beginCustom() {
    if (customSecs) {
      await begin(customSecs);
      setCustom("");
    }
  }

  if (phase === "idle") {
    return (
      <div className="session-card">
        <div className="session-card-top">
          <span className="session-card-title">⏱️ Modo Foco</span>
          <span className="pomos" title="pomodoros concluídos hoje">
            🍅 {pomodoros} hoje
            {breaksSkipped > 0 && (
              <span title="pausas puladas hoje"> · ⏭ {breaksSkipped}</span>
            )}
          </span>
        </div>
        <p className="session-card-sub">
          Um bloco protegido — eu cuido do tempo e te lembro da pausa.
        </p>
        <div className="session-card-actions">
          <button className="grant-btn" onClick={() => begin(25 * 60)}>
            ▶ Focar 25 min
          </button>
          <button className="link-btn" onClick={() => begin(15 * 60)}>
            15
          </button>
          <button className="link-btn" onClick={() => begin(50 * 60)}>
            50
          </button>
        </div>
        <div className="session-custom">
          <input
            className="session-custom-input"
            placeholder="tempo livre: 1h30, 90, 25:00…"
            value={custom}
            onChange={(e) => setCustom(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && beginCustom()}
          />
          <button className="link-btn" onClick={beginCustom} disabled={!customSecs}>
            ▶ {customSecs ? fmtClock(customSecs) : "tempo livre"}
          </button>
        </div>
      </div>
    );
  }

  const isFocus = phase === "focus";
  const isOver = phase === "overtime";
  const isBreak = phase === "break";
  const isBreakOver = phase === "break_over";

  let title = "🟢 Em foco";
  if (blockPaused) title = "⏸ Pausado";
  else if (isOver) title = "✅ Tempo cumprido";
  else if (isBreak) title = "☕ Pausa";
  else if (isBreakOver) title = "☕ Pausa acabou";

  const clock = isOver || isBreakOver ? `+${fmtClock(over)}` : fmtClock(remaining);

  return (
    <div
      className={`session-card running ${isBreak || isBreakOver ? "brk" : "foc"}${
        blockPaused ? " frozen" : ""
      }${isOver || isBreakOver ? " ended" : ""}`}
    >
      <div className="session-card-top">
        <span className="session-card-title">{title}</span>
        <span className="pomos" title="pomodoros concluídos hoje">🍅 {pomodoros}</span>
      </div>
      {(isFocus || isOver) && goal && <div className="session-goal">🎯 {goal}</div>}
      <div className="session-ring">
        <TimerRing
          size={140}
          fraction={
            isOver || isBreakOver
              ? 1
              : session.total > 0
                ? remaining / session.total
                : 0
          }
        >
          <EditableClock
            text={clock}
            className="session-clock"
            onRetime={session.retime}
          />
        </TimerRing>
      </div>
      {isOver && (
        <p className="session-hint">
          passou do tempo — isso conta pra tarefa até você iniciar a pausa
        </p>
      )}
      {isBreakOver && <p className="session-hint">no seu tempo — bora quando voltar</p>}

      <div className="session-card-actions">
        {isFocus && (
          <>
            <button className="link-btn" onClick={toggleBlock}>
              {blockPaused ? "retomar" : "pausar"}
            </button>
            <button className="link-btn" onClick={() => extend(5)} title="+5 minutos">
              +5min
            </button>
            <button className="grant-btn" onClick={finishTask}>
              ✓ terminei
            </button>
            <button className="link-btn" onClick={stop}>
              parar
            </button>
          </>
        )}
        {isOver && (
          <>
            <button className="grant-btn" onClick={startBreak}>
              ☕ iniciar pausa
            </button>
            <button className="link-btn" onClick={() => extend(5)} title="mais 5min de foco">
              +5min foco
            </button>
            <button className="link-btn" onClick={finishTask}>
              ✓ terminei a tarefa
            </button>
            <button className="link-btn" onClick={stop}>
              parar
            </button>
          </>
        )}
        {isBreak && (
          <>
            <button className="grant-btn" onClick={() => startNext()}>
              ▶ próximo agora
            </button>
            <button className="link-btn" onClick={() => extend(5)} title="+5min de pausa">
              +5min
            </button>
            <button className="link-btn" onClick={skipBreak}>
              pular pausa
            </button>
          </>
        )}
        {isBreakOver && (
          <>
            <button className="grant-btn" onClick={() => startNext()}>
              ▶ próximo bloco
            </button>
            <button className="link-btn" onClick={skipBreak}>
              encerrar
            </button>
          </>
        )}
      </div>
    </div>
  );
}
