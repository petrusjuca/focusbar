// Segurança de concorrência: nunca segurar o lock do DB (std::sync::Mutex)
// atravessando um `.await` — travaria o sampler. Enforçado sob `cargo clippy`.
#![deny(clippy::await_holding_lock)]

mod ai;
mod capture;
// Públicos para o binário `mcp` (servidor MCP local) reusar as queries e tipos.
pub mod category;
mod coach;
mod commands;
pub mod db;
mod insights;
pub mod models;
mod redact;
mod reminders;
mod state;

use capture::{ActiveWinProvider, WindowProvider};
use commands::assistant::{
    ai_available, ai_day_digest, ai_day_review, ai_pull_model, ai_status, start_ollama,
};
use commands::config::{
    categorize_pending, get_focus_time, get_mcp_info, get_ocr_enabled, get_setting,
    log_focus_time, log_pomodoro, run_ocr_selftest, set_ocr_enabled, set_setting,
};
use commands::focus::{check_focus, get_focus, set_focus, set_focus_judgment};
use commands::notes::{add_note, delete_note, list_notes};
use commands::todos::{
    add_todo, complete_todo_by_text, delete_todo, list_todos, toggle_todo,
};
use commands::permissions::{
    check_accessibility, check_screen_recording, request_accessibility, request_screen_recording,
};
use commands::reminders::{
    create_reminder, delete_reminder, list_reminders, set_reminder_enabled,
};
use commands::sessions::{delete_app_sessions, delete_session, get_recent_sessions};
use commands::summaries::{
    get_category_summary, get_daily_summary, get_day_insights, get_day_markers, get_day_sessions,
    get_metrics, get_weekly_summary, set_app_category, set_block_category,
};
use models::ActiveWindow;
use state::AppState;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, State, WindowEvent};
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

fn now_ts() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Pausa/retoma o rastreamento (pausado = não conta nada). Grava um marcador de
/// intervalo "paused" pra timeline mostrar uma faixa rotulada (nunca vazio), e
/// emite `paused-changed` pro banner/bandeja virarem na hora.
#[tauri::command]
fn set_paused(app: AppHandle, state: State<AppState>, paused: bool) {
    let prev = state.paused.swap(paused, Ordering::Relaxed);
    if prev == paused {
        return;
    }
    let now = now_ts();
    if let Ok(conn) = state.db.lock() {
        if paused {
            let _ = db::open_marker(&conn, "paused", now);
        } else {
            let _ = db::close_marker(&conn, "paused", now);
        }
    }
    let _ = app.emit("paused-changed", paused);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
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
            // Cura marcadores deixados ABERTOS por um crash/forçar-saída (senão a
            // timeline mostraria "pausado/ausente" pra sempre).
            let _ = db::close_marker(&conn, "paused", now_ts());
            let _ = db::close_marker(&conn, "away", now_ts());
            let _ = db::close_marker(&conn, "away_uncertain", now_ts());
            let db = Arc::new(Mutex::new(conn));
            let paused = Arc::new(AtomicBool::new(false));

            app.manage(AppState {
                db: db.clone(),
                paused: paused.clone(),
                last_check: std::sync::Mutex::new(None),
            });

            // Tenta ligar o Ollama sozinho (best-effort; se não tiver, ignora).
            commands::assistant::try_start_ollama();

            // Sobe o sampler de foco e o scheduler de lembretes em background.
            capture::sampler::spawn(app.handle().clone(), db.clone(), paused);

            // Auto-teste de OCR: 8s após abrir, OCRa a PRÓPRIA janela uma vez e
            // grava o resultado em settings (ocr_selftest). Diz a VERDADE sobre se o
            // OCR captura a tela nesta máquina — sem depender de foco nem do preflight
            // de permissão (que no macOS mente). Guarda também o que o preflight diz,
            // pra flagrar o falso-negativo.
            {
                let db_t = db.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_secs(8));
                    let pid = std::process::id() as i32;
                    let own = tokio::runtime::Builder::new_multi_thread()
                        .worker_threads(1)
                        .enable_all()
                        .build()
                        .ok()
                        .and_then(|rt| rt.block_on(capture::screen::ocr_window_by_pid(pid)));
                    let val = match own {
                        Some(t) => format!("ok:{}", t.chars().count()),
                        None => "falhou".to_string(),
                    };
                    if let Ok(c) = db_t.lock() {
                        let _ = db::set_setting(&c, "ocr_selftest", &val);
                    }
                });
            }

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
                        let prev = st.paused.load(Ordering::Relaxed);
                        let next = !prev;
                        st.paused.store(next, Ordering::Relaxed);
                        // Mesmo marcador/evento do set_paused (senão a pausa pela
                        // bandeja fica invisível na timeline e no banner).
                        let now = now_ts();
                        if let Ok(conn) = st.db.lock() {
                            if next {
                                let _ = db::open_marker(&conn, "paused", now);
                            } else {
                                let _ = db::close_marker(&conn, "paused", now);
                            }
                        }
                        let _ = app.emit("paused-changed", next);
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
            check_screen_recording,
            request_screen_recording,
            get_recent_sessions,
            delete_session,
            delete_app_sessions,
            get_daily_summary,
            get_day_sessions,
            get_day_markers,
            get_metrics,
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
            ai_available,
            ai_day_review,
            ai_day_digest,
            ai_status,
            ai_pull_model,
            start_ollama,
            add_note,
            list_notes,
            delete_note,
            set_app_category,
            set_block_category,
            add_todo,
            list_todos,
            toggle_todo,
            delete_todo,
            complete_todo_by_text,
            set_focus,
            get_focus,
            check_focus,
            set_focus_judgment,
            get_ocr_enabled,
            set_ocr_enabled,
            run_ocr_selftest,
            get_mcp_info,
            get_setting,
            set_setting,
            categorize_pending,
            log_focus_time,
            log_pomodoro,
            get_focus_time
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
