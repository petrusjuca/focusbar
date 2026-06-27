//! Configurações locais do app (key-value no SQLite) + tempo dedicado por tarefa.

use crate::commands::summaries::bounds_for_date_pub;
use crate::db;
use crate::models::GoalTime;
use crate::state::AppState;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;

fn now_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Registra tempo de foco dedicado a um objetivo (chamado no fim de um bloco).
#[tauri::command]
pub fn log_focus_time(state: State<AppState>, goal: String, secs: i64) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::log_focus_time(&conn, goal.trim(), secs, now_ts()).map_err(|e| e.to_string())
}

/// Registra um bloco Pomodoro concluído (goal, planejado, real, se concluiu).
#[tauri::command]
pub fn log_pomodoro(
    state: State<AppState>,
    goal: String,
    start_ts: i64,
    planned_secs: i64,
    actual_secs: i64,
    completed: bool,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::log_pomodoro(
        &conn,
        goal.trim(),
        start_ts,
        planned_secs,
        actual_secs,
        completed,
        now_ts(),
    )
    .map_err(|e| e.to_string())
}

/// Tempo dedicado por objetivo/tarefa no dia (pra "direcionar o dia pelo foco").
#[tauri::command]
pub fn get_focus_time(
    state: State<AppState>,
    day: Option<String>,
) -> Result<Vec<GoalTime>, String> {
    let (start, end) = bounds_for_date_pub(day.as_deref());
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::focus_time_by_goal(&conn, start, end).map_err(|e| e.to_string())
}

/// O OCR de pixel (Estágio 2 dos "olhos") está ligado?
#[tauri::command]
pub fn get_ocr_enabled(state: State<AppState>) -> Result<bool, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    Ok(db::get_setting(&conn, "ocr_enabled")
        .map_err(|e| e.to_string())?
        .map(|v| v == "1")
        .unwrap_or(false))
}

/// Liga/desliga o OCR de pixel. Desligado = só o Estágio 1 (texto via AX).
#[tauri::command]
pub fn set_ocr_enabled(state: State<AppState>, on: bool) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::set_setting(&conn, "ocr_enabled", if on { "1" } else { "0" }).map_err(|e| e.to_string())
}

/// Caminho do servidor MCP local (binário `mcp` ao lado do app) e se ele existe.
/// O front monta o comando do Claude a partir disso. É o "Claude lê seus dados
/// sem API": você aponta o seu Claude pro servidor, que lê o SQLite local.
#[tauri::command]
pub fn get_mcp_info() -> Result<crate::models::McpInfo, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let bin = if cfg!(target_os = "windows") {
        "mcp.exe"
    } else {
        "mcp"
    };
    let mcp = exe
        .parent()
        .map(|p| p.join(bin))
        .ok_or_else(|| "nao achei a pasta do app".to_string())?;
    Ok(crate::models::McpInfo {
        path: mcp.to_string_lossy().to_string(),
        exists: mcp.exists(),
    })
}

/// Lê uma config genérica (onboarding, horário de fim de dia…). None = não setada.
#[tauri::command]
pub fn get_setting(state: State<AppState>, key: String) -> Result<Option<String>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::get_setting(&conn, &key).map_err(|e| e.to_string())
}

/// Grava uma config genérica.
#[tauri::command]
pub fn set_setting(state: State<AppState>, key: String, value: String) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::set_setting(&conn, &key, &value).map_err(|e| e.to_string())
}

/// Categoriza, pela IA e pelo CONTEÚDO, algumas sessões ainda sem categoria.
/// Chamado pelo frontend de tempos em tempos. Para na 1ª falha (Ollama off).
/// Retorna quantas classificou. NUNCA segura o lock do DB atravessando um await.
#[tauri::command]
pub async fn categorize_pending(state: State<'_, AppState>) -> Result<u32, String> {
    let pending = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        db::sessions_needing_category(&conn, 2).map_err(|e| e.to_string())?
    };
    let mut done = 0u32;
    for (id, app, title, content) in pending {
        match crate::ai::categorize_activity(&app, &title, &content).await {
            Ok((cat, act)) => {
                if let Ok(conn) = state.db.lock() {
                    let _ = db::set_session_category(&conn, id, &cat, &act);
                    done += 1;
                }
            }
            Err(_) => break, // Ollama provavelmente indisponível — não insiste agora
        }
    }
    Ok(done)
}
