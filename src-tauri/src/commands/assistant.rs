use crate::ai;
use crate::category::categorize;
use crate::commands::summaries::bounds_for_date_pub;
use crate::db;
use crate::redact;
use crate::state::AppState;
use chrono::{Local, TimeZone};
use serde::Serialize;
use tauri::State;

/// Ollama/Llama está disponível?
#[tauri::command]
pub async fn ai_available() -> bool {
    ai::is_available().await
}

#[derive(Serialize)]
pub struct AiStatus {
    pub running: bool, // Ollama está rodando?
    pub model: bool,   // modelo já baixado?
}

/// Estado da IA: Ollama rodando? modelo baixado? (pro app guiar o setup sozinho)
#[tauri::command]
pub async fn ai_status() -> AiStatus {
    let running = ai::is_available().await;
    let model = if running { ai::has_model().await } else { false };
    AiStatus { running, model }
}

/// Baixa o modelo (clique único na UI; bloqueia até terminar).
#[tauri::command]
pub async fn ai_pull_model() -> Result<(), String> {
    ai::pull_model().await
}

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

/// Coleta as sessões do dia já LIMPAS (porteiro: redação + zonas de exclusão),
/// formatadas em linhas. Síncrono — não segura lock em await.
fn collect_clean_lines(
    state: &State<'_, AppState>,
    day: Option<&str>,
) -> Result<Vec<String>, String> {
    let (start, end) = bounds_for_date_pub(day);
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let sessions = db::sessions_in_range(&conn, start, end).map_err(|e| e.to_string())?;
    let mut lines: Vec<String> = sessions
        .into_iter()
        .filter(|s| s.duration_secs >= 20)
        .filter(|s| !redact::is_excluded(&s.app_name, &s.title))
        .map(|s| {
            let title = redact::redact(&s.title);
            let cat = categorize(&s.app_name, &title);
            let mins = (s.duration_secs / 60).max(1);
            format!(
                "{} · {}min · [{}] {} — {}",
                hhmm(s.start_ts),
                mins,
                cat,
                s.app_name,
                title
            )
        })
        .collect();
    if lines.len() > 120 {
        lines.truncate(120);
    }
    Ok(lines)
}

/// Resumo do dia gerado pelo Llama LOCAL: o que fez + como foi + 1 melhoria.
#[tauri::command]
pub async fn ai_day_review(
    state: State<'_, AppState>,
    day: Option<String>,
) -> Result<String, String> {
    let lines = collect_clean_lines(&state, day.as_deref())?;
    if lines.is_empty() {
        return Err("Sem dados suficientes hoje pra analisar.".into());
    }
    let intentions = intentions_block(&state, day.as_deref());
    let joined = lines.join("\n");
    let prompt = format!(
        "Você é um assistente de produtividade gentil e direto, para uma pessoa com TDAH. \
Tom NUNCA punitivo. Responda em português do Brasil, curto e honesto.\n\n\
{intentions}Sessões de hoje (horário · duração · [categoria] · app/site — janela/URL):\n\n{joined}\n\n\
Use as URLs e títulos pra identificar a ATIVIDADE CONCRETA, não só o nome do app.\n\n\
Responda em markdown com estas seções:\n\
## O que você fez\n(3 a 5 blocos de atividade concreta, com horário)\n\
## Como foi o dia\n(2 a 3 frases honestas; se houver intenções, diga se bateu com elas)\n\
## 1 melhoria pra amanhã\n(uma sugestão pequena e concreta)\n\n\
Seja conciso. Não invente nada que não esteja nos dados."
    );
    ai::generate(prompt).await
}

/// Gera um texto pronto (já limpo pelo porteiro) pra COLAR numa IA forte
/// (ex.: Claude.ai). Não chama modelo nenhum — só monta o digest.
#[tauri::command]
pub async fn ai_day_digest(
    state: State<'_, AppState>,
    day: Option<String>,
) -> Result<String, String> {
    let lines = collect_clean_lines(&state, day.as_deref())?;
    if lines.is_empty() {
        return Err("Sem dados suficientes hoje.".into());
    }
    let intentions = intentions_block(&state, day.as_deref());
    let joined = lines.join("\n");
    Ok(format!(
        "Você é meu assistente de produtividade (tenho TDAH). Analise meu dia de forma gentil \
e honesta, nunca punitiva. Identifique a atividade concreta pelos sites/títulos. Me diga: \
(1) o que fiz em blocos, (2) como foi meu foco, (3) onde perdi tempo sem perceber, \
(4) se bati minhas intenções, (5) UMA melhoria pra amanhã.\n\n\
{intentions}Sessões de hoje (horário · duração · [categoria] · app/site — janela/URL):\n\n{joined}"
    ))
}
