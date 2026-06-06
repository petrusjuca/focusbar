//! Sampler em background: amostra a janela em foco a cada POLL_SECS, detecta
//! troca, descarta trocas muito curtas (debounce) e grava UMA linha por sessão.
//! Enriquece o título com a URL do navegador (contexto pro assistente).
//!
//! Ociosidade (AFK): sem input por IDLE_THRESHOLD_SECS fecha a sessão no último
//! input e marca `was_idle_trimmed`.

use crate::capture::{browser, ActiveWinProvider, WindowProvider};
use crate::coach::Coach;
use crate::db;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};

const POLL_SECS: u64 = 1;
const MIN_SESSION_SECS: i64 = 2;
const IDLE_THRESHOLD_SECS: i64 = 120;

struct OpenSession {
    app_id: i64,
    app_name: String,
    raw_title: String,    // título cru, usado pra detectar troca
    stored_title: String, // título + URL, gravado e mandado pro assistente
    start_ts: i64,
}

fn now_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn idle_secs() -> i64 {
    user_idle::UserIdle::get_time()
        .map(|t| t.as_seconds() as i64)
        .unwrap_or(0)
}

/// Junta título + URL do navegador (busca AppleScript só aqui, na troca).
fn enrich_title(app_name: &str, title: &str) -> String {
    match browser::browser_url(app_name) {
        Some(url) if !url.is_empty() => format!("{} — {}", title, url),
        _ => title.to_string(),
    }
}

pub fn spawn(app: AppHandle, db: Arc<Mutex<Connection>>) {
    thread::spawn(move || {
        let provider = ActiveWinProvider;
        let mut current: Option<OpenSession> = None;
        let mut coach = Coach::new();

        loop {
            let now = now_ts();
            let idle = idle_secs();
            let is_idle = idle >= IDLE_THRESHOLD_SECS;

            let win = if is_idle { None } else { provider.current() };

            let new_key = win.as_ref().map(|w| (w.app_name.as_str(), w.title.as_str()));
            let cur_key = current
                .as_ref()
                .map(|c| (c.app_name.as_str(), c.raw_title.as_str()));

            if new_key != cur_key {
                // Fecha a sessão anterior.
                if let Some(prev) = current.take() {
                    let end = if is_idle {
                        (now - idle).max(prev.start_ts)
                    } else {
                        now
                    };
                    if end - prev.start_ts >= MIN_SESSION_SECS {
                        if let Ok(conn) = db.lock() {
                            let _ = db::insert_session(
                                &conn,
                                prev.app_id,
                                &prev.stored_title,
                                prev.start_ts,
                                end,
                                is_idle,
                            );
                        }
                    }
                }

                // Abre a nova (enriquece com URL aqui — uma vez por troca).
                if let Some(w) = win.as_ref() {
                    let bundle = w.app_bundle_id.clone().unwrap_or_default();
                    let app_id = db
                        .lock()
                        .ok()
                        .and_then(|conn| db::get_or_create_app(&conn, &w.app_name, &bundle).ok());
                    if let Some(app_id) = app_id {
                        let stored_title = enrich_title(&w.app_name, &w.title);
                        current = Some(OpenSession {
                            app_id,
                            app_name: w.app_name.clone(),
                            raw_title: w.title.clone(),
                            stored_title,
                            start_ts: now,
                        });
                    }
                }

                let _ = app.emit("focus-changed", &win);
                coach.note_switch(now);
            }

            // Coach ao vivo (usa o título enriquecido — pega youtube.com etc.).
            let cur_tuple = current
                .as_ref()
                .map(|c| (c.app_name.as_str(), c.stored_title.as_str(), c.start_ts));
            coach.tick(now, cur_tuple, &app, &db);

            thread::sleep(Duration::from_secs(POLL_SECS));
        }
    });
}
