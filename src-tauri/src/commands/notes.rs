use crate::db;
use crate::models::Note;
use crate::state::AppState;
use chrono::Local;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;

fn now_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn today() -> String {
    Local::now().date_naive().format("%Y-%m-%d").to_string()
}

/// Adiciona uma nota/intenção. kind = "intention" ou "note".
#[tauri::command]
pub fn add_note(
    state: State<AppState>,
    text: String,
    kind: String,
    day: Option<String>,
) -> Result<i64, String> {
    if text.trim().is_empty() {
        return Err("texto vazio".into());
    }
    let day = day.unwrap_or_else(today);
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::notes::add(&conn, &day, &kind, text.trim(), now_ts()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_notes(state: State<AppState>, day: Option<String>) -> Result<Vec<Note>, String> {
    let day = day.unwrap_or_else(today);
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::notes::for_day(&conn, &day).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_note(state: State<AppState>, id: i64) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::notes::delete(&conn, id).map_err(|e| e.to_string())
}
