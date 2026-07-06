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

/// OCR nativo (Apple Vision / Windows OCR) de uma imagem em MEMÓRIA — nunca toca
/// o disco. Devolve o texto, ou None se vazio.
async fn ocr_image(rgba: image::RgbaImage) -> Option<String> {
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
        Some(text.chars().take(900).collect())
    }
}

/// Captura pra OCR, ROBUSTA. Tenta só a janela do pid (privacidade); se o `xcap`
/// falhar em listá-la (a enumeração de janelas dele é INTERMITENTE — às vezes
/// devolve 2 janelas, às vezes 19), cai pra captura da TELA CHEIA, que é confiável.
/// Como o OCR roda enquanto a janela da sessão está em foco, a tela é dominada por
/// ela. A imagem nunca toca o disco e o texto passa pelo porteiro (redact) depois.
fn capture_for_ocr(pid: i32) -> Option<image::RgbaImage> {
    // Tenta a janela do pid; o xcap é intermitente, então tenta de novo uma vez.
    if let Some(img) = capture_window_by_pid(pid) {
        return Some(img);
    }
    if let Some(img) = capture_window_by_pid(pid) {
        return Some(img);
    }
    // Fallback de tela cheia SÓ se a janela em foco AINDA é a da sessão. Senão a
    // tela mostra outra janela e o conteúdo seria atribuído à sessão ERRADA
    // (ex.: conteúdo do Claude numa sessão de YouTube). Honestidade > cobertura.
    if current_focused_pid() != Some(pid) {
        return None;
    }
    let monitors = xcap::Monitor::all().ok()?;
    monitors.into_iter().next().and_then(|m| m.capture_image().ok())
}

/// PID do app atualmente em foco (pra validar o fallback de tela cheia).
fn current_focused_pid() -> Option<i32> {
    active_win_pos_rs::get_active_window()
        .ok()
        .map(|w| w.process_id as i32)
}

/// OCR da janela do processo `pid` (Estágio 2), com fallback de tela cheia. A
/// imagem nunca toca o disco. Best-effort; o chamador passa o texto pelo porteiro.
pub async fn ocr_window_by_pid(pid: i32) -> Option<String> {
    let rgba = capture_for_ocr(pid)?;
    ocr_image(rgba).await
}

/// Captura da janela pra SHOT + OCR num frame só (o sampler usa: uma captura
/// por sessão estável, reaproveitada pros dois fins).
pub fn frame_for_session(pid: i32) -> Option<image::RgbaImage> {
    capture_for_ocr(pid)
}

/// OCR de um frame JÁ capturado (evita fotografar duas vezes).
pub async fn ocr_frame(rgba: image::RgbaImage) -> Option<String> {
    ocr_image(rgba).await
}

/// D1 (decisão 04.07, pedido do João #16): salva o frame da sessão em disco,
/// LOCAL, pra "ver em que aba estava". Retenção curta (48h, ver purge) e
/// toggle em Configurações. Zonas de exclusão nunca chegam aqui — o sampler
/// barra antes. Reduz pra <=1280px e grava JPEG q70 (~100-200KB por shot).
pub fn save_shot(
    dir: &std::path::Path,
    rgba: &image::RgbaImage,
    ts: i64,
    app: &str,
) -> Option<String> {
    std::fs::create_dir_all(dir).ok()?;
    let img = image::DynamicImage::ImageRgba8(rgba.clone());
    let img = if img.width() > 1280 {
        img.resize(1280, 4096, image::imageops::FilterType::Triangle)
    } else {
        img
    };
    let safe: String = app
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .take(24)
        .collect();
    let path = dir.join(format!("{ts}-{safe}.jpg"));
    let file = std::fs::File::create(&path).ok()?;
    let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(file, 70);
    enc.encode_image(&img.to_rgb8()).ok()?;
    Some(path.to_string_lossy().into_owned())
}

/// Auto-limpeza da retenção: apaga shots mais velhos que `keep_secs`.
pub fn purge_old_shots(dir: &std::path::Path, keep_secs: u64) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let now = std::time::SystemTime::now();
    for e in entries.flatten() {
        let old = e
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| now.duration_since(t).ok())
            .is_some_and(|d| d.as_secs() > keep_secs);
        if old {
            let _ = std::fs::remove_file(e.path());
        }
    }
}

/// OCR da TELA CHEIA (monitor principal) — pro botão "testar os olhos": prova,
/// na hora, que o screenshot+OCR estão vivos nesta máquina. Imagem em memória.
pub async fn ocr_primary_monitor() -> Option<String> {
    let monitors = xcap::Monitor::all().ok()?;
    let img = monitors.into_iter().next()?.capture_image().ok()?;
    ocr_image(img).await
}
