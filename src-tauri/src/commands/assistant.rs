//! O "assistente" pós-Ollama (decisão D2, executada em 02.07): o app NÃO roda
//! LLM local. Ele PREPARA o dia (dados limpos + agregados) e quem pensa é o
//! Claude — via MCP (fim do dia) ou via texto copiado (ai_day_digest).

use crate::category::effective;
use crate::commands::summaries::bounds_for_date_pub;
use crate::db;
use crate::redact;
use crate::state::AppState;
use chrono::{Datelike, Local, NaiveDate, TimeZone};
use std::collections::HashMap;
use tauri::State;

fn hhmm(ts: i64) -> String {
    Local
        .timestamp_opt(ts, 0)
        .single()
        .map(|d| d.format("%H:%M").to_string())
        .unwrap_or_default()
}

fn day_label(day: Option<&str>) -> String {
    day.map(|s| s.to_string())
        .unwrap_or_else(|| Local::now().date_naive().format("%Y-%m-%d").to_string())
}

/// Bloco com as intenções declaradas no dia (pra IA comparar com a execução).
fn intentions_block(state: &State<'_, AppState>, day: Option<&str>) -> String {
    let label = day_label(day);
    let ints = state
        .db
        .lock()
        .ok()
        .and_then(|c| db::notes::intentions_for_day(&c, &label).ok())
        .unwrap_or_default();
    if ints.is_empty() {
        String::new()
    } else {
        format!("Minhas intenções declaradas hoje: {}.\n\n", ints.join("; "))
    }
}

/// Dados do dia já limpos (porteiro: redação + zonas de exclusão) + agregados,
/// pra montar um resumo com contexto. Síncrono — não segura lock em await.
struct DayData {
    lines: Vec<String>,
    total_secs: i64,
    total_count: usize,
    top_cats: Vec<(String, i64)>,
}

fn fmt_hm(secs: i64) -> String {
    let m = secs / 60;
    if m < 60 {
        format!("{}min", m)
    } else {
        format!("{}h{:02}", m / 60, m % 60)
    }
}

fn weekday_pt(date: NaiveDate) -> &'static str {
    use chrono::Weekday::*;
    match date.weekday() {
        Mon => "segunda",
        Tue => "terça",
        Wed => "quarta",
        Thu => "quinta",
        Fri => "sexta",
        Sat => "sábado",
        Sun => "domingo",
    }
}

/// Mensagem de "sem dados" que respeita o dia escolhido (não diz "hoje" pra dia passado).
fn empty_msg(day: Option<&str>) -> String {
    match day {
        Some(d) => format!("Sem dados suficientes em {} pra analisar.", d),
        None => "Sem dados suficientes hoje pra analisar.".into(),
    }
}

fn collect_day(state: &State<'_, AppState>, day: Option<&str>) -> Result<DayData, String> {
    let (start, end) = bounds_for_date_pub(day);
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let sessions = db::sessions_in_range(&conn, start, end).map_err(|e| e.to_string())?;
    let overrides = db::category_overrides(&conn).unwrap_or_default();
    let kept: Vec<_> = sessions
        .into_iter()
        .filter(|s| s.duration_secs >= 20)
        .filter(|s| !redact::is_excluded(&s.app_name, &s.title))
        .collect();

    let mut total_secs = 0i64;
    let mut cats: HashMap<String, i64> = HashMap::new();
    let mut lines: Vec<String> = Vec::with_capacity(kept.len());
    for s in &kept {
        let title = redact::redact(&s.title);
        let cat = effective(&overrides, &s.app_name, &title);
        total_secs += s.duration_secs;
        *cats.entry(cat.clone()).or_insert(0) += s.duration_secs;
        let mins = (s.duration_secs / 60).max(1);
        lines.push(format!(
            "{} · {}min · [{}] {} — {}",
            hhmm(s.start_ts),
            mins,
            cat,
            s.app_name,
            title
        ));
    }
    let total_count = lines.len();
    if lines.len() > 120 {
        lines.truncate(120);
    }
    let mut top_cats: Vec<(String, i64)> = cats.into_iter().collect();
    top_cats.sort_by(|a, b| b.1.cmp(&a.1));
    top_cats.truncate(4);

    Ok(DayData {
        lines,
        total_secs,
        total_count,
        top_cats,
    })
}

/// Cabeçalho-resumo: data por extenso + tempo total + top categorias + aviso de corte.
fn day_header(day: Option<&str>, d: &DayData) -> String {
    let label = day_label(day);
    let weekday = NaiveDate::parse_from_str(&label, "%Y-%m-%d")
        .map(weekday_pt)
        .unwrap_or("");
    let cats = d
        .top_cats
        .iter()
        .map(|(c, s)| format!("{} {}min", c, s / 60))
        .collect::<Vec<_>>()
        .join(" · ");
    let trunc = if d.total_count > d.lines.len() {
        format!(
            " (mostrando as {} primeiras de {})",
            d.lines.len(),
            d.total_count
        )
    } else {
        String::new()
    };
    format!(
        "Dia: {} ({}) · {} de uso · {} sessões{}\nTop categorias: {}\n",
        label,
        weekday,
        fmt_hm(d.total_secs),
        d.total_count,
        trunc,
        cats
    )
}

/// Gera um texto pronto (já limpo pelo porteiro) pra COLAR numa IA forte
/// (ex.: Claude.ai). Não chama modelo nenhum — só monta o digest.
#[tauri::command]
pub async fn ai_day_digest(
    state: State<'_, AppState>,
    day: Option<String>,
) -> Result<String, String> {
    let d = collect_day(&state, day.as_deref())?;
    if d.lines.is_empty() {
        return Err(empty_msg(day.as_deref()));
    }
    let intentions = intentions_block(&state, day.as_deref());
    let header = day_header(day.as_deref(), &d);
    let joined = d.lines.join("\n");
    Ok(format!(
        "Você é meu assistente de produtividade (tenho TDAH). Analise meu dia de forma gentil \
e honesta, nunca punitiva. Identifique a atividade concreta pelos sites/títulos. Me diga: \
(1) o que fiz em blocos, (2) como foi meu foco, (3) onde perdi tempo sem perceber, \
(4) se bati minhas intenções, (5) UMA melhoria pra amanhã.\n\n\
{header}\n{intentions}Sessões (horário · duração · [categoria] · app/site — janela/URL):\n\n{joined}"
    ))
}
