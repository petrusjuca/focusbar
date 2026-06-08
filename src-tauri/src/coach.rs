//! Coach ao vivo: roda dentro do loop do sampler. Detecta procrastinação
//! prolongada, fragmentação (muita troca de janela) e "travado" na mesma janela,
//! e dispara notificações com cooldown. Também dispara o resumo de fim de dia.

use crate::category::categorize;
use crate::insights;
use chrono::{Local, NaiveDate, TimeZone, Timelike};
use rusqlite::Connection;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use tauri_plugin_notification::NotificationExt;
use tauri::AppHandle;

const PROCRAST_THRESHOLD: i64 = 20 * 60; // 20min seguidos
const PROCRAST_COOLDOWN: i64 = 15 * 60;
const FRAG_WINDOW: i64 = 10 * 60; // janela de 10min (só pra poda; alerta desligado)
const STUCK_THRESHOLD: i64 = 50 * 60; // 50min na mesma janela
const EOD_HOUR: u32 = 18; // resumo às 18h

pub struct Coach {
    procrast_start: Option<i64>,
    switches: VecDeque<i64>,
    last_alert: HashMap<&'static str, i64>,
    stuck_alerted_for: Option<i64>,
    last_eod_day: Option<String>,
}

impl Coach {
    pub fn new() -> Self {
        Self {
            procrast_start: None,
            switches: VecDeque::new(),
            last_alert: HashMap::new(),
            stuck_alerted_for: None,
            last_eod_day: None,
        }
    }

    /// Registra uma troca de janela (para medir fragmentação).
    pub fn note_switch(&mut self, now: i64) {
        self.switches.push_back(now);
    }

    fn cooldown_ok(&self, key: &'static str, now: i64, cd: i64) -> bool {
        self.last_alert.get(key).map_or(true, |t| now - t >= cd)
    }

    fn fire(&mut self, app: &AppHandle, key: &'static str, now: i64, body: &str) {
        let _ = app.notification().builder().title("focusbar").body(body).show();
        self.last_alert.insert(key, now);
    }

    /// Chamado a cada tick do sampler. `current` = (app, título, início) ou None (ocioso).
    pub fn tick(
        &mut self,
        now: i64,
        current: Option<(&str, &str, i64)>,
        app: &AppHandle,
        db: &Arc<Mutex<Connection>>,
    ) {
        // Limpa trocas fora da janela de fragmentação.
        while let Some(&front) = self.switches.front() {
            if now - front > FRAG_WINDOW {
                self.switches.pop_front();
            } else {
                break;
            }
        }

        if let Some((app_name, title, start)) = current {
            // Categoria efetiva: override manual do usuário, ou regra.
            let cat_owned = db
                .lock()
                .ok()
                .and_then(|c| crate::db::app_category(&c, app_name).ok())
                .flatten()
                .unwrap_or_else(|| categorize(app_name, title).to_string());
            let cat = cat_owned.as_str();

            // Procrastinação prolongada.
            if cat == "Procrastinação" {
                let begin = *self.procrast_start.get_or_insert(start);
                let dur = now - begin;
                if dur >= PROCRAST_THRESHOLD && self.cooldown_ok("procrast", now, PROCRAST_COOLDOWN) {
                    self.fire(
                        app,
                        "procrast",
                        now,
                        &format!("Você está há {}min em procrastinação 👀", dur / 60),
                    );
                }
            } else {
                self.procrast_start = None;
            }

            // Travado na mesma janela.
            let stuck = now - start;
            if stuck >= STUCK_THRESHOLD && self.stuck_alerted_for != Some(start) {
                self.fire(
                    app,
                    "stuck",
                    now,
                    &format!("{}min na mesma janela — que tal uma pausa?", stuck / 60),
                );
                self.stuck_alerted_for = Some(start);
            }
        } else {
            self.procrast_start = None;
        }

        // (Alerta de fragmentação removido — incomodava quem usa muitas janelas.
        //  Fragmentação continua disponível só como insight do dia, não como alerta.)

        // Resumo de fim de dia (uma vez por dia, a partir das EOD_HOUR).
        let now_local = Local::now();
        let day = now_local.date_naive().format("%Y-%m-%d").to_string();
        if now_local.hour() >= EOD_HOUR && self.last_eod_day.as_deref() != Some(day.as_str()) {
            self.last_eod_day = Some(day.clone());
            if let Some(body) = build_eod(db, now_local.date_naive()) {
                self.fire(app, "eod", now, &body);
            }
        }
    }
}

/// Monta o corpo do resumo de fim de dia.
fn build_eod(db: &Arc<Mutex<Connection>>, date: NaiveDate) -> Option<String> {
    let start = Local
        .from_local_datetime(&date.and_hms_opt(0, 0, 0)?)
        .earliest()?
        .timestamp();
    let end = start + 86_400;
    let conn = db.lock().ok()?;
    let tips = insights::day_insights(&conn, start, end).ok()?;
    let first = tips.first()?;
    Some(format!("Resumo do dia: {}", first.text))
}
