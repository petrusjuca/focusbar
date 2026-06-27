import type { FocusSession, IntervalMarker } from "../types";
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

const GAP_LABEL: Record<string, string> = {
  paused: "Monitoramento pausado",
  away: "Ausente",
  away_uncertain: "Ausente (incerto)",
};

export function FocusTimeline({
  sessions,
  markers = [],
}: {
  sessions: FocusSession[];
  markers?: IntervalMarker[];
}) {
  if (sessions.length === 0 && markers.length === 0) return null;
  const now = Math.floor(Date.now() / 1000);

  // Janela do dia: cobre sessões E marcadores, pra todo minuto ter dono.
  const starts = [
    ...sessions.map((s) => s.start_ts),
    ...markers.map((m) => m.start_ts),
  ];
  const ends = [
    ...sessions.map((s) => s.start_ts + s.duration_secs),
    ...markers.map((m) => m.end_ts ?? now),
  ];
  const first = Math.min(...starts);
  const lastEnd = Math.max(...ends);
  const span = Math.max(1, lastEnd - first);

  const hasGaps = markers.length > 0;

  return (
    <section className="timeline">
      <div className="daily-header">
        <span>LINHA DO TEMPO</span>
        <span className="daily-day">
          {fmtTime(first)} – {fmtTime(lastEnd)}
        </span>
      </div>
      <div className="ribbon">
        {/* Marcadores PRIMEIRO (ficam atrás das sessões reais). */}
        {markers.map((m, i) => {
          const end = m.end_ts ?? now;
          const left = ((m.start_ts - first) / span) * 100;
          const width = Math.max(0.4, ((end - m.start_ts) / span) * 100);
          return (
            <div
              key={`m${i}`}
              className={`ribbon-gap gap-${m.kind}`}
              title={`${GAP_LABEL[m.kind] ?? m.kind} · ${fmtTime(m.start_ts)}`}
              style={{ left: `${left}%`, width: `${width}%` }}
            />
          );
        })}
        {sessions.map((s, i) => {
          const left = ((s.start_ts - first) / span) * 100;
          const width = Math.max(0.4, (s.duration_secs / span) * 100);
          return (
            <div
              key={`s${i}`}
              className="ribbon-seg"
              title={`${s.activity_ai ? s.activity_ai + " · " : ""}${s.app_name}${
                s.category_ai ? " (" + s.category_ai + ")" : ""
              } · ${fmtDuration(s.duration_secs)} · ${fmtTime(s.start_ts)}`}
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
      {hasGaps && (
        <div className="ribbon-legend">
          <span><i className="lg lg-work" /> foco</span>
          <span><i className="lg lg-paused" /> pausado</span>
          <span><i className="lg lg-away" /> ausente</span>
          <span><i className="lg lg-nodata" /> sem dados</span>
        </div>
      )}
    </section>
  );
}
