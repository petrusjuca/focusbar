use crate::ai;
use crate::category::categorize;
use crate::commands::summaries::bounds_for_date_pub;
use crate::db;
use crate::redact;
use crate::state::AppState;
use chrono::{Local, TimeZone};
use tauri::State;

/// Ollama/Llama está disponível?
#[tauri::command]
pub async fn ai_available() -> bool {
    ai::is_available().await
}

fn hhmm(ts: i64) -> String {
    Local
        .timestamp_opt(ts, 0)
        .single()
        .map(|d| d.format("%H:%M").to_string())
        .unwrap_or_default()
}

/// Resumo do dia gerado pelo Llama local: episódios + como foi + 1 melhoria.
/// Usa as sessões já gravadas (focus_events), passando pelo porteiro antes.
#[tauri::command]
pub async fn ai_day_review(
    state: State<'_, AppState>,
    day: Option<String>,
) -> Result<String, String> {
    let (start, end) = bounds_for_date_pub(day.as_deref());

    // Coleta + limpa SEM segurar o lock durante o await.
    let mut lines: Vec<String> = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let sessions = db::sessions_in_range(&conn, start, end).map_err(|e| e.to_string())?;
        sessions
            .into_iter()
            .filter(|s| s.duration_secs >= 20)
            .filter(|s| !redact::is_excluded(&s.app_name, &s.title)) // zona de exclusão
            .map(|s| {
                let title = redact::redact(&s.title); // redação de sensível
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
            .collect()
    };

    if lines.is_empty() {
        return Err("Sem dados suficientes hoje pra analisar.".into());
    }

    // Mantém o prompt pequeno (3B + leve): no máximo 80 linhas.
    if lines.len() > 80 {
        lines.truncate(80);
    }
    let joined = lines.join("\n");

    let prompt = format!(
        "Você é um assistente de produtividade gentil e direto, para uma pessoa com TDAH. \
Tom NUNCA punitivo (jamais diga 'você perdeu tempo'). Responda em português do Brasil, curto e honesto.\n\n\
Sessões de foco de hoje (horário · duração · [categoria] · app — janela/URL):\n\n{joined}\n\n\
Use as URLs e títulos pra identificar a ATIVIDADE CONCRETA (ex.: 'pesquisando proxy', \
'YouTube assistindo gameplay', 'editando planilha', 'codando o app'), não só o nome do app.\n\n\
Responda em markdown, exatamente com estas seções:\n\
## O que você fez\n(3 a 5 blocos de atividade concreta, com horário e o que era de fato)\n\
## Como foi o dia\n(2 a 3 frases honestas e gentis sobre foco e dispersão)\n\
## 1 melhoria pra amanhã\n(uma sugestão pequena e concreta)\n\n\
Seja conciso e específico. Não invente nada que não esteja nos dados."
    );

    ai::generate(prompt).await
}
