//! Última aba ativa reportada pela EXTENSÃO de browser (Fase A do roadmap).
//!
//! A extensão manda cada troca de aba pra API local (POST /api/tab-event);
//! aqui fica só o estado "qual é a aba ativa agora" + a decisão de quando essa
//! informação vale pra janela em foco. É a fonte de URL que funciona onde
//! AppleScript e Acessibilidade falham (Opera GX, Windows) — e a decisão é
//! pura, sem I/O, testável em qualquer SO.

use crate::capture::browser;
use std::sync::Mutex;

/// Evento recente o bastante pra confiar mesmo sem conferir o título: acabou
/// de trocar de aba e o sampler ainda nem viu o título novo estabilizar.
const FRESH_SECS: i64 = 10;

#[derive(Debug, Clone)]
pub struct TabInfo {
    pub url: String,
    pub title: String,
    /// Identificador que a extensão deduz do próprio browser ("opera gx",
    /// "chrome", "edge"...). Vazio = não soube dizer.
    pub browser: String,
    pub tab_id: String,
    pub ts: i64,
}

/// Estado compartilhado entre a API local (escreve) e o sampler (lê).
pub struct TabFeed {
    inner: Mutex<Option<TabInfo>>,
    /// ts do último evento recebido, mesmo que a aba já tenha sido trocada —
    /// é o "sinal de vida" da extensão pro /api/health e pra UI.
    last_event_ts: std::sync::atomic::AtomicI64,
}

impl TabFeed {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(None),
            last_event_ts: std::sync::atomic::AtomicI64::new(0),
        }
    }

    /// Aba ativou/atualizou: vira a "aba ativa agora".
    pub fn record(&self, info: TabInfo) {
        self.last_event_ts
            .store(info.ts, std::sync::atomic::Ordering::Relaxed);
        if let Ok(mut cur) = self.inner.lock() {
            *cur = Some(info);
        }
    }

    /// Aba fechou: se era a ativa, esquece (senão a URL dela "vazaria" pra
    /// próxima janela do browser).
    pub fn forget_tab(&self, tab_id: &str, ts: i64) {
        self.last_event_ts
            .store(ts, std::sync::atomic::Ordering::Relaxed);
        if let Ok(mut cur) = self.inner.lock() {
            if cur.as_ref().is_some_and(|t| t.tab_id == tab_id) {
                *cur = None;
            }
        }
    }

    /// ts do último evento da extensão (0 = nunca recebeu nada).
    pub fn last_event_ts(&self) -> i64 {
        self.last_event_ts.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// URL da aba ativa SE ela vale pra janela em foco agora (ver `applies`).
    pub fn url_for(&self, app_name: &str, window_title: &str, now: i64) -> Option<String> {
        let cur = self.inner.lock().ok()?;
        let info = cur.as_ref()?;
        if applies(info, app_name, window_title, now) {
            Some(info.url.clone())
        } else {
            None
        }
    }
}

impl Default for TabFeed {
    fn default() -> Self {
        Self::new()
    }
}

/// A aba reportada pela extensão descreve a janela em foco?
///
/// Duas defesas contra atribuir a URL ERRADA (pior que nenhuma):
/// 1. Browser identificado e é OUTRO (extensão no Opera GX, foco no Chrome) → não.
/// 2. Evento velho E título da janela não bate com o da aba → não (evento
///    perdido, janela nova do mesmo browser etc.).
fn applies(info: &TabInfo, app_name: &str, window_title: &str, now: i64) -> bool {
    if info.url.is_empty() {
        return false;
    }
    if let Some(matches) = browser_matches(&info.browser, app_name) {
        if !matches {
            return false;
        }
    }
    let fresh = now - info.ts <= FRESH_SECS;
    fresh || titles_match(window_title, &info.title)
}

/// O browser da extensão é o app em foco? None = extensão não identificou o
/// browser (aí só o frescor/título decidem).
fn browser_matches(feed_browser: &str, app_name: &str) -> Option<bool> {
    let b = feed_browser.trim().replace('-', " ").to_lowercase();
    if b.is_empty() || b == "chromium" {
        return None;
    }
    let app = app_name.to_lowercase();
    // "opera gx" ⊇ "opera", então basta o rótulo estar contido no nome do app
    // ou vice-versa ("opera gx" da extensão × app "Opera").
    Some(app.contains(&b) || b.contains(&app) || app.split_whitespace().any(|w| b.contains(w)))
}

/// Título da janela (limpo do lixo do browser) confere com o título da aba?
fn titles_match(window_title: &str, tab_title: &str) -> bool {
    let w = browser::clean_browser_title(window_title).to_lowercase();
    let t = tab_title.trim().to_lowercase();
    if w.is_empty() || t.is_empty() {
        return false;
    }
    w.contains(&t) || t.contains(&w)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(url: &str, title: &str, browser: &str, ts: i64) -> TabInfo {
        TabInfo {
            url: url.into(),
            title: title.into(),
            browser: browser.into(),
            tab_id: "42".into(),
            ts,
        }
    }

    #[test]
    fn evento_fresco_vale_mesmo_sem_titulo_bater() {
        let i = info("https://youtube.com/watch?v=a", "Video Novo", "opera gx", 100);
        assert!(applies(&i, "Opera GX", "titulo ainda antigo", 105));
    }

    #[test]
    fn evento_velho_vale_se_titulo_bate() {
        let i = info("https://web.whatsapp.com/", "WhatsApp", "opera gx", 100);
        // 2h depois, mesma aba aberta — título da janela ainda é o da aba.
        assert!(applies(&i, "Opera GX", "WhatsApp - Opera", 7300));
    }

    #[test]
    fn evento_velho_com_titulo_diferente_nao_vale() {
        let i = info("https://web.whatsapp.com/", "WhatsApp", "opera gx", 100);
        assert!(!applies(&i, "Opera GX", "Outra Coisa Qualquer", 7300));
    }

    #[test]
    fn browser_errado_nunca_vale() {
        let i = info("https://web.whatsapp.com/", "WhatsApp", "opera gx", 100);
        // Extensão no Opera GX, mas o foco é o Chrome: rejeita mesmo fresco.
        assert!(!applies(&i, "Google Chrome", "WhatsApp", 101));
    }

    #[test]
    fn browser_generico_cai_no_frescor_e_titulo() {
        let fresco = info("https://a.com/x", "Página A", "chromium", 100);
        assert!(applies(&fresco, "Google Chrome", "qualquer", 102));
        let velho = info("https://a.com/x", "Página A", "", 100);
        assert!(!applies(&velho, "Google Chrome", "qualquer", 999));
        assert!(applies(&velho, "Google Chrome", "Página A - Google Chrome", 999));
    }

    #[test]
    fn matching_de_browser_cobre_os_nomes_reais() {
        assert_eq!(browser_matches("opera gx", "Opera GX"), Some(true));
        assert_eq!(browser_matches("opera-gx", "Opera GX"), Some(true));
        assert_eq!(browser_matches("opera", "Opera GX"), Some(true));
        assert_eq!(browser_matches("chrome", "Google Chrome"), Some(true));
        assert_eq!(browser_matches("edge", "Microsoft Edge"), Some(true));
        assert_eq!(browser_matches("opera gx", "Google Chrome"), Some(false));
        assert_eq!(browser_matches("chromium", "Opera GX"), None);
        assert_eq!(browser_matches("", "Opera GX"), None);
    }

    #[test]
    fn url_vazia_nao_vale() {
        let i = info("", "WhatsApp", "opera gx", 100);
        assert!(!applies(&i, "Opera GX", "WhatsApp", 101));
    }

    #[test]
    fn fechar_a_aba_ativa_limpa_o_feed() {
        let feed = TabFeed::new();
        feed.record(info("https://a.com", "A", "opera gx", 100));
        assert!(feed.url_for("Opera GX", "A", 101).is_some());
        feed.forget_tab("42", 102);
        assert!(feed.url_for("Opera GX", "A", 103).is_none());
        assert_eq!(feed.last_event_ts(), 102);
    }

    #[test]
    fn fechar_outra_aba_nao_limpa() {
        let feed = TabFeed::new();
        feed.record(info("https://a.com", "A", "opera gx", 100));
        feed.forget_tab("99", 102);
        assert!(feed.url_for("Opera GX", "A", 103).is_some());
    }
}
