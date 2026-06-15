use crate::ai;
use crate::capture::{ActiveWinProvider, WindowProvider};
use crate::category::categorize;
use crate::db;
use crate::redact;
use crate::state::AppState;
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;

fn now_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[tauri::command]
pub fn set_focus(state: State<AppState>, text: String) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    if text.trim().is_empty() {
        db::clear_focus(&conn).map_err(|e| e.to_string())
    } else {
        db::set_focus(&conn, text.trim(), now_ts()).map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub fn get_focus(state: State<AppState>) -> Result<Option<String>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::get_focus(&conn).map_err(|e| e.to_string())
}

#[derive(Serialize)]
pub struct FocusCheck {
    pub focus: Option<String>,
    pub app: Option<String>,
    pub on_task: Option<bool>,
    pub reason: String,
    pub source: String, // "user" | "rule" | "ia" | "none"
}

/// Checa se a janela atual ajuda no foco. Camadas: correção do usuário → regra
/// (procrastinação) → IA local. Só chama o modelo quando precisa.
#[tauri::command]
pub async fn check_focus(state: State<'_, AppState>) -> Result<FocusCheck, String> {
    let none = |reason: &str| FocusCheck {
        focus: None,
        app: None,
        on_task: None,
        reason: reason.into(),
        source: "none".into(),
    };

    // Lê foco + correção/categoria e SOLTA o lock antes de qualquer await.
    let win = ActiveWinProvider.current();
    let (app, title, pid) = match &win {
        Some(w) => {
            // Porteiro: zonas de exclusão (banco, gerenciador de senha, saúde) NEM
            // são analisadas — nunca vão pro modelo. Checa o título BRUTO.
            if redact::is_excluded(&w.app_name, &w.title) {
                return Ok(none("Sessão em zona de exclusão (não analisada)."));
            }
            (w.app_name.clone(), redact::redact(&w.title), w.pid)
        }
        None => return Ok(none("Sem janela em foco.")),
    };

    let (focus, rule, cat, ocr_enabled) = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let focus = db::get_focus(&conn).map_err(|e| e.to_string())?;
        let focus = match focus {
            Some(f) if !f.trim().is_empty() => f,
            _ => return Ok(none("Defina um foco pra ele acompanhar.")),
        };
        let rule = db::get_focus_rule(&conn, &focus, &app)
            .map_err(|e| e.to_string())?;
        let cat = db::app_category(&conn, &app)
            .map_err(|e| e.to_string())?
            .unwrap_or_else(|| categorize(&app, &title).to_string());
        let ocr_enabled = db::get_setting(&conn, "ocr_enabled")
            .ok()
            .flatten()
            .map(|v| v == "1")
            .unwrap_or(false);
        (focus, rule, cat, ocr_enabled)
    };

    if app == "focusbar" {
        return Ok(FocusCheck {
            focus: Some(focus),
            app: Some(app),
            on_task: None,
            reason: "Você está no próprio focusbar.".into(),
            source: "none".into(),
        });
    }

    // 1) Correção do usuário (sticky).
    if let Some(on_task) = rule {
        return Ok(FocusCheck {
            focus: Some(focus),
            app: Some(app),
            on_task: Some(on_task),
            reason: if on_task {
                "Você marcou como útil pra esse foco.".into()
            } else {
                "Você marcou como distração.".into()
            },
            source: "user".into(),
        });
    }

    // "Olhos" Estágio 1: lê o texto visível da janela pela Acessibilidade (sem
    // screenshot), passa pelo porteiro, e dá esse contexto pra IA julgar melhor.
    let mut extra = crate::capture::focused_text(pid)
        .map(|t| redact::redact(&t))
        .unwrap_or_default();

    // "Olhos" Estágio 2 (OCR de pixel): só se a AX veio fraca/vazia, o OCR está
    // LIGADO e há permissão de Gravação de Tela. Lê o texto da janela em foco via
    // OCR nativo (Apple Vision / Windows OCR), em memória. Passa pelo porteiro.
    if extra.trim().len() < 40
        && ocr_enabled
        && crate::capture::screen::screen_recording_granted()
    {
        if let Some(t) = crate::capture::screen::ocr_focused_window().await {
            extra = redact::redact(&t);
        }
    }

    // 1.5) Casamento de TÓPICO (sinal forte, sem IA): se a atividade contém uma
    //      palavra-chave do foco, está NO FOCO. Ex.: foco "estudar cálculo" +
    //      página de Cálculo → no foco. Corrige o 3B errando o óbvio.
    {
        let hay = format!("{} {} {}", app, title, extra).to_lowercase();
        const STOP: &[&str] = &[
            "para", "prova", "sobre", "como", "isso", "esse", "essa", "tema",
            "fazer", "minha", "minhas", "meus", "pelo", "pela", "hoje", "dele",
            "trabalho", "coisa", "estudar",
        ];
        let kw = focus
            .to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| w.chars().count() >= 5 && !STOP.contains(w))
            .find(|w| hay.contains(*w))
            .map(|w| w.to_string());
        if let Some(kw) = kw {
            return Ok(FocusCheck {
                focus: Some(focus),
                app: Some(app),
                on_task: Some(true),
                reason: format!("Bate com seu foco (\"{}\").", kw),
                source: "match".into(),
            });
        }
    }

    // 2) IA julga pelo CONTEÚDO: título + texto da tela = o ASSUNTO real, não o
    //    nome do site. Ex.: vídeo de "Cálculo 1 - derivadas" → estudo; novela →
    //    distração; PDF com o nome do projeto → trabalho. (Precisa do Ollama.)
    if let Ok((on_task, reason)) = ai::on_task_check(&focus, &app, &title, &extra).await {
        return Ok(FocusCheck {
            focus: Some(focus),
            app: Some(app),
            on_task: Some(on_task),
            reason,
            source: "ia".into(),
        });
    }

    // 3) Sem Ollama: cai pra regra pelo site (mais burra, mas instantânea).
    let on_task = cat != "Procrastinação";
    let reason = if on_task {
        format!(
            "{} costuma ser trabalho. Ligue o Ollama (aba Assistente) pra eu avaliar pelo conteúdo.",
            app
        )
    } else {
        format!(
            "{} costuma ser distração. Ligue o Ollama (aba Assistente) pra eu avaliar pelo conteúdo.",
            app
        )
    };
    Ok(FocusCheck {
        focus: Some(focus),
        app: Some(app),
        on_task: Some(on_task),
        reason,
        source: "rule".into(),
    })
}

/// Correção do usuário: para o foco atual, marca o app como útil ou distração.
#[tauri::command]
pub fn set_focus_judgment(
    state: State<AppState>,
    app: String,
    on_task: bool,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let focus = db::get_focus(&conn)
        .map_err(|e| e.to_string())?
        .ok_or("Sem foco definido.")?;
    db::set_focus_rule(&conn, &focus, &app, on_task).map_err(|e| e.to_string())
}
