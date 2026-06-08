mod ai;
mod capture;
mod category;
mod coach;
mod commands;
mod db;
mod insights;
mod models;
mod redact;
mod reminders;
mod state;

use capture::{ActiveWinProvider, WindowProvider};
use commands::assistant::{
    ai_available, ai_day_digest, ai_day_review, ai_pull_model, ai_status,
};
use commands::notes::{add_note, delete_note, list_notes};
use commands::todos::{add_todo, delete_todo, list_todos, toggle_todo};
use commands::permissions::{check_accessibility, request_accessibility};
use commands::reminders::{
    create_reminder, delete_reminder, list_reminders, set_reminder_enabled,
};
use commands::sessions::get_recent_sessions;
use commands::summaries::{
    get_category_summary, get_daily_summary, get_day_insights, get_day_sessions,
    get_weekly_summary, set_app_category,
};
use commands::tasks::{
    create_task_rule, delete_task_rule, get_current_task, get_task_summary, list_task_rules,
};
use models::ActiveWindow;
use state::AppState;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, State, WindowEvent};
use tauri_plugin_autostart::{ManagerExt, MacosLauncher};

/// Lê a janela/app atualmente em foco. Chamado pelo frontend a cada ~1s.
#[tauri::command]
fn get_active_window() -> Option<ActiveWindow> {
    ActiveWinProvider.current()
}

/// Está configurado para iniciar com o sistema?
#[tauri::command]
fn get_autostart(app: AppHandle) -> bool {
    app.autolaunch().is_enabled().unwrap_or(false)
}

/// Liga/desliga iniciar com o sistema.
#[tauri::command]
fn set_autostart(app: AppHandle, enabled: bool) -> Result<(), String> {
    let m = app.autolaunch();
    let res = if enabled { m.enable() } else { m.disable() };
    res.map_err(|e| e.to_string())
}

/// O rastreamento está pausado?
#[tauri::command]
fn get_paused(state: State<AppState>) -> bool {
    state.paused.load(Ordering::Relaxed)
}

/// Pausa/retoma o rastreamento (pausado = não conta nada).
#[tauri::command]
fn set_paused(state: State<AppState>, paused: bool) {
    state.paused.store(paused, Ordering::Relaxed);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(|app| {
            // Banco em app_data_dir (por máquina, nunca sincronizado).
            let dir = app
                .path()
                .app_data_dir()
                .expect("nao foi possivel resolver app_data_dir");
            std::fs::create_dir_all(&dir).ok();
            let db_path = dir.join("focusbar.db");
            let conn = db::open(&db_path).expect("falha ao abrir o banco");
            let db = Arc::new(Mutex::new(conn));
            let paused = Arc::new(AtomicBool::new(false));

            app.manage(AppState {
                db: db.clone(),
                paused: paused.clone(),
            });

            // Sobe o sampler de foco e o scheduler de lembretes em background.
            capture::sampler::spawn(app.handle().clone(), db.clone(), paused);
            reminders::scheduler::spawn(app.handle().clone(), db);

            // Ícone na barra de menu (tray) com menu Abrir / Pausar / Sair.
            let show = MenuItem::with_id(app, "show", "Abrir focusbar", true, None::<&str>)?;
            let pause = MenuItem::with_id(app, "pause", "Pausar / retomar", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Sair", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &pause, &quit])?;
            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("focusbar")
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "pause" => {
                        let st = app.state::<AppState>();
                        let now = st.paused.load(Ordering::Relaxed);
                        st.paused.store(!now, Ordering::Relaxed);
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;

            Ok(())
        })
        // Fechar a janela = esconder (mantém o rastreamento rodando).
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_active_window,
            check_accessibility,
            request_accessibility,
            get_recent_sessions,
            get_daily_summary,
            get_day_sessions,
            get_weekly_summary,
            get_category_summary,
            list_reminders,
            create_reminder,
            set_reminder_enabled,
            delete_reminder,
            get_autostart,
            set_autostart,
            get_paused,
            set_paused,
            get_day_insights,
            list_task_rules,
            create_task_rule,
            delete_task_rule,
            get_current_task,
            get_task_summary,
            ai_available,
            ai_day_review,
            ai_day_digest,
            ai_status,
            ai_pull_model,
            add_note,
            list_notes,
            delete_note,
            set_app_category,
            add_todo,
            list_todos,
            toggle_todo,
            delete_todo
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
