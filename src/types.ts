// Espelha os structs serde do backend Rust (src-tauri/src/models.rs).
// Manter em sincronia: nomes em snake_case batem com #[serde] do Rust.

export interface ActiveWindow {
  app_name: string;
  app_bundle_id: string | null;
  title: string;
  pid: number;
}

export interface FocusSession {
  id: number;
  app_name: string;
  title: string;
  start_ts: number;
  duration_secs: number;
  was_idle_trimmed: boolean;
  category_ai: string | null; // categoria pela IA via conteúdo (não pelo app)
  activity_ai: string | null; // nome curto da atividade ("Estudando Cálculo")
  shot_path: string | null; // screenshot da sessão (D1, retenção 48h)
}

// Intervalo de ausência de dado real (todo minuto com dono na timeline).
export interface IntervalMarker {
  id: number;
  kind: "paused" | "away" | "away_uncertain";
  start_ts: number;
  end_ts: number | null; // null = ainda aberto
}

export interface AppTotal {
  app_name: string;
  total_secs: number;
  session_count: number;
  category: string;
}

export interface DailySummary {
  day: string;
  total_secs: number;
  session_count: number;
  by_app: AppTotal[];
  longest_session: FocusSession | null;
}

export interface DayTotal {
  day: string;
  total_secs: number;
}

export interface WeeklySummary {
  days: DayTotal[];
  total_secs: number;
  by_app: AppTotal[];
}

export interface CategoryTotal {
  category: string;
  total_secs: number;
}

export interface DayMetrics {
  total_tracked_secs: number;
  session_count: number;
  app_count: number;
  pomodoros: number;
  pomodoro_avg_secs: number;
  pomodoros_completed: number;
  paused_secs: number;
  away_secs: number;
}

export interface Insight {
  kind: string;
  text: string;
}

export interface GoalTime {
  goal: string;
  secs: number;
}

export interface Note {
  id: number;
  day: string;
  kind: string; // "intention" | "note"
  text: string;
  created_at: number;
}

export interface Todo {
  id: number;
  text: string;
  done: boolean;
  created_at: number;
  done_at: number | null;
}

export interface FocusCheck {
  focus: string | null;
  app: string | null;
  on_task: boolean | null;
  reason: string;
  source: string; // "user" | "rule" | "ia" | "none"
}

export interface Reminder {
  id: number;
  text: string;
  kind: "once" | "recurring";
  fire_at: number | null;
  interval_secs: number | null;
  enabled: boolean;
  last_fired_ts: number | null;
  created_at: number;
}
