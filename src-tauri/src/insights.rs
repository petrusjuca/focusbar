//! Geração de insights/dicas do dia por regras (sem IA).
//! Usado pelo command get_day_insights e pelo resumo de fim de dia do coach.

use crate::category::effective;
use crate::db;
use crate::models::Insight;
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

    // NÃO avisamos sobre fragmentação / "trocar muito de aba". Pro cérebro
    // ADHD do Petrus, pular entre janelas é o jeito NORMAL de trabalhar — não é
    // falha a cobrar. (O coach ao vivo já tinha tirado esse alerta; aqui idem.)
    // O número de trocas continua visível, sem julgamento, no painel de Dados.

    if out.is_empty() {
        out.push(Insight {
            kind: "good".into(),
            text: format!("Dia equilibrado: {} de uso registrado.", fmt_min(total)),
        });
    }

    Ok(out)
}
