import type { ActiveWindow, DailySummary } from "../types";
import { fmtDuration } from "../format";

// Widget compacto, flutuante e sempre-no-topo. Sem moldura — arrasta pela área.
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
  const app = win?.app_name?.trim();
  const showApp = paused ? "pausado" : app && app !== "focusbar" ? app : app || "—";

  return (
    <div className={paused ? "mini paused" : "mini"}>
      <div className="mini-top" data-tauri-drag-region>
        <span className="mini-now">{paused ? "PAUSADO" : "AGORA"}</span>
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

      <div className="mini-app" data-tauri-drag-region title={win?.title || ""}>
        {showApp}
      </div>

      <div className="mini-foot" data-tauri-drag-region>
        <span className="mini-focus-label">foco hoje</span>
        <span className="mini-focus-val">
          {summary ? fmtDuration(summary.total_secs) : "—"}
        </span>
      </div>
    </div>
  );
}
