//! Sampler em background: amostra a janela em foco a cada POLL_SECS, detecta
//! troca, descarta trocas curtas (debounce) e grava UMA linha por sessão.
//!
//! - Navegadores: o "app" vira o SITE (WhatsApp, Miro, YouTube) via a URL, em
//!   vez de "Chrome"/"Opera" — assim o "tempo por app" fica útil.
//! - Pause: quando `paused`, não conta nada (sem ser procrastinação).
//! - Idle: sem input por IDLE_THRESHOLD_SECS fecha no último input.

use crate::capture::tab_feed::TabFeed;
use crate::capture::{browser, signals, state, ActiveWinProvider, WindowProvider};
use crate::coach::Coach;
use crate::db;
use rusqlite::Connection;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager};

const POLL_SECS: u64 = 1;
const MIN_SESSION_SECS: i64 = 2;
const IDLE_THRESHOLD_SECS: i64 = 120;
const CONTENT_DELAY: i64 = 2; // lê o conteúdo cedo (2s): enquanto a janela ainda
                              // está em foco → o fallback de tela cheia casa e pega
                              // o CORPO da página com mais frequência (não só o título)

/// Texto da janela fraco demais pra categorizar? (curto ou poucas palavras) →
/// vale acionar o OCR de pixel.
fn content_is_weak(t: &str) -> bool {
    let s = t.trim();
    if s.chars().count() < 40 {
        return true;
    }
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.chars().count() >= 3)
        .count()
        < 4
}

struct OpenSession {
    app_id: i64,
    raw_app: String,      // ex.: "Opera GX" — usado pra detectar troca
    raw_title: String,    // título cru — usado pra detectar troca
    label_app: String,    // ex.: "WhatsApp" (site) — usado no banco/coach
    stored_title: String, // título + URL — gravado e mandado pro assistente
    start_ts: i64,
    pid: i32,
    content: Option<String>, // trecho REDIGIDO da tela (pra IA categorizar por conteúdo)
    content_done: bool,      // já capturamos o conteúdo desta sessão?
    /// Linha desta sessão no banco (HEARTBEAT, Fase B): criada quando a sessão
    /// se firma (>=2s) e estendida a cada batida. None = ainda curta demais.
    row_id: Option<i64>,
}

fn now_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn idle_secs() -> i64 {
    user_idle::UserIdle::get_time()
        .map(|t| t.as_seconds() as i64)
        .unwrap_or(0)
}

/// Apps que NÃO são foco real (tela de bloqueio, shell do SO) — ruído.
fn is_system_noise(app: &str) -> bool {
    let a = app.to_lowercase();
    // macOS
    if a == "loginwindow" || a == "windowserver" || a == "screensaverengine" {
        return true;
    }
    // Windows: tela de bloqueio, UAC, shell e processos de sistema. (NÃO inclui
    // "explorer" pra não descartar o uso real do Explorador de Arquivos.)
    matches!(
        a.as_str(),
        "lockapp"
            | "lockapp.exe"
            | "winlogon"
            | "winlogon.exe"
            | "csrss"
            | "csrss.exe"
            | "dwm"
            | "dwm.exe"
            | "conhost"
            | "conhost.exe"
            | "searchhost"
            | "searchhost.exe"
            | "shellexperiencehost"
            | "startmenuexperiencehost"
            | "applicationframehost"
            | "textinputhost"
    )
}

pub fn spawn(
    app: AppHandle,
    db: Arc<Mutex<Connection>>,
    paused: Arc<AtomicBool>,
    tab_feed: Arc<TabFeed>,
) {
    thread::spawn(move || {
        let provider = ActiveWinProvider;
        let mut current: Option<OpenSession> = None;
        let mut coach = Coach::new();
        let mut prev_state = state::TickState::Active;
        // Runtime próprio: o OCR (uni-ocr) é async, mas esta thread é síncrona.
        // 1 worker basta — só roda esporádico (1x por sessão estável).
        let ocr_rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .ok();

        loop {
            let now = now_ts();
            let idle = idle_secs();
            let is_idle = idle >= IDLE_THRESHOLD_SECS;
            let is_paused = paused.load(Ordering::Relaxed);

            // Motor de presença: classifica o tick (ativo/passivo/ocioso/ausente)
            // e abre/fecha marcadores de AUSÊNCIA na transição — pra timeline ter
            // dono em todo minuto. (Pausa é tratada no set_paused, não aqui.)
            let st = state::resolve_state(idle, signals::audio_active(), signals::locked(), is_paused);
            if st != prev_state {
                let prev_kind = prev_state.marker_kind();
                let new_kind = st.marker_kind();
                if prev_kind != new_kind {
                    if let Ok(conn) = db.lock() {
                        if let Some(k) = prev_kind {
                            let _ = db::close_marker(&conn, k, now);
                        }
                        if let Some(k) = new_kind {
                            // Recua ao momento aproximado em que saiu (último input).
                            let _ = db::open_marker(&conn, k, (now - idle).max(0));
                        }
                    }
                }
                let _ = app.emit("state-changed", st.as_str());
                prev_state = st;
            }

            // Pausado ou ocioso = sem foco (mas pausado NÃO é idle-trimmed).
            // Ignora "loginwindow" e cia. — tela de bloqueio não é foco real.
            let win = if is_paused || is_idle {
                None
            } else {
                provider.current().filter(|w| !is_system_noise(&w.app_name))
            };

            // Snapshot do último app REAL em foco (≠ focusbar) — é o que o
            // "checar agora" julga, já que o clique no botão foca o focusbar.
            if let Some(w) = win.as_ref() {
                if !w.app_name.eq_ignore_ascii_case("focusbar") {
                    let st = app.state::<crate::state::AppState>();
                    if let Ok(mut lw) = st.last_real_window.lock() {
                        *lw = Some(crate::state::LastRealWindow {
                            app: w.app_name.clone(),
                            title: w.title.clone(),
                            pid: w.pid,
                            ts: now,
                        });
                    };
                }
            }

            let new_key = win.as_ref().map(|w| (w.app_name.as_str(), w.title.as_str()));
            let cur_key = current
                .as_ref()
                .map(|c| (c.raw_app.as_str(), c.raw_title.as_str()));

            if new_key != cur_key {
                // Fecha a sessão anterior (fim exato; no idle, RECUA pro último input).
                if let Some(prev) = current.take() {
                    let end = if is_idle {
                        (now - idle).max(prev.start_ts)
                    } else {
                        now
                    };
                    if let Ok(conn) = db.lock() {
                        match prev.row_id {
                            // Viva no banco desde a abertura (heartbeat): só fecha.
                            // Se o idle-trim a encolheu abaixo do mínimo, descarta.
                            Some(id) => {
                                if end - prev.start_ts >= MIN_SESSION_SECS {
                                    let _ = db::finish_session(
                                        &conn, id, end, is_idle, prev.content.as_deref(),
                                    );
                                } else {
                                    let _ = db::delete_session(&conn, id);
                                }
                            }
                            // Curta demais pra ter ganhado linha: grava só se
                            // cruzou o mínimo entre a última batida e agora.
                            None if end - prev.start_ts >= MIN_SESSION_SECS => {
                                let _ = db::insert_session(
                                    &conn,
                                    prev.app_id,
                                    &prev.stored_title,
                                    prev.start_ts,
                                    end,
                                    is_idle,
                                    prev.content.as_deref(),
                                );
                            }
                            None => {}
                        }
                    }
                }

                // Abre a nova (resolve site + URL aqui, uma vez por troca).
                if let Some(w) = win.as_ref() {
                    // AppleScript pros navegadores scriptáveis; senão a EXTENSÃO
                    // (tab_feed — o caminho que funciona no Opera GX/Windows);
                    // senão a URL da barra de endereço pela Acessibilidade.
                    let url = browser::browser_url(&w.app_name)
                        .or_else(|| {
                            if browser::is_browser(&w.app_name) {
                                tab_feed.url_for(&w.app_name, &w.title, now)
                            } else {
                                None
                            }
                        })
                        .or_else(|| {
                            if browser::is_browser(&w.app_name) {
                                crate::capture::browser_url_ax(w.pid)
                            } else {
                                None
                            }
                        });
                    let (label_app, stored_title, bundle) = match &url {
                        Some(u) => (
                            browser::site_label(u).unwrap_or_else(|| w.app_name.clone()),
                            // Título sem o lixo do Chrome + URL limpa (sem query/fragment).
                            format!("{} — {}", browser::clean_browser_title(&w.title), browser::clean_url(u)),
                            String::new(), // site agrupa independente do navegador
                        ),
                        None => (
                            w.app_name.clone(),
                            w.title.clone(),
                            w.app_bundle_id.clone().unwrap_or_default(),
                        ),
                    };
                    let app_id = db
                        .lock()
                        .ok()
                        .and_then(|conn| db::get_or_create_app(&conn, &label_app, &bundle).ok());
                    if let Some(app_id) = app_id {
                        current = Some(OpenSession {
                            app_id,
                            raw_app: w.app_name.clone(),
                            raw_title: w.title.clone(),
                            label_app,
                            stored_title,
                            start_ts: now,
                            pid: w.pid,
                            content: None,
                            content_done: false,
                            row_id: None,
                        });
                    }
                }

                let _ = app.emit("focus-changed", &win);
                coach.note_switch(now);
            }

            // HEARTBEAT (Fase B): a sessão corrente vive no banco desde que se
            // firma (>= MIN_SESSION_SECS) e o fim é empurrado a cada batida.
            // Crash/kill/bateria: a última batida JÁ está gravada — o erro
            // máximo é 1 tick, nunca a sessão inteira (modelo ActivityWatch).
            if !is_paused && !is_idle {
                if let Some(cur) = current.as_mut() {
                    if let Ok(conn) = db.lock() {
                        match cur.row_id {
                            Some(id) => {
                                let _ = db::extend_session(&conn, id, now);
                            }
                            None if now - cur.start_ts >= MIN_SESSION_SECS => {
                                cur.row_id = db::open_session(
                                    &conn,
                                    cur.app_id,
                                    &cur.stored_title,
                                    cur.start_ts,
                                    now,
                                )
                                .ok();
                            }
                            None => {}
                        }
                    }
                }
            }

            // "Olhos" pra IA categorizar por CONTEÚDO: captura UMA vez por sessão,
            // só depois que ela ficou estável (>=5s — não OCRa alt-tab de 2s) e a
            // janela ainda é a mesma. AX primeiro; se vier fraco e o OCR estiver
            // ligado + com permissão, OCRa os pixels (é o que dá conteúdo no Windows).
            // Zonas de exclusão (banco/senha) NUNCA são capturadas.
            if let (Some(w), Some(cur)) = (win.as_ref(), current.as_mut()) {
                if !cur.content_done
                    && now - cur.start_ts >= CONTENT_DELAY
                    && !crate::redact::is_excluded(&w.app_name, &w.title)
                {
                    let mut text = crate::capture::focused_text(cur.pid)
                        .map(|t| crate::redact::redact(&t))
                        .unwrap_or_default();
                    // Chrome esconde a página da AX → só veio a moldura. Começa do
                    // título LIMPO (= o nome da página: vídeo, busca, artigo) como
                    // base confiável, e força o OCR pra ler o corpo de verdade.
                    let chrome_frame = crate::capture::browser::is_chrome_frame(&text);
                    if chrome_frame {
                        text = crate::capture::browser::clean_browser_title(&cur.raw_title);
                    }
                    if chrome_frame || content_is_weak(&text) {
                        let ocr_on = db
                            .lock()
                            .ok()
                            .and_then(|c| db::get_setting(&c, "ocr_enabled").ok())
                            .flatten()
                            .map(|v| v == "1")
                            .unwrap_or(false);
                        // NÃO confia no preflight de permissão (no macOS ele mente —
                        // retorna "negado" mesmo concedido). Só TENTA o OCR: se há
                        // permissão, funciona; se não, a captura falha sozinha e cai
                        // no título limpo. É o que o screenpipe faz: tenta sempre.
                        if ocr_on {
                            if let Some(rt) = ocr_rt.as_ref() {
                                // Captura pela janela do PID DESTA sessão (cur.pid),
                                // não "a em foco agora" — o block_on leva segundos e
                                // o foco pode trocar pra um app sensível no meio.
                                if let Some(t) = rt.block_on(
                                    crate::capture::screen::ocr_window_by_pid(cur.pid),
                                ) {
                                    text = crate::redact::redact(&t);
                                }
                            }
                        }
                    }
                    cur.content = Some(text.chars().take(600).collect::<String>())
                        .filter(|t: &String| t.trim().chars().count() >= 12);
                    cur.content_done = true;
                }
            }

            // Coach ao vivo (usa o nome do site + título com URL).
            let cur_tuple = current
                .as_ref()
                .map(|c| (c.label_app.as_str(), c.stored_title.as_str(), c.start_ts));
            coach.tick(now, cur_tuple, &app, &db);

            thread::sleep(Duration::from_secs(POLL_SECS));
        }
    });
}
