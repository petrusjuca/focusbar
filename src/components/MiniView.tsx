import type { ActiveWindow, DailySummary } from "../types";
import { fmtDuration } from "../format";

// Janela compacta sempre-no-topo: o essencial num cantinho da tela.
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
  return (
    <div className="mini">
      <div className="mini-top">
        <span className="mini-label">AGORA</span>
        <button className="mini-btn" onClick={onExpand} title="expandir">
          ⤢
        </button>
      </div>

      <div className="mini-app">{paused ? "⏸ pausado" : win?.app_name || "—"}</div>
      <div className="mini-title">{paused ? "" : win?.title || ""}</div>

      <div className="mini-bottom">
        <span className="mini-focus">
          foco hoje: <b>{summary ? fmtDuration(summary.total_secs) : "—"}</b>
        </span>
        <button
          className={paused ? "mini-pause paused" : "mini-pause"}
          onClick={onTogglePause}
        >
          {paused ? "▶" : "⏸"}
        </button>
      </div>
    </div>
  );
}
