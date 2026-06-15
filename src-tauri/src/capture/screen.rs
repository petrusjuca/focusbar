//! Estágio 2 dos "olhos": captura da janela em foco + permissão de Gravação de
//! Tela. A captura (xcap) e a permissão compilam SEM Xcode; o motor de OCR
//! (uni-ocr / Apple Vision) é plugado depois, por cima desta base.
//!
//! Privacidade: capturamos SÓ a janela em foco (não a tela toda), e o bitmap é
//! efêmero — nunca persistido. O texto do OCR ainda passa pelo porteiro (redact).

/// Permissão de Gravação de Tela (macOS). No Windows não é exigida.
#[cfg(target_os = "macos")]
mod perm {
    // Funções C do CoreGraphics (macOS 10.15+). Linkam direto o framework —
    // não precisa do Xcode completo, só do Command Line Tools.
    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGPreflightScreenCaptureAccess() -> bool;
        fn CGRequestScreenCaptureAccess() -> bool;
    }
    /// Já temos permissão de Gravação de Tela? (só checa, não pede)
    pub fn granted() -> bool {
        unsafe { CGPreflightScreenCaptureAccess() }
    }
    /// Dispara o diálogo do sistema pedindo a permissão. Após conceder, o app
    /// precisa ser reiniciado pra valer.
    pub fn request() -> bool {
        unsafe { CGRequestScreenCaptureAccess() }
    }
}

#[cfg(not(target_os = "macos"))]
mod perm {
    pub fn granted() -> bool {
        true // Windows: captura de janela não exige permissão.
    }
    pub fn request() -> bool {
        true
    }
}

pub fn screen_recording_granted() -> bool {
    perm::granted()
}

pub fn request_screen_recording() -> bool {
    perm::request()
}

/// Captura a imagem da janela ATUALMENTE em foco (só ela). `None` se não houver
/// janela visível em foco ou a captura falhar. Bitmap em memória, nunca salvo.
pub fn capture_focused_window() -> Option<image::RgbaImage> {
    let windows = xcap::Window::all().ok()?;
    for w in windows {
        if w.is_focused().unwrap_or(false) {
            if w.is_minimized().unwrap_or(false) {
                return None;
            }
            return w.capture_image().ok();
        }
    }
    None
}

/// OCR da janela em foco (Estágio 2). Captura SÓ a janela em foco e roda o OCR
/// NATIVO do SO (Apple Vision no Mac, Windows OCR no Win) em MEMÓRIA — a imagem
/// nunca toca o disco. Devolve o texto lido. Best-effort; precisa de permissão
/// de Gravação de Tela. O chamador deve passar o texto pelo porteiro (redact).
pub async fn ocr_focused_window() -> Option<String> {
    let rgba = capture_focused_window()?;
    let img = image::DynamicImage::ImageRgba8(rgba);
    let engine = uni_ocr::OcrEngine::new(uni_ocr::OcrProvider::Auto)
        .ok()?
        .with_options(
            uni_ocr::OcrOptions::default().languages(vec![
                uni_ocr::Language::Portuguese,
                uni_ocr::Language::English,
            ]),
        );
    let (text, _, _) = engine.recognize_image(&img).await.ok()?;
    let text = text.trim().to_string();
    if text.is_empty() {
        None
    } else {
        // Corta pra não estourar o contexto do modelo local.
        Some(text.chars().take(900).collect())
    }
}
