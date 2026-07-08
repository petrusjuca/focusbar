use serde::{Deserialize, Serialize};

/// Janela/app atualmente em foco. Serializa em snake_case para o frontend
/// (ver src/types.ts).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActiveWindow {
    pub app_name: String,
    pub app_bundle_id: Option<String>,
    pub title: String,
    pub pid: i32,
}

/// Uma sessão de foco já gravada (para listar no frontend).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FocusSession {
    pub id: i64,
    pub app_name: String,
    pub title: String,
    pub start_ts: i64,
    pub duration_secs: i64,
    pub was_idle_trimmed: bool,
    /// Categoria pela IA via CONTEÚDO da tela (não pelo nome do app). None = ainda
    /// não processado / sem Ollama → o dashboard cai na regra por app.
    pub category_ai: Option<String>,
    /// Nome curto da atividade deduzido do conteúdo ("Estudando Cálculo").
    pub activity_ai: Option<String>,
    /// Screenshot da sessão em disco (D1: retenção 48h) — "ver em que aba estava".
    pub shot_path: Option<String>,
}

/// Total de tempo por app num intervalo. `category` = override do usuário ou "".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppTotal {
    pub app_name: String,
    pub total_secs: i64,
    pub session_count: i64,
    pub category: String,
}

/// Resumo de um dia.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailySummary {
    pub day: String,
    pub total_secs: i64,
    pub session_count: i64,
    pub by_app: Vec<AppTotal>,
    pub longest_session: Option<FocusSession>,
}

/// Total de foco de um dia (para a visão semanal).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DayTotal {
    pub day: String,
    pub total_secs: i64,
}

/// Resumo dos últimos 7 dias.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeeklySummary {
    pub days: Vec<DayTotal>,
    pub total_secs: i64,
    pub by_app: Vec<AppTotal>,
}

/// Tempo total por categoria.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryTotal {
    pub category: String,
    pub total_secs: i64,
}

/// Intervalo de ausência de dado real na timeline. `kind`: "paused" | "away" |
/// "away_uncertain". `end_ts` None = ainda aberto (o frontend fecha em "agora").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntervalMarker {
    pub id: i64,
    pub kind: String,
    pub start_ts: i64,
    pub end_ts: Option<i64>,
}

/// Caminho do servidor MCP local + se o binário está presente no bundle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpInfo {
    pub path: String,
    pub exists: bool,
}

/// Métricas cruas do dia (painel de dados / debug — expõe tudo que capturamos).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DayMetrics {
    pub total_tracked_secs: i64,    // soma das durações de sessão
    pub session_count: i64,         // nº de sessões (= trocas de janela)
    pub app_count: i64,             // apps/sites distintos
    pub pomodoros: i64,             // blocos registrados no pomodoro_log
    pub pomodoro_avg_secs: i64,     // duração real média dos blocos
    pub pomodoros_completed: i64,   // blocos marcados como "terminei a tarefa"
    pub paused_secs: i64,           // tempo com monitoramento pausado
    pub away_secs: i64,             // tempo ausente (idle/lock)
}

/// Uma dica/insight do dia (kind controla o ícone no frontend).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Insight {
    pub kind: String,
    pub text: String,
}

/// Tempo de foco dedicado a um objetivo/tarefa (somatório dos blocos do dia).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalTime {
    pub goal: String,
    pub secs: i64,
}

/// Nota/intenção do dia. kind: "intention" | "note".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: i64,
    pub day: String,
    pub kind: String,
    pub text: String,
    pub created_at: i64,
}

/// Tarefa (to-do): algo a fazer, marca como feito. Persiste até concluir.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Todo {
    pub id: i64,
    pub text: String,
    pub done: bool,
    pub created_at: i64,
    pub done_at: Option<i64>,
    /// Duração própria da tarefa em segundos (tarefas de 10min ≠ de 90min).
    pub custom_secs: Option<i64>,
    /// Estimativa em pomodoros ("isso leva 2 🍅") — Rev4 #5.
    pub est_pomos: Option<i64>,
}

/// Um lembrete. `kind`: "once" (dispara uma vez em fire_at) ou
/// "recurring" (dispara a cada interval_secs).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reminder {
    pub id: i64,
    pub text: String,
    pub kind: String,
    pub fire_at: Option<i64>,
    pub interval_secs: Option<i64>,
    pub enabled: bool,
    pub last_fired_ts: Option<i64>,
    pub created_at: i64,
    /// "Chega por hoje": silenciado até este ts (lembretes v2).
    pub snoozed_until: Option<i64>,
}
