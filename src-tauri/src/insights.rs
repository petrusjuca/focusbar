//! Geração de insights/dicas do dia por regras (sem IA).
//! Usado pelo command get_day_insights e pelo resumo de fim de dia do coach.

use crate::category::effective;
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
    let overrides = db::category_overrides(conn).unwrap_or_default();
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
    let mut cats: HashMap<String, i64> = HashMap::new();
    for s in &sessions {
        *cats
            .entry(effective(&overrides, &s.app_name, &s.title))
            .or_insert(0) += s.duration_secs;
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

    // Maior ladrão de tempo: app/site que mais consumiu tempo em Procrastinação.
    // (Itera sessões + effective() — NÃO usa app_totals, que agrupa por categoria salva.)
    let mut by_app: HashMap<String, i64> = HashMap::new();
    for s in &sessions {
        if effective(&overrides, &s.app_name, &s.title) == "Procrastinação" {
            *by_app.entry(s.app_name.clone()).or_insert(0) += s.duration_secs;
        }
    }
    if let Some((app, &secs)) = by_app.iter().max_by_key(|(_, &v)| v) {
        if secs >= 600 {
            out.push(Insight {
                kind: "warn".into(),
                text: format!(
                    "Maior ladrão de tempo hoje: {} ({}). Considere fechar ou bloquear.",
                    app,
                    fmt_min(secs)
                ),
            });
        }
    }

    // Melhor hora de foco (tempo não-procrastinação por hora).
    let mut by_hour: HashMap<u32, i64> = HashMap::new();
    for s in &sessions {
        if effective(&overrides, &s.app_name, &s.title) != "Procrastinação" {
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

    // Maior bloco de foco CONTÍNUO (sessões não-procrastinação encadeadas, gap < 2min).
    // Diferente do "pico" (uma janela só): trocar VSCode→terminal→Notion ainda é foco.
    let mut best_run = 0i64;
    let mut run_secs = 0i64;
    let mut prev_end: Option<i64> = None;
    for s in &sessions {
        let cat = effective(&overrides, &s.app_name, &s.title);
        let gap = prev_end.map_or(0, |pe| (s.start_ts - pe).max(0));
        if cat == "Procrastinação" || gap > 120 {
            run_secs = 0;
        } else {
            run_secs += s.duration_secs;
            best_run = best_run.max(run_secs);
        }
        prev_end = Some(s.start_ts + s.duration_secs);
    }
    if best_run >= 1500 {
        out.push(Insight {
            kind: "focus".into(),
            text: format!(
                "Seu maior bloco de foco contínuo foi {}. Isso é ouro — replique.",
                fmt_min(best_run)
            ),
        });
    }

    // Comparação com a média dos últimos 6 dias (sem o dia atual).
    let mut prev_sum = 0i64;
    for i in 1..=6 {
        prev_sum += db::total_in_range(conn, start - i * 86_400, start - (i - 1) * 86_400)?;
    }
    let mean = prev_sum / 6;
    if mean >= 600 {
        let pct = ((total - mean) as f64 / mean as f64 * 100.0).round() as i64;
        if total >= (mean as f64 * 1.2) as i64 {
            out.push(Insight {
                kind: "good".into(),
                text: format!(
                    "Hoje você usou o tempo {}% acima da sua média dos últimos 6 dias.",
                    pct
                ),
            });
        } else if total <= (mean as f64 * 0.7) as i64 {
            out.push(Insight {
                kind: "info".into(),
                text: format!(
                    "Hoje foi {}% abaixo da sua média dos últimos 6 dias.",
                    pct.abs()
                ),
            });
        }
    }

    // Trocas de tarefa/contexto (só conta se houver task_rules cadastradas).
    let rules = db::tasks::list(conn)?;
    if !rules.is_empty() {
        let mut switches = 0i64;
        let mut prev: Option<String> = None;
        for s in &sessions {
            if let Some(task) = db::tasks::match_task(&rules, &s.app_name, &s.title) {
                if let Some(p) = &prev {
                    if p.as_str() != task {
                        switches += 1;
                    }
                }
                prev = Some(task.to_string());
            }
        }
        if switches >= 8 {
            out.push(Insight {
                kind: "warn".into(),
                text: format!(
                    "Você pulou entre tarefas {} vezes hoje. Tente agrupar por contexto.",
                    switches
                ),
            });
        }
    }

    // Intenção declarada vs realidade (por regra, offline — não depende de IA).
    let day = Local
        .timestamp_opt(start, 0)
        .single()
        .map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_default();
    let intentions = db::notes::intentions_for_day(conn, &day).unwrap_or_default();
    const STOP: &[&str] = &[
        "para", "pelo", "pela", "hoje", "fazer", "mais", "esse", "essa", "isso", "dele",
        "dela", "com", "uma", "que", "dos", "das", "por", "sobre", "meu", "minha", "the",
    ];
    for intent in &intentions {
        let toks: Vec<String> = intent
            .to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w: &&str| w.len() > 3 && !STOP.contains(w))
            .map(|w| w.to_string())
            .collect();
        if toks.is_empty() {
            continue;
        }
        let mut matched = 0i64;
        for s in &sessions {
            let hay = format!("{} {}", s.app_name, s.title).to_lowercase();
            if toks.iter().any(|t| hay.contains(t.as_str())) {
                matched += s.duration_secs;
            }
        }
        if matched < 60 {
            out.push(Insight {
                kind: "warn".into(),
                text: format!("A intenção \"{}\" ficou de fora hoje.", intent),
            });
        } else if matched >= 600 {
            out.push(Insight {
                kind: "good".into(),
                text: format!(
                    "Você trabalhou na intenção \"{}\" ({}).",
                    intent,
                    fmt_min(matched)
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
