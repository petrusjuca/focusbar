//! Lê a URL/aba ativa do navegador via AppleScript (Automação) — funciona no
//! macOS atual, sem gravação de tela. Dá ao assistente contexto MUITO melhor:
//! "Chrome" → "youtube.com/watch... vídeo de gameplay".
//!
//! Na primeira vez, o macOS pede permissão de Automação ("focusbar quer controlar
//! o Google Chrome") — é só permitir. Em outros SOs, retorna None.

#[cfg(target_os = "macos")]
use std::process::Command;

/// Navegadores baseados em Chromium (mesmo comando AppleScript).
#[cfg(target_os = "macos")]
const CHROMIUM: &[&str] = &[
    "Google Chrome",
    "Google Chrome Canary",
    "Brave Browser",
    "Microsoft Edge",
    "Arc",
    "Chromium",
    "Opera",
    "Vivaldi",
];

/// URL da aba/documento ativo, se o app for um navegador conhecido.
#[cfg(target_os = "macos")]
pub fn browser_url(app_name: &str) -> Option<String> {
    let script = if CHROMIUM.contains(&app_name) {
        format!(
            "tell application \"{}\" to get URL of active tab of front window",
            app_name
        )
    } else if app_name == "Safari" {
        "tell application \"Safari\" to get URL of front document".to_string()
    } else {
        return None;
    };
    // NOTA: nada de `with timeout` curto aqui — ele cancelava o prompt de permissão
    // de Automação antes do usuário clicar "Permitir". A proteção contra travamento
    // (raro) fica pra uma leitura de URL fora do thread do sampler, no futuro.

    let out = Command::new("osascript").arg("-e").arg(&script).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let url = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if url.is_empty() || url == "missing value" {
        None
    } else {
        Some(url)
    }
}

#[cfg(not(target_os = "macos"))]
pub fn browser_url(_app_name: &str) -> Option<String> {
    None
}

/// É um navegador? (lista ampla — inclui os NÃO-scriptáveis tipo Opera GX, pra
/// decidir se vale tentar o fallback de URL por Acessibilidade.) Case-insensitive.
pub fn is_browser(app_name: &str) -> bool {
    let n = app_name.to_lowercase();
    [
        "chrome", "chromium", "brave", "edge", "arc", "vivaldi", "opera", "opera gx",
        "safari", "firefox", "zen", "orion", "duckduckgo",
    ]
    .iter()
    .any(|b| n.contains(b))
}

/// Tira o LIXO que o Chrome enfia no título da janela: o aviso "Uso elevado da
/// memória em…", o "(NN)" de contador, o ": 1,2 GB" de memória, o "- Reprodução
/// de áudio" e o sufixo "- Google Chrome: perfil". Sobra o nome real da página
/// (o vídeo, a busca, o artigo) — o que faz a captura PARECER (e ser) inteligente.
pub fn clean_browser_title(title: &str) -> String {
    use std::sync::OnceLock;
    static MEM: OnceLock<regex::Regex> = OnceLock::new();
    static COUNT: OnceLock<regex::Regex> = OnceLock::new();
    let mem = MEM.get_or_init(|| regex::Regex::new(r":\s*[\d.,]+\s*(GB|MB)").unwrap());
    let count = COUNT.get_or_init(|| regex::Regex::new(r"^\(\d+\)\s*").unwrap());

    let mut t = title.to_string();
    // 1) corta o sufixo do navegador (e o ": perfil" que vem depois).
    for b in [
        " - Google Chrome",
        " — Google Chrome",
        " - Chromium",
        " - Microsoft Edge",
        " - Opera",
        " - Brave",
    ] {
        if let Some(i) = t.find(b) {
            t.truncate(i);
            break;
        }
    }
    // 2) remove decoradores do Chrome.
    t = t.replace(" - Reprodução de áudio", "");
    t = mem.replace_all(&t, "").into_owned();
    // 3) prefixos de aviso de memória.
    for p in ["Uso elevado da memória em ", "Uso da memória em "] {
        if let Some(rest) = t.strip_prefix(p) {
            t = rest.to_string();
        }
    }
    // 4) contador "(NN) " no começo.
    t = count.replace(&t, "").into_owned();
    t.trim().to_string()
}

/// O texto lido pela Acessibilidade é só a MOLDURA do Chrome (barra de abas/botões),
/// não a página? Só faz sentido no macOS, onde o Chrome ESCONDE o conteúdo web da AX
/// — aí caímos no título limpo + OCR. No Windows a UIA já traz o corpo da página,
/// então nunca descartamos (retorna false).
#[cfg(target_os = "macos")]
pub fn is_chrome_frame(text: &str) -> bool {
    text.contains(" - Google Chrome") || text.contains("Pesquisa em todas as guias")
}

#[cfg(not(target_os = "macos"))]
pub fn is_chrome_frame(_text: &str) -> bool {
    false
}

/// URL "limpa" pro título: descarta query (`?`) e fragment (`#`), que costumam
/// carregar tokens/PII (reset token, access_token de OAuth, `?email=`, session id).
/// Preserva scheme + host + path (contexto útil tipo `/watch`, `/r/rust`).
pub fn clean_url(url: &str) -> String {
    let no_frag = url.split('#').next().unwrap_or(url);
    no_frag
        .split('?')
        .next()
        .unwrap_or(no_frag)
        .trim_end_matches('/')
        .to_string()
}

/// Nome "bonito" do site a partir da URL (ex.: web.whatsapp.com → "WhatsApp",
/// youtube.com → "YouTube"). Assim cada site vira uma entrada própria no
/// dashboard, em vez de tudo cair em "Chrome"/"Opera".
pub fn site_name(url: &str) -> Option<String> {
    // tira esquema, pega o host, tira porta/caminho/query
    let after = url.split("://").nth(1).unwrap_or(url);
    let host = after
        .split('/')
        .next()
        .unwrap_or("")
        .split('?')
        .next()
        .unwrap_or("")
        .split(':')
        .next()
        .unwrap_or("")
        .trim_start_matches("www.")
        .to_lowercase();
    if host.is_empty() {
        return None;
    }
    // pega o rótulo principal do domínio (web.whatsapp.com → "whatsapp")
    let labels: Vec<&str> = host.split('.').collect();
    let main = if labels.len() >= 2 {
        labels[labels.len() - 2]
    } else {
        labels[0]
    };

    let nice = match main {
        "whatsapp" => "WhatsApp",
        "youtube" | "youtu" => "YouTube",
        "miro" => "Miro",
        "google" => "Google",
        "gmail" | "mail" => "Email",
        "github" => "GitHub",
        "discord" => "Discord",
        "reddit" => "Reddit",
        "notion" => "Notion",
        "figma" => "Figma",
        "x" | "twitter" => "X (Twitter)",
        "instagram" => "Instagram",
        "netflix" => "Netflix",
        "twitch" => "Twitch",
        "tiktok" => "TikTok",
        "chatgpt" | "openai" => "ChatGPT",
        "claude" | "anthropic" => "Claude",
        "linkedin" => "LinkedIn",
        "spotify" => "Spotify",
        "adspower" => "AdsPower",
        other => {
            // Title-case do rótulo (ex.: "stackoverflow" → "Stackoverflow")
            let mut c = other.chars();
            return c.next().map(|f| {
                f.to_uppercase().collect::<String>() + c.as_str()
            });
        }
    };
    Some(nice.to_string())
}

/// Host + 1º segmento do caminho de uma URL (ambos minúsculos).
fn host_and_first_seg(url: &str) -> (String, String) {
    let after = url.split("://").nth(1).unwrap_or(url);
    let mut parts = after.splitn(2, '/');
    let host = parts
        .next()
        .unwrap_or("")
        .split(['?', ':'])
        .next()
        .unwrap_or("")
        .to_lowercase();
    let seg = parts
        .next()
        .unwrap_or("")
        .split(['/', '?', '#'])
        .next()
        .unwrap_or("")
        .to_lowercase();
    (host, seg)
}

/// Pra domínios onde o nome-base esconde apps distintos (Claude chat × cowork ×
/// console), descobre a "superfície" pelo subdomínio/caminho.
fn claude_surface(url: &str) -> String {
    let (host, seg) = host_and_first_seg(url);
    if host.starts_with("console.") {
        return "console".into();
    }
    if host.starts_with("cowork.") {
        return "cowork".into();
    }
    match seg.as_str() {
        "" | "chat" | "chats" | "new" | "recents" => "chat".into(),
        "cowork" => "cowork".into(),
        "code" => "code".into(),
        "projects" | "project" => "projetos".into(),
        other => other.to_string(),
    }
}

/// Rótulo do site PRO DASHBOARD — como `site_name`, mas diferencia sub-apps do
/// mesmo domínio que senão colapsariam (ex.: os "2 Claudes" → "Claude (chat)"
/// vs "Claude (cowork)" vs "Claude (console)"). É o que o sampler usa.
pub fn site_label(url: &str) -> Option<String> {
    let base = site_name(url)?;
    if base == "Claude" {
        return Some(format!("Claude ({})", claude_surface(url)));
    }
    Some(base)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diferencia_os_dois_claudes() {
        assert_eq!(
            site_label("https://claude.ai/chat/abc").as_deref(),
            Some("Claude (chat)")
        );
        assert_eq!(site_label("https://claude.ai/").as_deref(), Some("Claude (chat)"));
        assert_eq!(
            site_label("https://cowork.anthropic.com/x").as_deref(),
            Some("Claude (cowork)")
        );
        assert_eq!(
            site_label("https://console.anthropic.com/").as_deref(),
            Some("Claude (console)")
        );
        // sites normais passam igual
        assert_eq!(
            site_label("https://www.youtube.com/watch?v=a").as_deref(),
            Some("YouTube")
        );
    }

    #[test]
    fn maps_known_sites() {
        assert_eq!(site_name("https://web.whatsapp.com/").as_deref(), Some("WhatsApp"));
        assert_eq!(
            site_name("https://www.youtube.com/watch?v=abc").as_deref(),
            Some("YouTube")
        );
        assert_eq!(site_name("https://github.com/owner/repo").as_deref(), Some("GitHub"));
        assert_eq!(site_name("https://x.com/home").as_deref(), Some("X (Twitter)"));
    }

    #[test]
    fn strips_www_path_and_query() {
        assert_eq!(
            site_name("https://www.reddit.com/r/rust?sort=new").as_deref(),
            Some("Reddit")
        );
    }

    #[test]
    fn unknown_host_is_title_cased() {
        assert_eq!(
            site_name("https://stackoverflow.com/questions/1").as_deref(),
            Some("Stackoverflow")
        );
    }

    #[test]
    fn empty_url_returns_none() {
        assert_eq!(site_name(""), None);
        assert_eq!(site_name("https://"), None);
    }

    #[test]
    fn limpa_lixo_do_titulo_do_chrome() {
        assert_eq!(
            clean_browser_title(
                "Uso elevado da memória em (143) NÃO FAZ SENTIDO O HENRY DANGER - YouTube - Reprodução de áudio: 1,3 GB - Google Chrome: petrus"
            ),
            "NÃO FAZ SENTIDO O HENRY DANGER - YouTube"
        );
        assert_eq!(
            clean_browser_title("Uso elevado da memória em (18) WhatsApp: 1,2 GB - Google Chrome: petrus"),
            "WhatsApp"
        );
        // título já limpo (en-dash não é separador) passa intacto, sem o sufixo.
        assert_eq!(
            clean_browser_title("Café – Wikipédia, a enciclopédia livre - Google Chrome: petrus"),
            "Café – Wikipédia, a enciclopédia livre"
        );
    }

    #[test]
    fn is_browser_pega_opera_gx_e_outros() {
        assert!(is_browser("Opera GX"));
        assert!(is_browser("Google Chrome"));
        assert!(is_browser("Safari"));
        assert!(is_browser("Firefox"));
        assert!(is_browser("Arc"));
        assert!(!is_browser("Notepad"));
        assert!(!is_browser("Slack"));
        assert!(!is_browser("Visual Studio Code"));
    }
}
