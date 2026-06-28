//! Leitura do título da janela em foco via API de Acessibilidade (AX) do macOS.
//! Diferente do CGWindowList (kCGWindowName, que exige permissão de Gravação de
//! Tela), a AX precisa apenas da permissão de Acessibilidade — alinhado com a
//! promessa de "só metadados".

use accessibility::{AXAttribute, AXUIElement};
use core_foundation::base::{CFType, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::string::CFString;

/// Destrava a leitura de CONTEÚDO no Chrome/Electron. Esses apps escondem a
/// árvore da página da Acessibilidade por padrão (só expõem o título) — por isso
/// "Chrome só dava o nome das coisas". `AXManualAccessibility = true` diz "exponha
/// seu conteúdo", e aí a árvore passa a ter o texto real da página (como o Firefox
/// e os apps nativos já faziam). Best-effort: apps que não entendem o atributo
/// simplesmente ignoram. Idempotente — pode chamar a cada leitura.
fn enable_web_accessibility(app: &AXUIElement) {
    // A crate só constrói AXAttribute<CFType>; passamos o booleano como CFType.
    let attr = AXAttribute::new(&CFString::from_static_string("AXManualAccessibility"));
    let _ = app.set_attribute(&attr, CFBoolean::true_value().into_CFType());
}

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
    // Limites maiores que antes: agora que o Chrome expõe a página, o texto útil
    // mora mais fundo na árvore (window → … → AXWebArea → grupos → textos). Ainda
    // bounded pra ficar barato (roda 1x por sessão).
    if depth > 10 || *nodes > 320 || out.len() > 1500 {
        return;
    }
    *nodes += 1;

    for name in ["AXValue", "AXTitle", "AXDescription"] {
        if let Some(t) = text_attr(el, name) {
            if !out.contains(&t) {
                out.push_str(&t);
                out.push('\n');
                if out.len() > 1500 {
                    return;
                }
            }
        }
    }

    if let Ok(children) = el.attribute(&AXAttribute::children()) {
        for child in children.iter() {
            collect_text(&child, depth + 1, out, nodes);
            if *nodes > 320 || out.len() > 1500 {
                return;
            }
        }
    }
}

/// Coleta o texto da PÁGINA: desce a árvore inteira, mas só junta texto quando já
/// está DENTRO de uma `AXWebArea` (a raiz do conteúdo web do Chrome/Safari/Firefox).
/// Assim ignora a moldura do navegador (barra de abas, botões, títulos de outras
/// abas) e pega o corpo da página de verdade. Orçamento de travessia generoso
/// porque a web area costuma ficar fundo, depois da UI do navegador.
fn collect_web_text(
    el: &AXUIElement,
    depth: usize,
    out: &mut String,
    nodes: &mut usize,
    in_web: bool,
) {
    if depth > 14 || *nodes > 800 || out.len() > 1500 {
        return;
    }
    *nodes += 1;

    let is_web = in_web || text_attr(el, "AXRole").as_deref() == Some("AXWebArea");
    if is_web {
        for name in ["AXValue", "AXTitle", "AXDescription"] {
            if let Some(t) = text_attr(el, name) {
                if !out.contains(&t) {
                    out.push_str(&t);
                    out.push('\n');
                    if out.len() > 1500 {
                        return;
                    }
                }
            }
        }
    }

    if let Ok(children) = el.attribute(&AXAttribute::children()) {
        for child in children.iter() {
            collect_web_text(&child, depth + 1, out, nodes, is_web);
            if *nodes > 800 || out.len() > 1500 {
                return;
            }
        }
    }
}

/// A string tem cara de URL/domínio? (pra achar a barra de endereço na árvore AX
/// sem depender do rótulo localizado do campo, que muda por idioma/navegador.)
fn looks_like_url(s: &str) -> bool {
    let t = s.trim();
    if t.is_empty() || t.contains(' ') || t.len() > 300 {
        return false;
    }
    if t.contains("://") {
        return true;
    }
    // host/caminho sem esquema: precisa de host com TLD alfabético plausível.
    let host = t.split('/').next().unwrap_or("");
    let labels: Vec<&str> = host.split('.').collect();
    if labels.len() < 2 {
        return false;
    }
    let tld = labels[labels.len() - 1];
    tld.len() >= 2 && tld.len() <= 24 && tld.chars().all(|c| c.is_ascii_alphabetic())
}

/// Caminha a árvore AX procurando a barra de endereço: o primeiro AXTextField cujo
/// valor parece uma URL. Funciona em navegadores que NÃO expõem AppleScript (Opera
/// GX, Firefox), porque lê o que está na tela, não a API de scripting.
fn find_url_field(el: &AXUIElement, depth: usize, nodes: &mut usize) -> Option<String> {
    if depth > 8 || *nodes > 400 {
        return None;
    }
    *nodes += 1;

    if text_attr(el, "AXRole").as_deref() == Some("AXTextField") {
        if let Some(v) = text_attr(el, "AXValue") {
            if looks_like_url(&v) {
                return Some(v);
            }
        }
    }

    if let Ok(children) = el.attribute(&AXAttribute::children()) {
        for child in children.iter() {
            if let Some(u) = find_url_field(&child, depth + 1, nodes) {
                return Some(u);
            }
            if *nodes > 400 {
                return None;
            }
        }
    }
    None
}

/// URL da aba ativa lida pela Acessibilidade (fallback do AppleScript). Para
/// navegadores não-scriptáveis como o Opera GX: lê o valor da barra de endereço
/// direto da árvore AX da janela em foco. Só precisa da permissão de Acessibilidade.
pub fn focused_browser_url(pid: i32) -> Option<String> {
    let app = AXUIElement::application(pid);
    let fw = AXAttribute::new(&CFString::from_static_string("AXFocusedWindow"));
    let value: CFType = app.attribute(&fw).ok()?;
    let window = value.downcast::<AXUIElement>()?;
    let mut nodes = 0usize;
    find_url_field(&window, 0, &mut nodes)
}

#[cfg(test)]
mod tests {
    use super::looks_like_url;

    #[test]
    fn url_heuristic() {
        assert!(looks_like_url("https://github.com/owner/repo"));
        assert!(looks_like_url("youtube.com/watch")); // sem esquema
        assert!(looks_like_url("claude.ai"));
        assert!(!looks_like_url("3.14"));            // tld não-alfabético
        assert!(!looks_like_url("buscar no histórico")); // tem espaço
        assert!(!looks_like_url("documento"));       // sem ponto
        assert!(!looks_like_url(""));
    }
}

/// Estágio 1 do "olhos": texto visível da janela em foco, lido pela árvore de
/// Acessibilidade (mesma permissão que já temos — SEM screenshot, sem Gravação
/// de Tela). Cobre apps nativos (PDF no Preview, Notas, Slack, docs). Best-effort.
pub fn focused_window_text(pid: i32) -> Option<String> {
    let app = AXUIElement::application(pid);
    // Liga a exposição de conteúdo (Chrome/Electron) ANTES de ler — senão só vem
    // o título. É o conserto do "ele só pega o nome das coisas".
    enable_web_accessibility(&app);

    let fw = AXAttribute::new(&CFString::from_static_string("AXFocusedWindow"));
    let value: CFType = app.attribute(&fw).ok()?;
    let window = value.downcast::<AXUIElement>()?;

    let mut out = String::new();
    let mut nodes = 0usize;
    // 1) Tenta o CONTEÚDO DA PÁGINA (navegadores): mira a AXWebArea, pula a moldura.
    collect_web_text(&window, 0, &mut out, &mut nodes, false);
    // 2) Sem web area (app nativo: PDF, Notas, Ajustes…) → lê a janela inteira.
    if out.trim().is_empty() {
        nodes = 0;
        collect_text(&window, 0, &mut out, &mut nodes);
    }

    let s = out.trim();
    if s.is_empty() {
        None
    } else {
        // Corta em ~700 caracteres pra não estourar o contexto do modelo local.
        Some(s.chars().take(700).collect())
    }
}
