use crate::db;
use crate::models::Todo;
use crate::state::AppState;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;

fn now_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[tauri::command]
pub fn add_todo(state: State<AppState>, text: String) -> Result<i64, String> {
    if text.trim().is_empty() {
        return Err("texto vazio".into());
    }
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::todos::add(&conn, text.trim(), now_ts()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_todos(state: State<AppState>) -> Result<Vec<Todo>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::todos::list(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn toggle_todo(state: State<AppState>, id: i64) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::todos::toggle(&conn, id, now_ts()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_todo(state: State<AppState>, id: i64) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::todos::delete(&conn, id).map_err(|e| e.to_string())
}
