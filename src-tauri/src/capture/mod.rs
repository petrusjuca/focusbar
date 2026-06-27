use crate::models::ActiveWindow;

#[cfg(target_os = "macos")]
mod macos;

pub mod browser;
pub mod sampler;
pub mod screen;
pub mod signals;
pub mod state;

/// "Olhos" Estágio 1: texto visível da janela em foco via Acessibilidade (sem
/// screenshot). macOS por enquanto; Windows (UI Automation) é o próximo passo.
#[cfg(target_os = "macos")]
pub fn focused_text(pid: i32) -> Option<String> {
    macos::focused_window_text(pid)
}

#[cfg(not(target_os = "macos"))]
pub fn focused_text(_pid: i32) -> Option<String> {
    None
}

/// Abstração de captura da janela em foco. Mantém o resto do app independente
/// da crate concreta usada por baixo — dá pra trocar por uma implementação
/// nativa sem mexer no sampler nem nos commands.
pub trait WindowProvider: Send + Sync {
    /// Retorna a janela atualmente em foco, ou None se nada estiver em foco
    /// (ex.: desktop vazio) ou a leitura falhar de forma recuperável.
    fn current(&self) -> Option<ActiveWindow>;
}

/// Implementação cross-platform via crate `active-win-pos-rs`.
/// - macOS: nome do app sempre (kCGWindowOwnerName, sem permissão); título via
///   AX (precisa de Acessibilidade) quando o CGWindowList vier vazio.
/// - Windows: GetForegroundWindow + título (sem permissão).
pub struct ActiveWinProvider;

impl WindowProvider for ActiveWinProvider {
    fn current(&self) -> Option<ActiveWindow> {
        match active_win_pos_rs::get_active_window() {
            Ok(w) => {
                let mut title = w.title;

                // No macOS o título via CGWindowList exige Gravação de Tela; se
                // vier vazio, tenta a via AX (só precisa de Acessibilidade).
                #[cfg(target_os = "macos")]
                if title.is_empty() {
                    if let Some(t) = macos::focused_window_title(w.process_id as i32) {
                        title = t;
                    }
                }

                Some(ActiveWindow {
                    app_name: w.app_name,
                    app_bundle_id: if w.process_path.as_os_str().is_empty() {
                        None
                    } else {
                        Some(w.process_path.to_string_lossy().into_owned())
                    },
                    title,
                    pid: w.process_id as i32,
                })
            }
            Err(_) => None,
        }
    }
}
