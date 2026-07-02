//! Captura da janela em foco → eventos BRUTOS.
//!
//! Fase 1: detecção por poll de 1s (grava `foreground` SÓ quando muda — semântica
//! de evento) + `heartbeat` a cada 15s confirmando que segue a mesma janela.
//! Fase seguinte pluga os hooks nativos (SetWinEventHook no Win / NSWorkspace no
//! Mac) como fonte primária `source=event`, mantendo este poll como fallback —
//! o schema já carrega o campo `source` pra isso.

use crate::db::{self, NewEvent};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use std::time::Duration;

const POLL_MS: u64 = 1_000;
const HEARTBEAT_MS: i64 = 15_000;

fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub fn spawn(db: Arc<Mutex<Connection>>) {
    std::thread::spawn(move || {
        let mut last: Option<(String, String, i64)> = None; // (app, title, pid)
        let mut last_beat = 0i64;

        loop {
            let now = now_ms();
            if let Ok(w) = active_win_pos_rs::get_active_window() {
                let app = w.app_name.clone();
                let title = w.title.clone();
                let pid = w.process_id as i64;

                let changed = match &last {
                    Some((a, _t, p)) => *a != app || *p != pid, // título não é identidade
                    None => true,
                };
                let title_changed = matches!(&last, Some((_, t, _)) if *t != title);

                if changed {
                    if let Ok(c) = db.lock() {
                        let _ = db::append(&c, &NewEvent {
                            ts_ms: now, kind: "foreground", app: &app, title: &title,
                            url: None, tab_id: None, pid: Some(pid), source: "poll", payload: None,
                        });
                    }
                    last_beat = now;
                } else if now - last_beat >= HEARTBEAT_MS || title_changed {
                    // heartbeat também quando o título muda (metadado fresco pro derivador)
                    if let Ok(c) = db.lock() {
                        let _ = db::append(&c, &NewEvent {
                            ts_ms: now, kind: "heartbeat", app: &app, title: &title,
                            url: None, tab_id: None, pid: Some(pid), source: "poll", payload: None,
                        });
                    }
                    last_beat = now;
                }
                last = Some((app, title, pid));
            }
            std::thread::sleep(Duration::from_millis(POLL_MS));
        }
    });
}
