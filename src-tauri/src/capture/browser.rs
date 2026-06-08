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
