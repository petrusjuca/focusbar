import type { Insight } from "../types";

const ICON: Record<string, string> = {
  warn: "⚠️",
  good: "✅",
  focus: "🎯",
  info: "💡",
};

export function InsightsPanel({ insights }: { insights: Insight[] }) {
  if (insights.length === 0) return null;
  return (
    <section className="insights">
      <div className="daily-header">
        <span>O QUE MELHORAR HOJE</span>
      </div>
      <ul className="insight-list">
        {insights.map((it, i) => (
          <li key={i} className={`insight-row ${it.kind}`}>
            <span className="insight-icon">{ICON[it.kind] ?? "•"}</span>
            <span>{it.text}</span>
          </li>
        ))}
      </ul>
    </section>
  );
}
