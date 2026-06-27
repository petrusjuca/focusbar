use crate::category::{categorize, effective};
use crate::db;
use crate::insights::day_insights;
use crate::models::{
    CategoryTotal, DailySummary, DayMetrics, DayTotal, FocusSession, Insight, IntervalMarker,
    WeeklySummary,
};
use std::collections::HashSet;
use crate::state::AppState;
use chrono::{Duration, Local, LocalResult, NaiveDate, TimeZone};
use std::collections::HashMap;
use tauri::State;

/// Início (epoch) do dia local para uma data. Robusto a saltos de horário de
/// verão à meia-noite: se 00:00 não existir (spring-forward), anda minuto a minuto
/// até o 1º instante válido — nunca cai em epoch 0 (que zerava o dia inteiro).
fn day_start_ts(date: NaiveDate) -> i64 {
    let naive = date.and_hms_opt(0, 0, 0).unwrap();
    match Local.from_local_datetime(&naive) {
        LocalResult::Single(dt) => dt.timestamp(),
        LocalResult::Ambiguous(dt, _) => dt.timestamp(), // usa a 1ª ocorrência
        LocalResult::None => (1..=120)
            .find_map(|m| Local.from_local_datetime(&(naive + Duration::minutes(m))).earliest())
            .map(|dt| dt.timestamp())
            .unwrap_or_else(|| naive.and_utc().timestamp()),
    }
}

/// Bounds [start, end) e label de uma data. `end` é o início do dia SEGUINTE,
/// então dias de DST (23h/25h) ficam corretos — não somamos 86_400 fixo.
fn bounds_for_date(date: NaiveDate) -> (i64, i64, String) {
    let start = day_start_ts(date);
    let end = day_start_ts(date + Duration::days(1));
    (start, end, date.format("%Y-%m-%d").to_string())
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
    let raw = db::sessions_in_range(&conn, start, end).map_err(|e| e.to_string())?;
    // Agrupa o "confete" em atividades reais (mesma app, gaps <= 90s). Só na
    // exibição — o dado bruto fica intacto no banco.
    Ok(db::group_sessions(&raw, db::GROUP_GAP_SECS))
}

/// Métricas cruas do dia (painel de dados / debug — expõe tudo que capturamos).
#[tauri::command]
pub fn get_metrics(state: State<AppState>, day: Option<String>) -> Result<DayMetrics, String> {
    let (start, end, _) = bounds_for_date(parse_day(day.as_deref()));
    let now = chrono::Local::now().timestamp();
    let conn = state.db.lock().map_err(|e| e.to_string())?;

    let sessions = db::sessions_in_range(&conn, start, end).map_err(|e| e.to_string())?;
    let total_tracked_secs: i64 = sessions.iter().map(|s| s.duration_secs).sum();
    let session_count = sessions.len() as i64;
    let app_count = sessions
        .iter()
        .map(|s| s.app_name.as_str())
        .collect::<HashSet<_>>()
        .len() as i64;

    let (pomodoros, pomo_sum, pomodoros_completed) =
        db::pomodoro_stats(&conn, start, end).map_err(|e| e.to_string())?;
    let pomodoro_avg_secs = if pomodoros > 0 { pomo_sum / pomodoros } else { 0 };

    let paused_secs = db::marker_secs(&conn, &["paused"], start, end, now).map_err(|e| e.to_string())?;
    let away_secs = db::marker_secs(&conn, &["away", "away_uncertain"], start, end, now)
        .map_err(|e| e.to_string())?;

    Ok(DayMetrics {
        total_tracked_secs,
        session_count,
        app_count,
        pomodoros,
        pomodoro_avg_secs,
        pomodoros_completed,
        paused_secs,
        away_secs,
    })
}

/// Marcadores de intervalo do dia (pausado / ausente) — pra timeline rotular
/// cada minuto (o "sem dados" é derivado no frontend pelos buracos restantes).
#[tauri::command]
pub fn get_day_markers(
    state: State<AppState>,
    day: Option<String>,
) -> Result<Vec<IntervalMarker>, String> {
    let (start, end, _) = bounds_for_date(parse_day(day.as_deref()));
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::markers_in_range(&conn, start, end).map_err(|e| e.to_string())
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
        // Prioridade: override do usuário > categoria por CONTEÚDO (IA) > regra por app.
        let cat = if let Some(c) = db::app_category(&conn, &s.app_name)
            .ok()
            .flatten()
            .filter(|c| !c.is_empty())
        {
            c
        } else if let Some(c) = s.category_ai.clone() {
            c
        } else {
            effective(&overrides, &s.app_name, &s.title)
        };
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

/// Recategoriza UM bloco da linha do tempo (1 clique). `until_ts` é o começo do
/// próximo bloco (ou agora+1 se for o último) — assim só as sessões deste bloco
/// mudam, sem afetar outros usos do mesmo app no dia.
#[tauri::command]
pub fn set_block_category(
    state: State<AppState>,
    app: String,
    start_ts: i64,
    until_ts: i64,
    category: String,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::set_block_category(&conn, &app, start_ts, until_ts, &category)
        .map_err(|e| e.to_string())?;
    Ok(())
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
    let week_end = day_start_ts(last + Duration::days(1));
    let by_app = db::app_totals(&conn, week_start, week_end).map_err(|e| e.to_string())?;
    let total_secs: i64 = by_app.iter().map(|a| a.total_secs).sum();

    Ok(WeeklySummary {
        days,
        total_secs,
        by_app,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn day_window_is_24h() {
        let (start, end) = bounds_for_date_pub(Some("2026-01-15"));
        assert_eq!(end - start, 86_400);
        assert!(start > 0);
    }

    #[test]
    fn invalid_date_falls_back_to_today() {
        let (start, end) = bounds_for_date_pub(Some("nao-e-data"));
        assert_eq!(end - start, 86_400);
        assert!(start > 0);
    }

    #[test]
    fn none_uses_today() {
        let (start, end) = bounds_for_date_pub(None);
        assert_eq!(end - start, 86_400);
    }

    #[test]
    fn parse_day_roundtrips_label() {
        let (_, _, label) = bounds_for_date(parse_day(Some("2026-03-09")));
        assert_eq!(label, "2026-03-09");
    }
}
