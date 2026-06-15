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
