//! Geração de insights/dicas do dia por regras (sem IA).
//! Usado pelo command get_day_insights e pelo resumo de fim de dia do coach.

use crate::category::categorize;
use crate::db;
use crate::models::Insight;
use chrono::{Local, TimeZone, Timelike};
use rusqlite::Connection;
use std::collections::HashMap;

fn fmt_min(secs: i64) -> String {
    let m = secs / 60;
    if m < 60 {
        format!("{}min", m)
    } else {
        format!("{}h{:02}", m / 60, m % 60)
    }
}

/// Hora local (0-23) de um timestamp epoch.
fn hour_of(ts: i64) -> u32 {
    Local.timestamp_opt(ts, 0).single().map(|d| d.hour()).unwrap_or(0)
}

pub fn day_insights(conn: &Connection, start: i64, end: i64) -> rusqlite::Result<Vec<Insight>> {
    let sessions = db::sessions_in_range(conn, start, end)?;
    let mut out = Vec::new();

    let total: i64 = sessions.iter().map(|s| s.duration_secs).sum();
    if total < 60 {
        out.push(Insight {
            kind: "info".into(),
            text: "Ainda não há dados suficientes hoje. Continue usando o computador normalmente.".into(),
        });
        return Ok(out);
    }

    // Totais por categoria.
    let mut cats: HashMap<&str, i64> = HashMap::new();
    for s in &sessions {
        *cats.entry(categorize(&s.app_name, &s.title)).or_insert(0) += s.duration_secs;
    }
    let procrast = *cats.get("Procrastinação").unwrap_or(&0);
    let procrast_pct = (procrast as f64 / total as f64 * 100.0).round() as i64;

    // Procrastinação alta?
    if procrast_pct >= 40 {
        out.push(Insight {
            kind: "warn".into(),
            text: format!(
                "Procrastinação alta hoje: {}% do tempo ({}). Tente blocos de foco de 25min.",
                procrast_pct,
                fmt_min(procrast)
            ),
        });
    } else if procrast_pct <= 15 {
        out.push(Insight {
            kind: "good".into(),
            text: format!("Boa! Só {}% em procrastinação hoje.", procrast_pct),
        });
    }

    // Melhor hora de foco (tempo não-procrastinação por hora).
    let mut by_hour: HashMap<u32, i64> = HashMap::new();
    for s in &sessions {
        if categorize(&s.app_name, &s.title) != "Procrastinação" {
            *by_hour.entry(hour_of(s.start_ts)).or_insert(0) += s.duration_secs;
        }
    }
    if let Some((&h, &secs)) = by_hour.iter().max_by_key(|(_, &v)| v) {
        if secs >= 120 {
            out.push(Insight {
                kind: "focus".into(),
                text: format!(
                    "Seu melhor foco foi por volta das {}h ({}). Agende tarefas difíceis nesse horário.",
                    h,
                    fmt_min(secs)
                ),
            });
        }
    }

    // Fragmentação: muitas sessões curtas.
    let count = sessions.len() as i64;
    let avg = total / count.max(1);
    if count >= 20 && avg < 90 {
        out.push(Insight {
            kind: "warn".into(),
            text: format!(
                "Muitas trocas de janela ({} sessões, média de {}). Tente fechar distrações e focar numa coisa por vez.",
                count,
                fmt_min(avg.max(1))
            ),
        });
    }

    // Pico de foco.
    if let Some(longest) = sessions.iter().max_by_key(|s| s.duration_secs) {
        if longest.duration_secs >= 300 {
            out.push(Insight {
                kind: "focus".into(),
                text: format!(
                    "Seu pico de foco foi {} em {}. Replique esse tipo de bloco.",
                    fmt_min(longest.duration_secs),
                    longest.app_name
                ),
            });
        }
    }

    if out.is_empty() {
        out.push(Insight {
            kind: "good".into(),
            text: format!("Dia equilibrado: {} de uso registrado.", fmt_min(total)),
        });
    }

    Ok(out)
}
