import { useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { LogicalSize } from "@tauri-apps/api/dpi";
import type { ActiveWindow, DailySummary } from "../types";
import { fmtDuration } from "../format";
import { useTodos } from "../hooks/useTodos";

// Widget compacto, flutuante e sempre-no-topo. Arrasta pela área (sem moldura).
export function MiniView({
  win,
  summary,
  paused,
  onExpand,
  onTogglePause,
}: {
  win: ActiveWindow | null;
  summary: DailySummary | null;
  paused: boolean;
  onExpand: () => void;
  onTogglePause: () => void;
}) {
  const [view, setView] = useState<"now" | "tarefas">("now");
  const { open, add, toggle } = useTodos(3000);
  const [text, setText] = useState("");

  async function switchView(v: "now" | "tarefas") {
    setView(v);
    try {
      await getCurrentWindow().setSize(
        new LogicalSize(244, v === "tarefas" ? 250 : 116)
      );
    } catch {
      /* ignore */
    }
  }

  const app = win?.app_name?.trim();
  const showApp = paused ? "pausado" : app && app !== "focusbar" ? app : app || "—";

  return (
    <div className={paused ? "mini paused" : "mini"} data-tauri-drag-region>
      <div className="mini-top">
        <div className="mini-tabs">
          <button
            className={view === "now" ? "mini-tab active" : "mini-tab"}
            onClick={() => switchView("now")}
          >
            Agora
          </button>
          <button
            className={view === "tarefas" ? "mini-tab active" : "mini-tab"}
            onClick={() => switchView("tarefas")}
          >
            Tarefas{open.length ? ` · ${open.length}` : ""}
          </button>
        </div>
        <div className="mini-actions">
          <button
            className="mini-icon"
            onClick={onTogglePause}
            title={paused ? "retomar" : "pausar"}
          >
            {paused ? "▶" : "⏸"}
          </button>
          <button className="mini-icon" onClick={onExpand} title="expandir">
            ⤢
          </button>
        </div>
      </div>

      {view === "now" ? (
        <>
          <div className="mini-app" title={win?.title || ""}>
            {showApp}
          </div>
          <div className="mini-foot">
            <span className="mini-focus-label">foco hoje</span>
            <span className="mini-focus-val">
              {summary ? fmtDuration(summary.total_secs) : "—"}
            </span>
          </div>
        </>
      ) : (
        <div className="mini-todos">
          <input
            className="mini-todo-input"
            placeholder="+ nova tarefa (Enter)"
            value={text}
            onChange={(e) => setText(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                add(text);
                setText("");
              }
            }}
          />
          <ul className="mini-todo-list">
            {open.length === 0 ? (
              <li className="mini-todo-empty">tudo feito ✓</li>
            ) : (
              open.slice(0, 6).map((t) => (
                <li key={t.id} className="mini-todo-row">
                  <button className="todo-check" onClick={() => toggle(t.id)} />
                  <span>{t.text}</span>
                </li>
              ))
            )}
          </ul>
        </div>
      )}
    </div>
  );
}
