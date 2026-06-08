use crate::category::{categorize, effective};
use crate::db;
use crate::insights::day_insights;
use crate::models::{CategoryTotal, DailySummary, DayTotal, FocusSession, Insight, WeeklySummary};
use crate::state::AppState;
use chrono::{Duration, Local, NaiveDate, TimeZone};
use std::collections::HashMap;
use tauri::State;

/// Início (epoch) do dia local para uma data.
fn day_start_ts(date: NaiveDate) -> i64 {
    let naive_start = date.and_hms_opt(0, 0, 0).unwrap();
    Local
        .from_local_datetime(&naive_start)
        .earliest()
        .map(|dt| dt.timestamp())
        .unwrap_or(0)
}

/// Bounds [start, end) e label de uma data.
fn bounds_for_date(date: NaiveDate) -> (i64, i64, String) {
    let start = day_start_ts(date);
    (start, start + 86_400, date.format("%Y-%m-%d").to_string())
}

/// Parseia 'YYYY-MM-DD' ou usa hoje.
fn parse_day(day: Option<&str>) -> NaiveDate {
    day.and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        .unwrap_or_else(|| Local::now().date_naive())
}

/// Helper público: bounds [start, end) de um dia (usado por outros módulos).
pub fn bounds_for_date_pub(day: Option<&str>) -> (i64, i64) {
    let (start, end, _) = bounds_for_date(parse_day(day));
    (start, end)
}

/// Resumo do dia: total focado, contagem de sessões, tempo por app e o pico de foco.
#[tauri::command]
pub fn get_daily_summary(
    state: State<AppState>,
    day: Option<String>,
) -> Result<DailySummary, String> {
    let (start, end, day_label) = bounds_for_date(parse_day(day.as_deref()));
    let conn = state.db.lock().map_err(|e| e.to_string())?;

    let mut by_app = db::app_totals(&conn, start, end).map_err(|e| e.to_string())?;
    // Preenche a categoria efetiva (override do usuário, ou regra pelo nome).
    for a in &mut by_app {
        if a.category.is_empty() {
            a.category = categorize(&a.app_name, "").to_string();
        }
    }
    let longest_session = db::longest_session(&conn, start, end).map_err(|e| e.to_string())?;

    let total_secs: i64 = by_app.iter().map(|a| a.total_secs).sum();
    let session_count: i64 = by_app.iter().map(|a| a.session_count).sum();

    Ok(DailySummary {
        day: day_label,
        total_secs,
        session_count,
        by_app,
        longest_session,
    })
}

/// Sessões de um dia, em ordem cronológica (para a timeline).
#[tauri::command]
pub fn get_day_sessions(
    state: State<AppState>,
    day: Option<String>,
) -> Result<Vec<FocusSession>, String> {
    let (start, end, _) = bounds_for_date(parse_day(day.as_deref()));
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::sessions_in_range(&conn, start, end).map_err(|e| e.to_string())
}

/// Tempo por categoria (Trabalho / Ferramenta / Procrastinação / Outro) num dia.
#[tauri::command]
pub fn get_category_summary(
    state: State<AppState>,
    day: Option<String>,
) -> Result<Vec<CategoryTotal>, String> {
    let (start, end, _) = bounds_for_date(parse_day(day.as_deref()));
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let sessions = db::sessions_in_range(&conn, start, end).map_err(|e| e.to_string())?;
    let overrides = db::category_overrides(&conn).map_err(|e| e.to_string())?;

    let mut totals: HashMap<String, i64> = HashMap::new();
    for s in &sessions {
        let cat = effective(&overrides, &s.app_name, &s.title);
        *totals.entry(cat).or_insert(0) += s.duration_secs;
    }

    let mut out: Vec<CategoryTotal> = totals
        .into_iter()
        .map(|(category, total_secs)| CategoryTotal {
            category,
            total_secs,
        })
        .collect();
    out.sort_by(|a, b| b.total_secs.cmp(&a.total_secs));
    Ok(out)
}

/// Define a categoria manual de um app/site (ex.: "Trabalho"). Vazio = limpa.
#[tauri::command]
pub fn set_app_category(
    state: State<AppState>,
    app: String,
    category: String,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::set_app_category(&conn, &app, &category).map_err(|e| e.to_string())
}

/// Dicas/insights do dia (o que melhorar), por regras.
#[tauri::command]
pub fn get_day_insights(
    state: State<AppState>,
    day: Option<String>,
) -> Result<Vec<Insight>, String> {
    let (start, end, _) = bounds_for_date(parse_day(day.as_deref()));
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    day_insights(&conn, start, end).map_err(|e| e.to_string())
}

/// Resumo dos últimos 7 dias (terminando em `end_day` ou hoje).
#[tauri::command]
pub fn get_weekly_summary(
    state: State<AppState>,
    end_day: Option<String>,
) -> Result<WeeklySummary, String> {
    let last = parse_day(end_day.as_deref());
    let conn = state.db.lock().map_err(|e| e.to_string())?;

    let mut days = Vec::with_capacity(7);
    for i in (0..7).rev() {
        let date = last - Duration::days(i);
        let (start, end, label) = bounds_for_date(date);
        let total = db::total_in_range(&conn, start, end).map_err(|e| e.to_string())?;
        days.push(DayTotal {
            day: label,
            total_secs: total,
        });
    }

    // Agregado da semana inteira por app.
    let week_start = day_start_ts(last - Duration::days(6));
    let week_end = day_start_ts(last) + 86_400;
    let by_app = db::app_totals(&conn, week_start, week_end).map_err(|e| e.to_string())?;
    let total_secs: i64 = by_app.iter().map(|a| a.total_secs).sum();

    Ok(WeeklySummary {
        days,
        total_secs,
        by_app,
    })
}
