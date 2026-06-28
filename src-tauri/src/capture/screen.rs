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

/// Captura a imagem da janela do processo `pid` (só ela). Casa pelo PID, NÃO por
/// "quem está em foco agora" — porque entre a decisão de capturar e a captura em
/// si o foco pode mudar (o OCR leva segundos). Capturar pelo PID da sessão impede
/// dois bugs: (a) atribuir o texto da janela errada à sessão; (b) — crítico —
/// fotografar um app sensível (banco/senha) que ganhou foco no meio. Se a janela
/// sumiu, devolve `None`. Bitmap em memória, nunca salvo.
pub fn capture_window_by_pid(pid: i32) -> Option<image::RgbaImage> {
    let target = pid as u32;
    let windows = xcap::Window::all().ok()?;
    // Prefere a janela em foco DESSE pid; senão, a primeira não-minimizada dele.
    let mut fallback: Option<xcap::Window> = None;
    for w in windows {
        if w.pid().unwrap_or(0) != target || w.is_minimized().unwrap_or(false) {
            continue;
        }
        if w.is_focused().unwrap_or(false) {
            return w.capture_image().ok();
        }
        if fallback.is_none() {
            fallback = Some(w);
        }
    }
    fallback.and_then(|w| w.capture_image().ok())
}

/// OCR da janela do processo `pid` (Estágio 2). Captura SÓ aquela janela e roda o
/// OCR NATIVO do SO (Apple Vision no Mac, Windows OCR no Win) em MEMÓRIA — a
/// imagem nunca toca o disco. Devolve o texto lido. Best-effort; precisa de
/// permissão de Gravação de Tela. O chamador deve passar o texto pelo porteiro.
pub async fn ocr_window_by_pid(pid: i32) -> Option<String> {
    let rgba = capture_window_by_pid(pid)?;
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
