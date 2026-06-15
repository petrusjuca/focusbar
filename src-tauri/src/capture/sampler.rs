//! Sampler em background: amostra a janela em foco a cada POLL_SECS, detecta
//! troca, descarta trocas curtas (debounce) e grava UMA linha por sessão.
//!
//! - Navegadores: o "app" vira o SITE (WhatsApp, Miro, YouTube) via a URL, em
//!   vez de "Chrome"/"Opera" — assim o "tempo por app" fica útil.
//! - Pause: quando `paused`, não conta nada (sem ser procrastinação).
//! - Idle: sem input por IDLE_THRESHOLD_SECS fecha no último input.

use crate::capture::{browser, ActiveWinProvider, WindowProvider};
use crate::coach::Coach;
use crate::db;
use rusqlite::Connection;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};

const POLL_SECS: u64 = 1;
const MIN_SESSION_SECS: i64 = 2;
const IDLE_THRESHOLD_SECS: i64 = 120;

struct OpenSession {
    app_id: i64,
    raw_app: String,      // ex.: "Opera GX" — usado pra detectar troca
    raw_title: String,    // título cru — usado pra detectar troca
    label_app: String,    // ex.: "WhatsApp" (site) — usado no banco/coach
    stored_title: String, // título + URL — gravado e mandado pro assistente
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

/// Apps que NÃO são foco real (tela de bloqueio, servidor de janelas) — ruído.
fn is_system_noise(app: &str) -> bool {
    let a = app.to_lowercase();
    a == "loginwindow" || a == "windowserver" || a == "screensaverengine"
}

pub fn spawn(app: AppHandle, db: Arc<Mutex<Connection>>, paused: Arc<AtomicBool>) {
    thread::spawn(move || {
        let provider = ActiveWinProvider;
        let mut current: Option<OpenSession> = None;
        let mut coach = Coach::new();

        loop {
            let now = now_ts();
            let idle = idle_secs();
            let is_idle = idle >= IDLE_THRESHOLD_SECS;
            let is_paused = paused.load(Ordering::Relaxed);

            // Pausado ou ocioso = sem foco (mas pausado NÃO é idle-trimmed).
            // Ignora "loginwindow" e cia. — tela de bloqueio não é foco real.
            let win = if is_paused || is_idle {
                None
            } else {
                provider.current().filter(|w| !is_system_noise(&w.app_name))
            };

            let new_key = win.as_ref().map(|w| (w.app_name.as_str(), w.title.as_str()));
            let cur_key = current
                .as_ref()
                .map(|c| (c.raw_app.as_str(), c.raw_title.as_str()));

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

                // Abre a nova (resolve site + URL aqui, uma vez por troca).
                if let Some(w) = win.as_ref() {
                    let url = browser::browser_url(&w.app_name);
                    let (label_app, stored_title, bundle) = match &url {
                        Some(u) => (
                            browser::site_name(u).unwrap_or_else(|| w.app_name.clone()),
                            // URL limpa: sem query/fragment (onde vivem tokens/PII).
                            format!("{} — {}", w.title, browser::clean_url(u)),
                            String::new(), // site agrupa independente do navegador
                        ),
                        None => (
                            w.app_name.clone(),
                            w.title.clone(),
                            w.app_bundle_id.clone().unwrap_or_default(),
                        ),
                    };
                    let app_id = db
                        .lock()
                        .ok()
                        .and_then(|conn| db::get_or_create_app(&conn, &label_app, &bundle).ok());
                    if let Some(app_id) = app_id {
                        current = Some(OpenSession {
                            app_id,
                            raw_app: w.app_name.clone(),
                            raw_title: w.title.clone(),
                            label_app,
                            stored_title,
                            start_ts: now,
                        });
                    }
                }

                let _ = app.emit("focus-changed", &win);
                coach.note_switch(now);
            }

            // Coach ao vivo (usa o nome do site + título com URL).
            let cur_tuple = current
                .as_ref()
                .map(|c| (c.label_app.as_str(), c.stored_title.as_str(), c.start_ts));
            coach.tick(now, cur_tuple, &app, &db);

            thread::sleep(Duration::from_secs(POLL_SECS));
        }
    });
}
