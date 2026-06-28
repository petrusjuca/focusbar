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

/// Palavras genéricas que não contam como "tópico" do foco.
const STOP: &[&str] = &[
    "para", "prova", "sobre", "como", "isso", "esse", "essa", "tema", "fazer",
    "minha", "minhas", "meus", "pelo", "pela", "hoje", "dele", "trabalho",
    "coisa", "estudar", "terminar", "fazendo", "preciso",
];

/// Normaliza pra casar tópicos: minúsculas + remove acentos comuns do PT.
/// Resultado é ASCII (seguro pra fatiar por byte). "Cálculo" e "calculo" viram
/// a mesma coisa — corrige o erro silencioso mais comum em português.
fn fold(s: &str) -> String {
    s.chars()
        .flat_map(|c| c.to_lowercase())
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            'ñ' => 'n',
            other => other,
        })
        .collect()
}

/// O texto da janela (AX/título) está fraco demais pra julgar pelo conteúdo?
/// Curto, ou com poucas palavras distintas, ou só ruído de UI → vale acionar o
/// OCR de pixel. Evita o caso "40 chars de boilerplate passam no gate antigo".
fn extra_is_weak(text: &str) -> bool {
    let t = text.trim();
    if t.chars().count() < 40 {
        return true;
    }
    let distinct: std::collections::HashSet<&str> = t
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.chars().count() >= 3)
        .collect();
    distinct.len() < 4
}

/// Procura uma palavra-chave do foco (≥5 letras, fora da STOP) dentro de um
/// texto JÁ normalizado por `fold`. Tolera plural simples (sufixo "s").
fn focus_keyword_in(focus: &str, hay_folded: &str) -> Option<String> {
    fold(focus)
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.chars().count() >= 5 && !STOP.contains(w))
        .map(|w| w.to_string())
        .find_map(|w| {
            if hay_folded.contains(&w) {
                Some(w)
            } else if w.len() >= 6 && w.ends_with('s') && hay_folded.contains(&w[..w.len() - 1])
            {
                Some(w[..w.len() - 1].to_string())
            } else {
                None
            }
        })
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

    // CACHE: enquanto a janela em foco (foco|app|título) não muda, reusa o último
    // veredito por até 120s — NÃO relê AX/OCR nem reaciona o Ollama. É o que evita
    // o modelo ficar "quente" pra sempre batendo a cada 30s (bateria/ventilador).
    let now = now_ts();
    let key = format!("{}\u{1}{}\u{1}{}", focus, app, title);
    if let Ok(c) = state.last_check.lock() {
        if let Some(cc) = c.as_ref() {
            if cc.key == key && now - cc.ts < 300 {
                return Ok(FocusCheck {
                    focus: Some(cc.focus.clone()),
                    app: Some(cc.app.clone()),
                    on_task: cc.on_task,
                    reason: cc.reason.clone(),
                    source: cc.source.clone(),
                });
            }
        }
    }
    // Constrói o FocusCheck E grava no cache (usado em todos os retornos pesados).
    let store = |on_task: Option<bool>, reason: String, source: &str| -> FocusCheck {
        if let Ok(mut c) = state.last_check.lock() {
            *c = Some(crate::state::CachedCheck {
                key: key.clone(),
                focus: focus.clone(),
                app: app.clone(),
                on_task,
                reason: reason.clone(),
                source: source.to_string(),
                ts: now,
            });
        }
        FocusCheck {
            focus: Some(focus.clone()),
            app: Some(app.clone()),
            on_task,
            reason,
            source: source.to_string(),
        }
    };

    // "Olhos" Estágio 1: lê o texto visível da janela pela Acessibilidade (sem
    // screenshot), passa pelo porteiro, e dá esse contexto pra IA julgar melhor.
    let mut extra = crate::capture::focused_text(pid)
        .map(|t| redact::redact(&t))
        .unwrap_or_default();

    // "Olhos" Estágio 2 (OCR de pixel): só se a AX veio fraca/vazia, o OCR está
    // LIGADO e há permissão de Gravação de Tela. Lê o texto da janela em foco via
    // OCR nativo (Apple Vision / Windows OCR), em memória. Passa pelo porteiro.
    if extra_is_weak(&extra)
        && ocr_enabled
        && crate::capture::screen::screen_recording_granted()
    {
        if let Some(t) = crate::capture::screen::ocr_window_by_pid(pid).await {
            extra = redact::redact(&t);
        }
    }

    // 1.5) Casamento de TÓPICO (sinal forte, sem IA): se a atividade contém uma
    //      palavra-chave do foco, está NO FOCO. Ex.: foco "estudar cálculo" +
    //      página de Cálculo → no foco. Corrige o 3B errando o óbvio.
    {
        let hay = fold(&format!("{} {} {}", app, title, extra));
        if let Some(kw) = focus_keyword_in(&focus, &hay) {
            return Ok(store(Some(true), format!("Bate com seu foco (\"{}\").", kw), "match"));
        }
    }

    // 2) IA julga pelo CONTEÚDO — mas ANCORADA: ela precisa copiar um trecho
    //    real da tela como prova. Se o trecho não existir de verdade (modelo
    //    inventou pela "funcionalidade" do app), DESCARTAMOS o veredito e caímos
    //    no fallback determinístico. Assim a IA só "fala" o que dá pra comprovar.
    if let Ok(j) = ai::on_task_check(&focus, &app, &title, &extra).await {
        let ev = j.evidence.trim();
        let grounded = ev.len() >= 3
            && ev.to_lowercase() != "nada"
            && fold(&format!("{} {}", title, extra)).contains(&fold(ev));
        if grounded {
            return Ok(store(Some(j.on_task), format!("Vi na tela: \"{}\".", ev), "ia"));
        }
        // Sem prova verificável → a IA não ancorou. Ignora e segue pro fallback.
    }

    // 3) Sem Ollama: antes da regra burra, reaproveita o texto que JÁ lemos
    //    (OCR/AX) — se o conteúdo cita o foco, é um palpite informado de "no foco".
    if let Some(kw) = focus_keyword_in(&focus, &fold(&extra)) {
        return Ok(store(
            Some(true),
            format!("O conteúdo mostra \"{}\" (sem Ollama, é um palpite).", kw),
            "match",
        ));
    }

    // 4) Último recurso: regra pela categoria do app (estimativa honesta).
    let on_task = cat != "Procrastinação";
    let reason = if on_task {
        format!(
            "Pela categoria, {} costuma ser trabalho — mas é só estimativa. Ligue o Ollama (aba Assistente) pra eu julgar pelo conteúdo.",
            app
        )
    } else {
        format!(
            "Pela categoria, {} costuma ser distração — mas é só estimativa. Ligue o Ollama (aba Assistente) pra eu julgar pelo conteúdo.",
            app
        )
    };
    Ok(store(Some(on_task), reason, "rule"))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fold_remove_acentos() {
        assert_eq!(fold("Cálculo"), "calculo");
        assert_eq!(fold("AÇÃO"), "acao");
        assert_eq!(fold("relatório"), "relatorio");
    }

    #[test]
    fn match_ignora_acento_e_caixa() {
        // foco com acento casa com texto sem acento (e vice-versa)
        assert_eq!(
            focus_keyword_in("estudar cálculo", &fold("Lista 3 - calculo 1.pdf")),
            Some("calculo".into())
        );
        assert_eq!(
            focus_keyword_in("relatorio do projeto", &fold("Relatório Q4 — Docs")),
            Some("relatorio".into())
        );
    }

    #[test]
    fn match_tolera_plural() {
        assert_eq!(
            focus_keyword_in("derivadas", &fold("aula de derivada")),
            Some("derivada".into())
        );
    }

    #[test]
    fn match_ignora_stopwords_e_curtas() {
        // só "trabalho"(stop) e "no"(curta) → nenhum tópico real
        assert_eq!(focus_keyword_in("trabalho", &fold("YouTube - novela")), None);
    }

    #[test]
    fn weak_detecta_texto_fraco() {
        assert!(extra_is_weak(""));
        assert!(extra_is_weak("Nova aba"));
        assert!(extra_is_weak("a b c d e f")); // muitas palavras curtas (<3)
        assert!(!extra_is_weak(
            "Cálculo diferencial: máximos e mínimos por derivada primeira"
        ));
    }
}
