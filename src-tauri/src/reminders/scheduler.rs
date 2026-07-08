//! Scheduler de lembretes: a cada TICK_SECS, verifica os lembretes habilitados
//! e dispara notificações nativas para os que estão "vencidos".
//!
//! - "once": dispara quando now >= fire_at (e desabilita em seguida).
//! - "recurring": dispara quando now - COALESCE(last_fired, created_at) >= interval.

use crate::db;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};
use tauri_plugin_notification::NotificationExt;

const TICK_SECS: u64 = 15;

fn now_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub fn spawn(app: AppHandle, db: Arc<Mutex<Connection>>) {
    thread::spawn(move || loop {
        let now = now_ts();

        // Coleta os que devem disparar (segurando o lock o mínimo possível).
        let mut to_fire: Vec<(i64, String, bool)> = Vec::new();
        if let Ok(conn) = db.lock() {
            if let Ok(list) = db::reminders::enabled(&conn) {
                for r in list {
                    // "Chega por hoje": silenciado até a meia-noite — pula.
                    if r.snoozed_until.is_some_and(|s| now < s) {
                        continue;
                    }
                    let due = match r.kind.as_str() {
                        "once" => r.last_fired_ts.is_none() && r.fire_at.map_or(false, |t| now >= t),
                        "recurring" => {
                            let base = r.last_fired_ts.unwrap_or(r.created_at);
                            r.interval_secs.map_or(false, |iv| now - base >= iv)
                        }
                        _ => false,
                    };
                    if due {
                        let disable = r.kind == "once";
                        to_fire.push((r.id, r.text, disable));
                    }
                }
            }
        }

        // Dispara fora do lock.
        for (id, text, disable) in to_fire {
            let _ = app
                .notification()
                .builder()
                .title("focusbar")
                .body(&text)
                .show();
            if let Ok(conn) = db.lock() {
                let _ = db::reminders::mark_fired(&conn, id, now, disable);
            }
            // Payload com id: a UI mostra o aviso que FICA na tela, com
            // "feito" / "chega por hoje" (silencia até amanhã) — lembretes v2.
            let _ = app.emit("reminder-fired", serde_json::json!({ "id": id, "text": text }));
        }

        thread::sleep(Duration::from_secs(TICK_SECS));
    });
}
