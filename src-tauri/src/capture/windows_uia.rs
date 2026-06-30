//! Leitura do CONTEÚDO da janela em foco no Windows via UI Automation (UIA) — o
//! equivalente Windows da Acessibilidade do Mac. SEM screenshot, só a árvore de
//! UI (nomes de controles, textos, valores). Best-effort: se algo falhar, devolve
//! None e o chamador cai pro OCR / título.
//!
//! Bom saber: no Windows o Chrome/Edge EXPÕEM o conteúdo da página pra UIA quando
//! uma ferramenta de acessibilidade consulta (é o que fazemos) — então aqui dá pra
//! ler o corpo da página, diferente do macOS (que precisava do AXManualAccessibility).

use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED,
};
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationElement, IUIAutomationTreeWalker,
};
use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

/// Caminha a árvore de UIA juntando os `Name` dos elementos. Limitado em
/// profundidade/nós/tamanho — UIA é COM cross-process (cada chamada custa), então
/// pegamos só o topo da janela, que já basta pra categorizar.
fn collect(
    walker: &IUIAutomationTreeWalker,
    el: &IUIAutomationElement,
    depth: usize,
    out: &mut String,
    nodes: &mut usize,
) {
    if depth > 6 || *nodes > 120 || out.len() > 1200 {
        return;
    }
    *nodes += 1;

    unsafe {
        if let Ok(name) = el.CurrentName() {
            let s = name.to_string();
            let t = s.trim();
            if t.len() >= 2 && !out.contains(t) {
                out.push_str(t);
                out.push('\n');
                if out.len() > 1200 {
                    return;
                }
            }
        }

        if let Ok(first) = walker.GetFirstChildElement(el) {
            let mut cur = first;
            loop {
                collect(walker, &cur, depth + 1, out, nodes);
                if *nodes > 120 || out.len() > 1200 {
                    return;
                }
                match walker.GetNextSiblingElement(&cur) {
                    Ok(next) => cur = next,
                    Err(_) => break,
                }
            }
        }
    }
}

/// Texto visível da janela em PRIMEIRO PLANO via UIA. `_pid` é ignorado — usamos
/// a janela de foreground do SO (que é a da sessão no momento da leitura).
pub fn focused_window_text(_pid: i32) -> Option<String> {
    unsafe {
        // COM em multithread; se já inicializado em outro modo, segue.
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);

        let automation: IUIAutomation =
            CoCreateInstance(&CUIAutomation, None, CLSCTX_ALL).ok()?;
        let hwnd: HWND = GetForegroundWindow();
        if hwnd.0.is_null() {
            return None;
        }
        let root: IUIAutomationElement = automation.ElementFromHandle(hwnd).ok()?;
        let walker: IUIAutomationTreeWalker = automation.RawViewWalker().ok()?;

        let mut out = String::new();
        let mut nodes = 0usize;
        collect(&walker, &root, 0, &mut out, &mut nodes);

        let s = out.trim();
        if s.is_empty() {
            None
        } else {
            Some(s.chars().take(700).collect())
        }
    }
}
