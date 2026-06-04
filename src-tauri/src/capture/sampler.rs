//! Sampler em background: amostra a janela em foco a cada POLL_SECS, detecta
//! troca, descarta trocas muito curtas (debounce) e grava UMA linha por sessão
//! de foco. Emite o evento `focus-changed` para o frontend atualizar ao vivo.
//!
//! Ociosidade (AFK): se não houver input por IDLE_THRESHOLD_SECS, a sessão
//! aberta é fechada no momento do ÚLTIMO input (não no momento atual), marcada
//! como `was_idle_trimmed`, e o tempo ocioso não é contado como foco.

use crate::capture::{ActiveWinProvider, WindowProvider};
use crate::coach::Coach;
use crate::db;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};

const POLL_SECS: u64 = 1;
/// Sessões mais curtas que isso são descartadas (alt-tab acidental).
const MIN_SESSION_SECS: i64 = 2;
/// Sem input por mais que isso = ocioso (AFK).
const IDLE_THRESHOLD_SECS: i64 = 120;

struct OpenSession {
    app_id: i64,
    app_name: String,
    title: String,
    start_ts: i64,
}

fn now_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Segundos desde o último input do usuário (teclado/mouse). 0 se indisponível.
fn idle_secs() -> i64 {
    user_idle::UserIdle::get_time()
        .map(|t| t.as_seconds() as i64)
        .unwrap_or(0)
}

/// Sobe a thread do sampler. Roda enquanto o processo viver.
pub fn spawn(app: AppHandle, db: Arc<Mutex<Connection>>) {
    thread::spawn(move || {
        let provider = ActiveWinProvider;
        let mut current: Option<OpenSession> = None;
        let mut coach = Coach::new();

        loop {
            let now = now_ts();
            let idle = idle_secs();
            let is_idle = idle >= IDLE_THRESHOLD_SECS;

            // Ocioso conta como "sem foco" → janela vira None.
            let win = if is_idle { None } else { provider.current() };

            let new_key = win.as_ref().map(|w| (w.app_name.as_str(), w.title.as_str()));
            let cur_key = current
                .as_ref()
                .map(|c| (c.app_name.as_str(), c.title.as_str()));

            if new_key != cur_key {
                // Fecha a sessão anterior (se durou o suficiente).
                if let Some(prev) = current.take() {
                    // Se entrou em ociosidade, fecha no último input; senão, agora.
                    let end = if is_idle {
                        (now - idle).max(prev.start_ts)
                    } else {
                        now
                    };
                    let dur = end - prev.start_ts;
                    if dur >= MIN_SESSION_SECS {
                        if let Ok(conn) = db.lock() {
                            let _ = db::insert_session(
                                &conn,
                                prev.app_id,
                                &prev.title,
                                prev.start_ts,
                                end,
                                is_idle,
                            );
                        }
                    }
                }

                // Abre a nova sessão (só se houver janela e não estiver ocioso).
                if let Some(w) = win.as_ref() {
                    let bundle = w.app_bundle_id.clone().unwrap_or_default();
                    let app_id = db
                        .lock()
                        .ok()
                        .and_then(|conn| db::get_or_create_app(&conn, &w.app_name, &bundle).ok());
                    if let Some(app_id) = app_id {
                        current = Some(OpenSession {
                            app_id,
                            app_name: w.app_name.clone(),
                            title: w.title.clone(),
                            start_ts: now,
                        });
                    }
                }

                // Avisa o frontend e o coach (troca conta para fragmentação).
                let _ = app.emit("focus-changed", &win);
                coach.note_switch(now);
            }

            // Coach ao vivo (procrastinação, fragmentação, travado, fim de dia).
            let cur_tuple = current
                .as_ref()
                .map(|c| (c.app_name.as_str(), c.title.as_str(), c.start_ts));
            coach.tick(now, cur_tuple, &app, &db);

            thread::sleep(Duration::from_secs(POLL_SECS));
        }
    });
}
