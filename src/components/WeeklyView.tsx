import {
  Bar,
  BarChart,
  CartesianGrid,
  Cell,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import type { WeeklySummary } from "../types";
import { fmtDuration } from "../format";

const COLORS = [
  "#007aff",
  "#34c759",
  "#ff9500",
  "#af52de",
  "#ff2d55",
  "#5ac8fa",
  "#ffcc00",
];

function weekdayLabel(day: string): string {
  // day = 'YYYY-MM-DD' → rótulo curto em PT, sem problema de fuso (meio-dia).
  const d = new Date(day + "T12:00:00");
  return d.toLocaleDateString("pt-BR", { weekday: "short" }).replace(".", "");
}

export function WeeklyView({ summary }: { summary: WeeklySummary }) {
  const data = summary.days.map((d) => ({
    label: weekdayLabel(d.day),
    minutes: Math.round((d.total_secs / 60) * 10) / 10,
    secs: d.total_secs,
  }));

  const avg = Math.round(summary.total_secs / 7);
  const topApps = summary.by_app.slice(0, 5);

  return (
    <section className="daily">
      <div className="daily-header">
        <span>ÚLTIMOS 7 DIAS</span>
      </div>

      <div className="streak-grid">
        <div className="stat-card">
          <div className="stat-label">FOCO NA SEMANA</div>
          <div className="stat-value">{fmtDuration(summary.total_secs)}</div>
          <div className="stat-sub">média {fmtDuration(avg)}/dia</div>
        </div>
        <div className="stat-card highlight">
          <div className="stat-label">APP TOP DA SEMANA</div>
          {topApps[0] ? (
            <>
              <div className="stat-value" style={{ fontSize: "1.3rem" }}>
                {topApps[0].app_name}
              </div>
              <div className="stat-sub">{fmtDuration(topApps[0].total_secs)}</div>
            </>
          ) : (
            <div className="stat-sub">sem dados</div>
          )}
        </div>
      </div>

      <div className="chart-wrap">
        <ResponsiveContainer width="100%" height={200}>
          <BarChart data={data} margin={{ top: 8, left: 0, right: 8 }}>
            <CartesianGrid vertical={false} stroke="rgba(0,0,0,0.06)" />
            <XAxis dataKey="label" tick={{ fontSize: 12 }} axisLine={false} tickLine={false} />
            <YAxis hide />
            <Tooltip
              formatter={(_v, _n, p) => [fmtDuration(p.payload.secs), "foco"]}
              cursor={{ fill: "rgba(0,0,0,0.04)" }}
            />
            <Bar dataKey="minutes" radius={[6, 6, 0, 0]}>
              {data.map((_, i) => (
                <Cell key={i} fill={COLORS[i % COLORS.length]} />
              ))}
            </Bar>
          </BarChart>
        </ResponsiveContainer>
      </div>

      {topApps.length > 0 && (
        <div className="week-apps">
          {topApps.map((a, i) => (
            <div key={i} className="week-app-row">
              <span className="dot" style={{ background: COLORS[i % COLORS.length] }} />
              <span className="week-app-name">{a.app_name}</span>
              <span className="week-app-dur">{fmtDuration(a.total_secs)}</span>
            </div>
          ))}
        </div>
      )}
    </section>
  );
}
