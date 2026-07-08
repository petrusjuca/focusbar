use crate::db;
use crate::models::Reminder;
use crate::state::AppState;
use chrono::{Duration as ChDuration, Local, TimeZone};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;

fn now_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[tauri::command]
pub fn list_reminders(state: State<AppState>) -> Result<Vec<Reminder>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::reminders::list(&conn).map_err(|e| e.to_string())
}

/// Cria um lembrete. kind = "once" (usa fire_at) ou "recurring" (usa interval_secs).
#[tauri::command]
pub fn create_reminder(
    state: State<AppState>,
    text: String,
    kind: String,
    fire_at: Option<i64>,
    interval_secs: Option<i64>,
) -> Result<i64, String> {
    if text.trim().is_empty() {
        return Err("texto vazio".into());
    }
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::reminders::create(&conn, text.trim(), &kind, fire_at, interval_secs, now_ts())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_reminder_enabled(
    state: State<AppState>,
    id: i64,
    enabled: bool,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::reminders::set_enabled(&conn, id, enabled).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_reminder(state: State<AppState>, id: i64) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::reminders::delete(&conn, id).map_err(|e| e.to_string())
}

/// "Chega por hoje": silencia o lembrete até a próxima meia-noite LOCAL —
/// amanhã ele volta sozinho (lembretes v2 do FLOWMODE).
#[tauri::command]
pub fn snooze_reminder_today(state: State<AppState>, id: i64) -> Result<(), String> {
    let tomorrow = Local::now().date_naive() + ChDuration::days(1);
    let midnight = tomorrow
        .and_hms_opt(0, 0, 0)
        .and_then(|n| Local.from_local_datetime(&n).earliest())
        .map(|t| t.timestamp())
        .unwrap_or_else(|| now_ts() + 24 * 3600);
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::reminders::snooze_until(&conn, id, midnight).map_err(|e| e.to_string())
}
