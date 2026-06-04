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
