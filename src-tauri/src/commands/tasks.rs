use crate::capture::{ActiveWinProvider, WindowProvider};
use crate::db;
use crate::models::{TaskRule, TaskTotal};
use crate::state::AppState;
use std::collections::HashMap;
use tauri::State;

#[tauri::command]
pub fn list_task_rules(state: State<AppState>) -> Result<Vec<TaskRule>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::tasks::list(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_task_rule(
    state: State<AppState>,
    keyword: String,
    task_name: String,
) -> Result<i64, String> {
    if keyword.trim().is_empty() || task_name.trim().is_empty() {
        return Err("keyword e task vazios".into());
    }
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::tasks::create(&conn, keyword.trim(), task_name.trim()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_task_rule(state: State<AppState>, id: i64) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::tasks::delete(&conn, id).map_err(|e| e.to_string())
}

/// Task detectada para a janela em foco agora (ou null se nenhuma regra bate).
#[tauri::command]
pub fn get_current_task(state: State<AppState>) -> Result<Option<String>, String> {
    let win = match ActiveWinProvider.current() {
        Some(w) => w,
        None => return Ok(None),
    };
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let rules = db::tasks::list(&conn).map_err(|e| e.to_string())?;
    Ok(db::tasks::match_task(&rules, &win.app_name, &win.title).map(|s| s.to_string()))
}

/// Tempo por task num dia. Sessões sem regra caem em "(sem task)".
#[tauri::command]
pub fn get_task_summary(
    state: State<AppState>,
    day: Option<String>,
) -> Result<Vec<TaskTotal>, String> {
    use crate::commands::summaries::bounds_for_date_pub;
    let (start, end) = bounds_for_date_pub(day.as_deref());
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let rules = db::tasks::list(&conn).map_err(|e| e.to_string())?;
    let sessions = db::sessions_in_range(&conn, start, end).map_err(|e| e.to_string())?;

    let mut totals: HashMap<String, i64> = HashMap::new();
    for s in &sessions {
        let task = db::tasks::match_task(&rules, &s.app_name, &s.title)
            .map(|t| t.to_string())
            .unwrap_or_else(|| "(sem task)".to_string());
        *totals.entry(task).or_insert(0) += s.duration_secs;
    }

    let mut out: Vec<TaskTotal> = totals
        .into_iter()
        .map(|(task, total_secs)| TaskTotal { task, total_secs })
        .collect();
    out.sort_by(|a, b| b.total_secs.cmp(&a.total_secs));
    Ok(out)
}
