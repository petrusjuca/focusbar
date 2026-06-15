//! Leitura do título da janela em foco via API de Acessibilidade (AX) do macOS.
//! Diferente do CGWindowList (kCGWindowName, que exige permissão de Gravação de
//! Tela), a AX precisa apenas da permissão de Acessibilidade — alinhado com a
//! promessa de "só metadados".

use accessibility::{AXAttribute, AXUIElement};
use core_foundation::base::CFType;
use core_foundation::string::CFString;

/// Título da janela atualmente focada do processo `pid`.
/// Retorna None se a permissão de Acessibilidade não foi concedida, se o app
/// não tem janela focada, ou se o título estiver vazio.
pub fn focused_window_title(pid: i32) -> Option<String> {
    let app = AXUIElement::application(pid);

    // A crate `accessibility` não expõe um atalho pra kAXFocusedWindowAttribute,
    // então construímos o atributo manualmente e fazemos downcast pro elemento.
    let focused_window_attr =
        AXAttribute::new(&CFString::from_static_string("AXFocusedWindow"));
    let value: CFType = app.attribute(&focused_window_attr).ok()?;
    let window = value.downcast::<AXUIElement>()?;

    let title = window.attribute(&AXAttribute::title()).ok()?;
    let s = title.to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

/// Lê um atributo de texto (CFString) de um elemento AX, se houver e não for vazio.
fn text_attr(el: &AXUIElement, name: &str) -> Option<String> {
    let a = AXAttribute::new(&CFString::new(name));
    let v: CFType = el.attribute(&a).ok()?;
    let s = v.downcast::<CFString>()?.to_string();
    let t = s.trim();
    if t.len() >= 2 {
        Some(t.to_string())
    } else {
        None
    }
}

/// Caminha a árvore AX coletando o texto visível (labels, valores de campos,
/// textos estáticos). Limitado em profundidade/nós/tamanho pra ficar barato.
fn collect_text(el: &AXUIElement, depth: usize, out: &mut String, nodes: &mut usize) {
    if depth > 4 || *nodes > 60 || out.len() > 1200 {
        return;
    }
    *nodes += 1;

    for name in ["AXValue", "AXTitle", "AXDescription"] {
        if let Some(t) = text_attr(el, name) {
            if !out.contains(&t) {
                out.push_str(&t);
                out.push('\n');
                if out.len() > 1200 {
                    return;
                }
            }
        }
    }

    if let Ok(children) = el.attribute(&AXAttribute::children()) {
        for child in children.iter() {
            collect_text(&child, depth + 1, out, nodes);
            if *nodes > 60 || out.len() > 1200 {
                return;
            }
        }
    }
}

/// Estágio 1 do "olhos": texto visível da janela em foco, lido pela árvore de
/// Acessibilidade (mesma permissão que já temos — SEM screenshot, sem Gravação
/// de Tela). Cobre apps nativos (PDF no Preview, Notas, Slack, docs). Best-effort.
pub fn focused_window_text(pid: i32) -> Option<String> {
    let app = AXUIElement::application(pid);
    let fw = AXAttribute::new(&CFString::from_static_string("AXFocusedWindow"));
    let value: CFType = app.attribute(&fw).ok()?;
    let window = value.downcast::<AXUIElement>()?;

    let mut out = String::new();
    let mut nodes = 0usize;
    collect_text(&window, 0, &mut out, &mut nodes);

    let s = out.trim();
    if s.is_empty() {
        None
    } else {
        // Corta em ~700 caracteres pra não estourar o contexto do modelo local.
        Some(s.chars().take(700).collect())
    }
}
