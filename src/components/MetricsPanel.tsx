import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { CategoryTotal, DailySummary, DayMetrics } from "../types";
import { fmtDuration } from "../format";
import type { FocusSessionApi } from "../hooks/useFocusSession";

function Stat({ label, value, hint }: { label: string; value: string; hint?: string }) {
  return (
    <div className="metric">
      <div className="metric-value">{value}</div>
      <div className="metric-label">{label}</div>
      {hint && <div className="metric-hint">{hint}</div>}
    </div>
  );
}

// Painel de dados crus — expõe TUDO que o app captura (valida que a captura está
// certa + dá os números: Pomodoros, abas/trocas, presença, categorias).
export function MetricsPanel({ session }: { session: FocusSessionApi }) {
  const [m, setM] = useState<DayMetrics | null>(null);
  const [cats, setCats] = useState<CategoryTotal[]>([]);
  const [daily, setDaily] = useState<DailySummary | null>(null);

  async function load() {
    try {
      const [mm, cc, dd] = await Promise.all([
        invoke<DayMetrics>("get_metrics", { day: null }),
        invoke<CategoryTotal[]>("get_category_summary", { day: null }),
        invoke<DailySummary>("get_daily_summary", { day: null }),
      ]);
      setM(mm);
      setCats(cc);
      setDaily(dd);
    } catch {
      /* ignore */
    }
  }
  useEffect(() => {
    load();
    const id = setInterval(load, 15000);
    return () => clearInterval(id);
  }, []);

  if (!m) return <div className="loading-state">carregando dados…</div>;
  const d = (s: number) => fmtDuration(s);

  return (
    <section className="metrics">
      <div className="daily-header">
        <span>POMODORO</span>
      </div>
      <div className="metric-grid">
        <Stat label="pomodoros" value={`${m.pomodoros}`} hint={`${m.pomodoros_completed} concluídos`} />
        <Stat label="duração média" value={m.pomodoros ? d(m.pomodoro_avg_secs) : "—"} />
        <Stat label="pausas puladas hoje" value={`${session.breaksSkipped}`} />
      </div>

      <div className="daily-header">
        <span>ATIVIDADE</span>
      </div>
      <div className="metric-grid">
        <Stat label="tempo rastreado" value={d(m.total_tracked_secs)} />
        <Stat label="sessões (trocas)" value={`${m.session_count}`} />
        <Stat label="apps/sites distintos" value={`${m.app_count}`} />
      </div>

      <div className="daily-header">
        <span>PRESENÇA</span>
      </div>
      <div className="metric-grid">
        <Stat label="pausado" value={d(m.paused_secs)} />
        <Stat label="ausente (idle/lock)" value={d(m.away_secs)} />
      </div>

      <div className="daily-header">
        <span>POR CATEGORIA (pelo conteúdo)</span>
      </div>
      {cats.length === 0 ? (
        <p className="empty">sem dados ainda.</p>
      ) : (
        <ul className="session-list">
          {cats.map((c) => (
            <li key={c.category} className="session-row">
              <div className="session-app">{c.category}</div>
              <div className="metric-dur">{d(c.total_secs)}</div>
            </li>
          ))}
        </ul>
      )}

      <div className="daily-header">
        <span>TOP APPS / SITES</span>
      </div>
      {!daily || daily.by_app.length === 0 ? (
        <p className="empty">sem dados ainda.</p>
      ) : (
        <ul className="session-list">
          {daily.by_app.slice(0, 10).map((a) => (
            <li key={a.app_name} className="session-row">
              <div className="session-app">{a.app_name}</div>
              <div className="metric-dur">
                {d(a.total_secs)} · {a.session_count}×
              </div>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
