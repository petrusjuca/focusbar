mod capture;
mod category;
mod coach;
mod commands;
mod db;
mod insights;
mod models;
mod reminders;
mod state;

use capture::{ActiveWinProvider, WindowProvider};
use commands::permissions::{check_accessibility, request_accessibility};
use commands::reminders::{
    create_reminder, delete_reminder, list_reminders, set_reminder_enabled,
};
use commands::sessions::get_recent_sessions;
use commands::summaries::{
    get_category_summary, get_daily_summary, get_day_insights, get_day_sessions,
    get_weekly_summary,
};
use commands::tasks::{
    create_task_rule, delete_task_rule, get_current_task, get_task_summary, list_task_rules,
};
use models::ActiveWindow;
use state::AppState;
use std::sync::{Arc, Mutex};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, WindowEvent};
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

            app.manage(AppState { db: db.clone() });

            // Sobe o sampler de foco e o scheduler de lembretes em background.
            capture::sampler::spawn(app.handle().clone(), db.clone());
            reminders::scheduler::spawn(app.handle().clone(), db);

            // Ícone na barra de menu (tray) com menu Abrir/Sair.
            let show = MenuItem::with_id(app, "show", "Abrir focusbar", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Sair", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;
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
            get_day_insights,
            list_task_rules,
            create_task_rule,
            delete_task_rule,
            get_current_task,
            get_task_summary
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
