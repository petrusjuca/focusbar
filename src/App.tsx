import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { LogicalSize } from "@tauri-apps/api/dpi";
import type {
  ActiveWindow,
  CategoryTotal,
  DailySummary,
  FocusSession,
  Insight,
  TaskRule,
  TaskTotal,
  WeeklySummary,
} from "./types";
import { fmtDuration, fmtTime } from "./format";
import { DailyView } from "./components/DailyView";
import { WeeklyView } from "./components/WeeklyView";
import { FocusTimeline } from "./components/FocusTimeline";
import { CategoryBreakdown } from "./components/CategoryBreakdown";
import { TaskBreakdown } from "./components/TaskBreakdown";
import { InsightsPanel } from "./components/InsightsPanel";
import { RemindersView } from "./components/RemindersView";
import { AssistantView } from "./components/AssistantView";
import { MiniView } from "./components/MiniView";
import { DiaryView } from "./components/DiaryView";
import "./App.css";

type Tab = "hoje" | "semana" | "assistente" | "lembretes";

function App() {
  const [tab, setTab] = useState<Tab>("hoje");
  const [win, setWin] = useState<ActiveWindow | null>(null);
  const [summary, setSummary] = useState<DailySummary | null>(null);
  const [weekly, setWeekly] = useState<WeeklySummary | null>(null);
  const [daySessions, setDaySessions] = useState<FocusSession[]>([]);
  const [categories, setCategories] = useState<CategoryTotal[]>([]);
  const [tasks, setTasks] = useState<TaskTotal[]>([]);
  const [taskRules, setTaskRules] = useState<TaskRule[]>([]);
  const [currentTask, setCurrentTask] = useState<string | null>(null);
  const [dayInsights, setDayInsights] = useState<Insight[]>([]);
  const [settingTask, setSettingTask] = useState(false);
  const [taskKw, setTaskKw] = useState("");
  const [taskName, setTaskName] = useState("");
  const [sessions, setSessions] = useState<FocusSession[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [hasAccess, setHasAccess] = useState<boolean>(true);
  const [autostart, setAutostart] = useState<boolean>(false);
  const [paused, setPaused] = useState<boolean>(false);
  const [mini, setMini] = useState<boolean>(false);

  useEffect(() => {
    let alive = true;

    async function poll() {
      try {
        const [result, access, daily, week, dayS, cats, tsum, trules, ctask, tips, recent] =
          await Promise.all([
            invoke<ActiveWindow | null>("get_active_window"),
            invoke<boolean>("check_accessibility"),
            invoke<DailySummary>("get_daily_summary", { day: null }),
            invoke<WeeklySummary>("get_weekly_summary", { endDay: null }),
            invoke<FocusSession[]>("get_day_sessions", { day: null }),
            invoke<CategoryTotal[]>("get_category_summary", { day: null }),
            invoke<TaskTotal[]>("get_task_summary", { day: null }),
            invoke<TaskRule[]>("list_task_rules"),
            invoke<string | null>("get_current_task"),
            invoke<Insight[]>("get_day_insights", { day: null }),
            invoke<FocusSession[]>("get_recent_sessions", { limit: 25 }),
          ]);
        if (alive) {
          setWin(result);
          setHasAccess(access);
          setSummary(daily);
          setWeekly(week);
          setDaySessions(dayS);
          setCategories(cats);
          setTasks(tsum);
          setTaskRules(trules);
          setCurrentTask(ctask);
          setDayInsights(tips);
          setSessions(recent);
          setError(null);
        }
      } catch (e) {
        if (alive) setError(String(e));
      }
    }

    poll();
    const id = setInterval(poll, 1500);
    return () => {
      alive = false;
      clearInterval(id);
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
      setError(String(e));
    }
  }

  async function enterMini() {
    try {
      const w = getCurrentWindow();
      await w.setSize(new LogicalSize(300, 175));
      await w.setAlwaysOnTop(true);
      setMini(true);
    } catch (e) {
      setError(String(e));
    }
  }

  async function exitMini() {
    try {
      const w = getCurrentWindow();
      await w.setAlwaysOnTop(false);
      await w.setSize(new LogicalSize(680, 820));
      setMini(false);
    } catch (e) {
      setError(String(e));
    }
  }

  async function grant() {
    try {
      await invoke<boolean>("request_accessibility");
    } catch (e) {
      setError(String(e));
    }
  }

  async function toggleAutostart() {
    try {
      const next = !autostart;
      await invoke("set_autostart", { enabled: next });
      setAutostart(next);
    } catch (e) {
      setError(String(e));
    }
  }

  async function reloadTasks() {
    const [tsum, trules, ctask] = await Promise.all([
      invoke<TaskTotal[]>("get_task_summary", { day: null }),
      invoke<TaskRule[]>("list_task_rules"),
      invoke<string | null>("get_current_task"),
    ]);
    setTasks(tsum);
    setTaskRules(trules);
    setCurrentTask(ctask);
  }

  function openSetTask() {
    setTaskKw((win?.app_name ?? "").toLowerCase());
    setTaskName("");
    setSettingTask(true);
  }

  async function saveTask() {
    if (!taskKw.trim() || !taskName.trim()) return;
    try {
      await invoke("create_task_rule", {
        keyword: taskKw.trim(),
        taskName: taskName.trim(),
      });
      setSettingTask(false);
      reloadTasks();
    } catch (e) {
      setError(String(e));
    }
  }

  if (mini) {
    return (
      <MiniView
        win={win}
        summary={summary}
        paused={paused}
        onExpand={exitMini}
        onTogglePause={togglePause}
      />
    );
  }

  return (
    <main className="container">
      <h1>focusbar</h1>
      <p className="subtitle">seu tempo, do seu jeito</p>

      <button
        className={paused ? "pause-btn paused" : "pause-btn"}
        onClick={togglePause}
      >
        {paused ? "▶ Retomar rastreamento (PAUSADO)" : "⏸ Pausar rastreamento"}
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

      <div className="card now-card">
        {win ? (
          <>
            <div className="now-label">AGORA</div>
            <div className="app-name">{win.app_name || "(sem nome)"}</div>
            <div className="title">
              {win.title || "(sem titulo — falta permissao?)"}
            </div>
            <div className="now-task">
              <span className="task-chip">
                task: <b>{currentTask ?? "—"}</b>
              </span>
              <button className="link-btn" onClick={openSetTask}>
                definir task
              </button>
            </div>
            {settingTask && (
              <div className="set-task">
                <input
                  placeholder="palavra no título"
                  value={taskKw}
                  onChange={(e) => setTaskKw(e.target.value)}
                />
                <input
                  placeholder="nome da task"
                  value={taskName}
                  onChange={(e) => setTaskName(e.target.value)}
                  onKeyDown={(e) => e.key === "Enter" && saveTask()}
                />
                <button className="grant-btn" onClick={saveTask}>
                  salvar
                </button>
                <button className="link-btn" onClick={() => setSettingTask(false)}>
                  cancelar
                </button>
              </div>
            )}
          </>
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

      {tab === "lembretes" ? (
        <RemindersView />
      ) : tab === "assistente" ? (
        <AssistantView />
      ) : tab === "semana" ? (
        weekly && <WeeklyView summary={weekly} />
      ) : (
        <>
          <DiaryView />
          <InsightsPanel insights={dayInsights} />
          {summary && <DailyView summary={summary} />}
          <CategoryBreakdown data={categories} />
          <TaskBreakdown data={tasks} rules={taskRules} onChange={reloadTasks} />
          <FocusTimeline sessions={daySessions} />

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
                {sessions.map((s, i) => (
                  <li key={i} className="session-row">
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
                    </div>
                  </li>
                ))}
              </ul>
            )}
          </div>
        </>
      )}

      <footer className="footer">
        <button className="mini-toggle" onClick={enterMini}>
          ⤡ Janela pequena (fica no canto da tela)
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
