import type { FocusSession } from "../types";
import { fmtDuration, fmtTime } from "../format";

const COLORS = [
  "#007aff",
  "#34c759",
  "#ff9500",
  "#af52de",
  "#ff2d55",
  "#5ac8fa",
  "#ffcc00",
];

// Cor estável por nome de app.
function colorFor(name: string): string {
  let h = 0;
  for (let i = 0; i < name.length; i++) h = (h * 31 + name.charCodeAt(i)) | 0;
  return COLORS[Math.abs(h) % COLORS.length];
}

export function FocusTimeline({ sessions }: { sessions: FocusSession[] }) {
  if (sessions.length === 0) return null;

  // Janela do dia: do início da 1a sessão ao fim da última.
  const first = sessions[0].start_ts;
  const lastEnd = Math.max(
    ...sessions.map((s) => s.start_ts + s.duration_secs)
  );
  const span = Math.max(1, lastEnd - first);

  return (
    <section className="timeline">
      <div className="daily-header">
        <span>LINHA DO TEMPO</span>
        <span className="daily-day">
          {fmtTime(first)} – {fmtTime(lastEnd)}
        </span>
      </div>
      <div className="ribbon">
        {sessions.map((s, i) => {
          const left = ((s.start_ts - first) / span) * 100;
          const width = Math.max(0.4, (s.duration_secs / span) * 100);
          return (
            <div
              key={i}
              className="ribbon-seg"
              title={`${s.app_name} · ${fmtDuration(s.duration_secs)} · ${fmtTime(
                s.start_ts
              )}`}
              style={{
                left: `${left}%`,
                width: `${width}%`,
                background: colorFor(s.app_name),
                opacity: s.was_idle_trimmed ? 0.45 : 1,
              }}
            />
          );
        })}
      </div>
    </section>
  );
}
