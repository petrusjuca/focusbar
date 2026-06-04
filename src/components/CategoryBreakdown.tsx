import type { CategoryTotal } from "../types";
import { fmtDuration } from "../format";

// Cores fixas por categoria (as desconhecidas caem em cinza).
const CAT_COLORS: Record<string, string> = {
  Trabalho: "#34c759",
  Ferramenta: "#007aff",
  Procrastinação: "#ff3b30",
  Outro: "#8e8e93",
};

function colorFor(cat: string): string {
  return CAT_COLORS[cat] ?? "#8e8e93";
}

export function CategoryBreakdown({ data }: { data: CategoryTotal[] }) {
  const total = data.reduce((s, c) => s + c.total_secs, 0);
  if (total === 0) return null;

  return (
    <section className="category">
      <div className="daily-header">
        <span>POR CATEGORIA</span>
      </div>

      <div className="cat-bar">
        {data.map((c, i) => (
          <div
            key={i}
            className="cat-seg"
            title={`${c.category}: ${fmtDuration(c.total_secs)}`}
            style={{
              width: `${(c.total_secs / total) * 100}%`,
              background: colorFor(c.category),
            }}
          />
        ))}
      </div>

      <div className="cat-legend">
        {data.map((c, i) => (
          <div key={i} className="cat-row">
            <span className="dot" style={{ background: colorFor(c.category) }} />
            <span className="cat-name">{c.category}</span>
            <span className="cat-pct">
              {Math.round((c.total_secs / total) * 100)}%
            </span>
            <span className="cat-dur">{fmtDuration(c.total_secs)}</span>
          </div>
        ))}
      </div>
    </section>
  );
}
