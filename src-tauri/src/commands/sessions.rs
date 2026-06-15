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

/// Apaga uma sessão específica (o usuário escolhe — ex.: navegação sigilosa).
#[tauri::command]
pub fn delete_session(state: State<AppState>, id: i64) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::delete_session(&conn, id).map_err(|e| e.to_string())
}

/// Apaga todas as sessões de um app/site (pelo nome exibido).
#[tauri::command]
pub fn delete_app_sessions(state: State<AppState>, app_name: String) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::delete_app_sessions(&conn, &app_name)
        .map(|_| ())
        .map_err(|e| e.to_string())
}
