import { useEffect, useState, lazy, Suspense } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { LogicalSize } from "@tauri-apps/api/dpi";
import type {
  ActiveWindow,
  CategoryTotal,
  DailySummary,
  FocusSession,
  Insight,
  IntervalMarker,
  WeeklySummary,
} from "./types";
import { fmtClock, fmtDuration, fmtTime, friendlyError } from "./format";
import { FocusTimeline } from "./components/FocusTimeline";
import { CategoryBreakdown } from "./components/CategoryBreakdown";
import { InsightsPanel } from "./components/InsightsPanel";
import { RemindersView } from "./components/RemindersView";
import { AssistantView } from "./components/AssistantView";
import { MiniView } from "./components/MiniView";
import { SelfCheck } from "./components/SelfCheck";
import { DiaryView } from "./components/DiaryView";
import { TodoView } from "./components/TodoView";
import { FocusBar } from "./components/FocusBar";
import { CopyToClaudeButton } from "./components/CopyToClaudeButton";
import { FocusSessionCard } from "./components/FocusSessionCard";
import { DedicationToday } from "./components/DedicationToday";
import { useFocusSession } from "./hooks/useFocusSession";
import "./App.css";

// Lazy: estes usam recharts (pesado) — carregam num chunk separado, sob demanda.
const DailyView = lazy(() =>
  import("./components/DailyView").then((m) => ({ default: m.DailyView }))
);
const WeeklyView = lazy(() =>
  import("./components/WeeklyView").then((m) => ({ default: m.WeeklyView }))
);

const chartFallback = <div className="loading-state">carregando gráfico…</div>;

type Tab = "hoje" | "semana" | "assistente" | "lembretes";

function App() {
  const [tab, setTab] = useState<Tab>("hoje");
  const [win, setWin] = useState<ActiveWindow | null>(null);
  const [summary, setSummary] = useState<DailySummary | null>(null);
  const [weekly, setWeekly] = useState<WeeklySummary | null>(null);
  const [daySessions, setDaySessions] = useState<FocusSession[]>([]);
  const [dayMarkers, setDayMarkers] = useState<IntervalMarker[]>([]);
  const [categories, setCategories] = useState<CategoryTotal[]>([]);
  const [dayInsights, setDayInsights] = useState<Insight[]>([]);
  const [sessions, setSessions] = useState<FocusSession[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [hasAccess, setHasAccess] = useState<boolean>(true);
  const [autostart, setAutostart] = useState<boolean>(false);
  const [paused, setPaused] = useState<boolean>(false);
  const [mini, setMini] = useState<boolean>(false);
  const [loaded, setLoaded] = useState<boolean>(false);
  const [prevSize, setPrevSize] = useState<{ w: number; h: number } | null>(null);
  const [confirmDel, setConfirmDel] = useState<number | null>(null);
  const [showDetails, setShowDetails] = useState<boolean>(false);
  const session = useFocusSession();

  // Tarefa → Foco: define o foco e inicia um bloco Pomodoro JÁ naquela tarefa.
  async function focusTask(text: string) {
    try {
      await invoke("set_focus", { text });
    } catch (e) {
      setError(friendlyError(e));
    }
    session.start(25 * 60, text);
    setTab("hoje");
  }

  async function deleteSession(id: number) {
    setSessions((prev) => prev.filter((s) => s.id !== id)); // some já da tela
    setConfirmDel(null);
    try {
      await invoke("delete_session", { id });
    } catch (e) {
      setError(friendlyError(e));
    }
  }

  const [theme, setTheme] = useState<"dark" | "light">(() =>
    document.documentElement.dataset.theme === "light" ? "light" : "dark"
  );
  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    localStorage.setItem("theme", theme);
  }, [theme]);

  useEffect(() => {
    let alive = true;

    // Leve: janela ativa + permissão. Roda a cada 1,5s (barato, mantém o "AGORA" vivo).
    async function pollLight() {
      try {
        const [result, access] = await Promise.all([
          invoke<ActiveWindow | null>("get_active_window"),
          invoke<boolean>("check_accessibility"),
        ]);
        if (alive) {
          setWin(result);
          setHasAccess(access);
        }
      } catch (e) {
        if (alive) setError(friendlyError(e));
      }
    }

    // Pesado: resumos/sessões/insights (queries SQL). Só quando muda algo —
    // no evento focus-changed (fim de sessão) + uma rede de segurança a cada 20s.
    async function loadData() {
      try {
        const [daily, week, dayS, cats, tips, recent, marks] =
          await Promise.all([
            invoke<DailySummary>("get_daily_summary", { day: null }),
            invoke<WeeklySummary>("get_weekly_summary", { endDay: null }),
            invoke<FocusSession[]>("get_day_sessions", { day: null }),
            invoke<CategoryTotal[]>("get_category_summary", { day: null }),
            invoke<Insight[]>("get_day_insights", { day: null }),
            invoke<FocusSession[]>("get_recent_sessions", { limit: 25 }),
            invoke<IntervalMarker[]>("get_day_markers", { day: null }).catch(
              () => [] as IntervalMarker[]
            ),
          ]);
        if (alive) {
          setSummary(daily);
          setWeekly(week);
          setDaySessions(dayS);
          setCategories(cats);
          setDayInsights(tips);
          setSessions(recent);
          setDayMarkers(marks);
          setError(null);
          setLoaded(true);
        }
      } catch (e) {
        if (alive) setError(friendlyError(e));
      }
    }

    pollLight();
    loadData();
    const lightId = setInterval(pollLight, 1500);
    const heavyId = setInterval(loadData, 20000); // rede de segurança
    const unlisten = listen("focus-changed", () => loadData());
    // Pausa/retoma (vindo do app OU da bandeja) reflete na hora no banner.
    const unlistenPause = listen<boolean>("paused-changed", (e) => {
      setPaused(e.payload);
      loadData();
    });

    return () => {
      alive = false;
      clearInterval(lightId);
      clearInterval(heavyId);
      unlisten.then((un) => un());
      unlistenPause.then((un) => un());
    };
  }, []);

  useEffect(() => {
    invoke<boolean>("get_autostart").then(setAutostart).catch(() => {});
    const id = setInterval(() => {
      invoke<boolean>("get_paused").then(setPaused).catch(() => {});
    }, 1500);
    invoke<boolean>("get_paused").then(setPaused).catch(() => {});
    return () => clearInterval(id);
  }, []);

  async function togglePause() {
    const next = !paused;
    setPaused(next);
    try {
      await invoke("set_paused", { paused: next });
    } catch (e) {
      setError(friendlyError(e));
    }
  }

  async function enterMini() {
    try {
      const w = getCurrentWindow();
      // Guarda o tamanho atual pra restaurar exatamente ao sair do mini.
      const sz = await w.innerSize();
      const sf = await w.scaleFactor();
      setPrevSize({ w: Math.round(sz.width / sf), h: Math.round(sz.height / sf) });
      await w.setDecorations(false);
      await w.setAlwaysOnTop(true);
      document.documentElement.dataset.mini = "1";
      try {
        await w.setShadow(false);
      } catch {
        /* sombra nativa não é crítica */
      }
      await w.setSize(new LogicalSize(264, 150));
      setMini(true);
    } catch (e) {
      setError(friendlyError(e));
    }
  }

  async function exitMini() {
    try {
      const w = getCurrentWindow();
      delete document.documentElement.dataset.mini;
      try {
        await w.setShadow(true);
      } catch {
        /* sombra nativa não é crítica */
      }
      await w.setAlwaysOnTop(false);
      await w.setDecorations(true);
      const s = prevSize ?? { w: 800, h: 720 };
      await w.setSize(new LogicalSize(s.w, s.h));
      setMini(false);
    } catch (e) {
      setError(friendlyError(e));
    }
  }

  async function grant() {
    try {
      await invoke<boolean>("request_accessibility");
    } catch (e) {
      setError(friendlyError(e));
    }
  }

  async function toggleAutostart() {
    try {
      const next = !autostart;
      await invoke("set_autostart", { enabled: next });
      setAutostart(next);
    } catch (e) {
      setError(friendlyError(e));
    }
  }

  if (mini) {
    return (
      <MiniView
        paused={paused}
        session={session}
        onExpand={exitMini}
        onTogglePause={togglePause}
      />
    );
  }

  return (
    <main className="container">
      {paused && (
        <div className="pause-banner" role="status">
          ⏸ Monitoramento pausado — nada está sendo registrado.
          <button onClick={togglePause}>Retomar</button>
        </div>
      )}
      <h1 className="brand-compact">focusbar</h1>

      <button className="agent-cta" onClick={enterMini}>
        🧭 Abrir o agente de foco
        <span className="agent-cta-sub">vira uma janelinha no canto que te guia</span>
      </button>
      <SelfCheck />

      {session.phase !== "idle" &&
        (() => {
          const p = session.phase;
          const isBrk = p === "break" || p === "break_over";
          const counting = p === "overtime" || p === "break_over";
          const icon = session.blockPaused
            ? "⏸"
            : p === "overtime"
              ? "✅"
              : isBrk
                ? "☕"
                : "🟢";
          const clock = counting
            ? `+${fmtClock(session.over)}`
            : fmtClock(session.remaining);
          return (
            <div
              className={`run-strip ${isBrk ? "brk" : "foc"}${
                session.blockPaused ? " frozen" : ""
              }`}
            >
              <span className="run-clock">
                {icon} {clock}
              </span>
              {(p === "focus" || p === "overtime") && session.goal && (
                <span className="run-goal" title={session.goal}>
                  🎯 {session.goal}
                </span>
              )}
              {p === "focus" && (
                <>
                  <button className="link-btn" onClick={session.toggleBlock}>
                    {session.blockPaused ? "retomar" : "pausar"}
                  </button>
                  <button className="link-btn" onClick={session.finishTask}>
                    ✓ terminei
                  </button>
                  <button className="link-btn" onClick={session.stop}>
                    parar
                  </button>
                </>
              )}
              {p === "overtime" && (
                <>
                  <button className="link-btn" onClick={session.startBreak}>
                    ☕ pausa
                  </button>
                  <button className="link-btn" onClick={session.finishTask}>
                    ✓ terminei
                  </button>
                </>
              )}
              {p === "break" && (
                <>
                  <button className="link-btn" onClick={() => session.startNext()}>
                    próximo
                  </button>
                  <button className="link-btn" onClick={session.skipBreak}>
                    pular
                  </button>
                </>
              )}
              {p === "break_over" && (
                <>
                  <button className="link-btn" onClick={() => session.startNext()}>
                    ▶ próximo
                  </button>
                  <button className="link-btn" onClick={session.skipBreak}>
                    encerrar
                  </button>
                </>
              )}
            </div>
          );
        })()}

      <button
        className={paused ? "pause-btn paused" : "pause-btn"}
        onClick={togglePause}
      >
        {paused ? "▶ Retomar monitoramento (PAUSADO)" : "⏸ Pausar monitoramento"}
      </button>

      {error && <p className="error">erro: {error}</p>}

      {!hasAccess && (
        <div className="permission-banner">
          <div className="perm-title">Permissão de Acessibilidade necessária</div>
          <p className="perm-text">
            Pra ler o título das janelas (não os pixels), o macOS pede permissão
            de Acessibilidade. Sem ela, só o nome do app é rastreado.
          </p>
          <button className="grant-btn" onClick={grant}>
            Conceder permissão
          </button>
          <p className="perm-hint">
            Vai abrir Ajustes do Sistema → Privacidade e Segurança →
            Acessibilidade. Ative o focusbar na lista.
          </p>
        </div>
      )}

      <div className={paused ? "card now-card is-paused" : "card now-card"}>
        {paused ? (
          <div className="title">
            ⏸ Monitoramento pausado — nada está sendo registrado agora.
            <div className="paused-badge">PAUSADO</div>
          </div>
        ) : win ? (
          <div className="now-line" title={win.title || ""}>
            <span className="now-tag">AGORA</span>
            <span className="now-app">{win.app_name || "(sem nome)"}</span>
            {win.title && <span className="now-title">— {win.title}</span>}
          </div>
        ) : (
          <div className="title">nenhuma janela em foco (ou ocioso)</div>
        )}
      </div>

      <div className="tabs">
        <button
          className={tab === "hoje" ? "tab active" : "tab"}
          onClick={() => setTab("hoje")}
        >
          Hoje
        </button>
        <button
          className={tab === "semana" ? "tab active" : "tab"}
          onClick={() => setTab("semana")}
        >
          Semana
        </button>
        <button
          className={tab === "assistente" ? "tab active" : "tab"}
          onClick={() => setTab("assistente")}
        >
          Assistente
        </button>
        <button
          className={tab === "lembretes" ? "tab active" : "tab"}
          onClick={() => setTab("lembretes")}
        >
          Lembretes
        </button>
      </div>

      {!loaded && tab === "hoje" && (
        <div className="loading-state">carregando seus dados…</div>
      )}

      {tab === "lembretes" ? (
        <RemindersView />
      ) : tab === "assistente" ? (
        <AssistantView />
      ) : tab === "semana" ? (
        weekly ? (
          <Suspense fallback={chartFallback}>
            <WeeklyView summary={weekly} />
          </Suspense>
        ) : (
          <div className="loading-state">
            {loaded ? "Ainda sem dados na semana." : "carregando…"}
          </div>
        )
      ) : (
        <>
          {/* Núcleo acionável (TDAH: o que importa AGORA, sem rolagem infinita) */}
          <FocusBar />
          <FocusSessionCard session={session} />
          <TodoView onFocusTask={focusTask} />
          <InsightsPanel insights={dayInsights} />
          <DedicationToday refreshKey={session.pomodoros} />

          {/* Detalhes do dia — recolhidos por padrão pra não sobrecarregar */}
          <button
            className="details-toggle"
            onClick={() => setShowDetails((v) => !v)}
          >
            {showDetails
              ? "▴ esconder detalhes do dia"
              : "▾ ver meu dia em detalhe"}
          </button>

          {showDetails && (
            <>
              <DiaryView />
              <CopyToClaudeButton />
              {summary && (
                <Suspense fallback={chartFallback}>
                  <DailyView summary={summary} />
                </Suspense>
              )}
              <CategoryBreakdown data={categories} apps={summary?.by_app ?? []} />
              <FocusTimeline sessions={daySessions} markers={dayMarkers} />

              <div className="sessions">
            <div className="sessions-header">
              <span>SESSÕES RECENTES</span>
              <span className="sessions-count">{sessions.length}</span>
            </div>
            {sessions.length === 0 ? (
              <p className="empty">
                Nenhuma sessão ainda. Troque de app algumas vezes (cada janela
                precisa ficar em foco ≥2s) e elas aparecem aqui.
              </p>
            ) : (
              <ul className="session-list">
                {sessions.map((s) => (
                  <li key={s.id} className="session-row">
                    <div className="session-app">
                      {s.app_name}
                      {s.was_idle_trimmed && (
                        <span className="idle-tag">⏸ cortada (ocioso)</span>
                      )}
                    </div>
                    <div className="session-title">{s.title || "—"}</div>
                    <div className="session-meta">
                      <span>{fmtTime(s.start_ts)}</span>
                      <span className="session-dur">
                        {fmtDuration(s.duration_secs)}
                      </span>
                      {confirmDel === s.id ? (
                        <span className="session-confirm">
                          apagar?
                          <button
                            className="link-btn danger"
                            onClick={() => deleteSession(s.id)}
                          >
                            sim
                          </button>
                          <button
                            className="link-btn"
                            onClick={() => setConfirmDel(null)}
                          >
                            não
                          </button>
                        </span>
                      ) : (
                        <button
                          className="session-del"
                          title="apagar esta sessão do histórico"
                          onClick={() => setConfirmDel(s.id)}
                        >
                          🗑
                        </button>
                      )}
                    </div>
                  </li>
                ))}
              </ul>
            )}
              </div>
            </>
          )}
        </>
      )}

      <footer className="footer">
        <button
          className="mini-toggle"
          onClick={() => setTheme((t) => (t === "dark" ? "light" : "dark"))}
        >
          {theme === "dark" ? "☀️ Tema claro" : "🌙 Tema escuro"}
        </button>
        <label className="autostart">
          <input
            type="checkbox"
            checked={autostart}
            onChange={toggleAutostart}
          />
          iniciar com o sistema
        </label>
        <span className="bg-note">
          fechar a janela mantém o rastreamento rodando (ícone na barra de menu)
        </span>
      </footer>
    </main>
  );
}

export default App;
