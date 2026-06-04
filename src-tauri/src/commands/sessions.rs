use crate::db;
use crate::models::FocusSession;
use crate::state::AppState;
use tauri::State;

/// Últimas N sessões de foco gravadas (mais recentes primeiro).
#[tauri::command]
pub fn get_recent_sessions(state: State<AppState>, limit: i64) -> Result<Vec<FocusSession>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::recent_sessions(&conn, limit).map_err(|e| e.to_string())
}
