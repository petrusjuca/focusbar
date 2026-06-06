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
