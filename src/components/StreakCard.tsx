import type { DailySummary } from "../types";
import { fmtDuration, fmtTime } from "../format";

export function StreakCard({ summary }: { summary: DailySummary }) {
  const longest = summary.longest_session;
  return (
    <div className="streak-grid">
      <div className="stat-card">
        <div className="stat-label">FOCO TOTAL HOJE</div>
        <div className="stat-value">{fmtDuration(summary.total_secs)}</div>
        <div className="stat-sub">{summary.session_count} sessões</div>
      </div>
      <div className="stat-card highlight">
        <div className="stat-label">PICO DE FOCO</div>
        {longest ? (
          <>
            <div className="stat-value">{fmtDuration(longest.duration_secs)}</div>
            <div className="stat-sub">
              {longest.app_name} · {fmtTime(longest.start_ts)}
            </div>
          </>
        ) : (
          <div className="stat-sub">sem dados ainda</div>
        )}
      </div>
    </div>
  );
}
