//! Checagem e solicitação da permissão de Acessibilidade (macOS).
//! Em outros SOs (Windows), o título da janela não exige permissão → "ok".

/// True se o app tem permissão de Acessibilidade (sempre true fora do macOS).
#[tauri::command]
pub fn check_accessibility() -> bool {
    #[cfg(target_os = "macos")]
    {
        macos_accessibility_client::accessibility::application_is_trusted()
    }
    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}

/// Dispara o prompt do sistema e abre Ajustes do Sistema → Privacidade e
/// Segurança → Acessibilidade. Retorna o status atual (geralmente ainda false
/// logo após o prompt, pois o usuário precisa marcar o toggle).
#[tauri::command]
pub fn request_accessibility() -> bool {
    #[cfg(target_os = "macos")]
    {
        macos_accessibility_client::accessibility::application_is_trusted_with_prompt()
    }
    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}

/// True se há permissão de Gravação de Tela (necessária pro OCR de pixel).
/// No Windows não é exigida → sempre true.
#[tauri::command]
pub fn check_screen_recording() -> bool {
    crate::capture::screen::screen_recording_granted()
}

/// Dispara o diálogo de permissão de Gravação de Tela (macOS). Após conceder,
/// o app precisa ser reiniciado pra valer.
#[tauri::command]
pub fn request_screen_recording() -> bool {
    crate::capture::screen::request_screen_recording()
}
