import {
  Bar,
  BarChart,
  Cell,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import type { DailySummary } from "../types";
import { fmtDuration } from "../format";
import { StreakCard } from "./StreakCard";

const COLORS = [
  "#007aff",
  "#34c759",
  "#ff9500",
  "#af52de",
  "#ff2d55",
  "#5ac8fa",
  "#ffcc00",
];

export function DailyView({ summary }: { summary: DailySummary }) {
  // Top 7 apps; minutos pro eixo ficar legível.
  const data = summary.by_app.slice(0, 7).map((a) => ({
    name: a.app_name.length > 18 ? a.app_name.slice(0, 17) + "…" : a.app_name,
    minutes: Math.round((a.total_secs / 60) * 10) / 10,
    secs: a.total_secs,
  }));

  return (
    <section className="daily">
      <div className="daily-header">
        <span>RESUMO DE HOJE</span>
        <span className="daily-day">{summary.day}</span>
      </div>

      <StreakCard summary={summary} />

      {data.length === 0 ? (
        <p className="empty">Sem tempo registrado hoje ainda.</p>
      ) : (
        <div className="chart-wrap">
          <ResponsiveContainer width="100%" height={Math.max(140, data.length * 38)}>
            <BarChart layout="vertical" data={data} margin={{ left: 8, right: 16 }}>
              <XAxis type="number" hide />
              <YAxis
                type="category"
                dataKey="name"
                width={120}
                tick={{ fontSize: 12 }}
              />
              <Tooltip
                formatter={(_v, _n, p) => [fmtDuration(p.payload.secs), "tempo"]}
                cursor={{ fill: "rgba(0,0,0,0.04)" }}
              />
              <Bar dataKey="minutes" radius={[0, 6, 6, 0]}>
                {data.map((_, i) => (
                  <Cell key={i} fill={COLORS[i % COLORS.length]} />
                ))}
              </Bar>
            </BarChart>
          </ResponsiveContainer>
        </div>
      )}
    </section>
  );
}
