use crate::ai;
use crate::capture::{ActiveWinProvider, WindowProvider};
use crate::db;
use crate::redact;
use crate::state::AppState;
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;

fn now_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[tauri::command]
pub fn set_focus(state: State<AppState>, text: String) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    if text.trim().is_empty() {
        db::clear_focus(&conn).map_err(|e| e.to_string())
    } else {
        db::set_focus(&conn, text.trim(), now_ts()).map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub fn get_focus(state: State<AppState>) -> Result<Option<String>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::get_focus(&conn).map_err(|e| e.to_string())
}

#[derive(Serialize)]
pub struct FocusCheck {
    pub focus: Option<String>,
    pub app: Option<String>,
    pub on_task: Option<bool>,
    pub reason: String,
}

/// Checa, pela IA local, se a janela atual ajuda no foco declarado.
#[tauri::command]
pub async fn check_focus(state: State<'_, AppState>) -> Result<FocusCheck, String> {
    // Lê o foco e SOLTA o lock antes do await.
    let focus = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        db::get_focus(&conn).map_err(|e| e.to_string())?
    };
    let focus = match focus {
        Some(f) if !f.trim().is_empty() => f,
        _ => {
            return Ok(FocusCheck {
                focus: None,
                app: None,
                on_task: None,
                reason: "Defina um foco pra ele acompanhar.".into(),
            })
        }
    };

    let win = ActiveWinProvider.current();
    let (app, title) = match &win {
        Some(w) => (w.app_name.clone(), redact::redact(&w.title)),
        None => {
            return Ok(FocusCheck {
                focus: Some(focus),
                app: None,
                on_task: None,
                reason: "Sem janela em foco.".into(),
            })
        }
    };
    if app == "focusbar" {
        return Ok(FocusCheck {
            focus: Some(focus),
            app: Some(app),
            on_task: None,
            reason: "Você está no próprio focusbar.".into(),
        });
    }

    let (on_task, reason) = ai::on_task_check(&focus, &app, &title).await?;
    Ok(FocusCheck {
        focus: Some(focus),
        app: Some(app),
        on_task: Some(on_task),
        reason,
    })
}
