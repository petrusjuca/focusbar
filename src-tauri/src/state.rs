use rusqlite::Connection;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

/// Resultado de uma checagem de foco, em cache. Enquanto a janela em foco não
/// muda, reusamos isto em vez de reler AX/OCR e re-acionar o Ollama a cada 30s —
/// é o que evita o modelo ficar "quente" pra sempre, drenando bateria.
#[derive(Clone)]
pub struct CachedCheck {
    pub key: String, // foco|app|título
    pub focus: String,
    pub app: String,
    pub on_task: Option<bool>,
    pub reason: String,
    pub source: String,
    pub ts: i64,
}

/// Último app REAL (≠ focusbar) que o sampler viu em foco. Existe por causa do
/// "checar agora": clicar no botão foca o PRÓPRIO focusbar, e julgar isso seria
/// inútil — o juiz usa este snapshot pra julgar o que você fazia ANTES do clique.
#[derive(Clone)]
pub struct LastRealWindow {
    pub app: String,
    pub title: String, // cru — a limpeza acontece em quem consome
    pub pid: i32,
    pub ts: i64,
}

/// Estado global gerenciado pelo Tauri. A conexão é compartilhada entre os
/// commands (leitura) e o sampler em background (escrita) via Arc<Mutex<>>.
/// `paused` desliga o rastreamento sem fechar o app (sem contar como nada).
pub struct AppState {
    /// IMPORTANTE: este Mutex é `std` (não async). O guard NUNCA pode cruzar um
    /// `.await` — senão o sampler em background fica travado esperando o lock.
    /// Em comandos async, leia o que precisa num bloco `{}` e solte o lock antes
    /// de qualquer await (ver commands/focus.rs). O lint `await_holding_lock`
    /// (em lib.rs) protege isso sob `cargo clippy`.
    pub db: Arc<Mutex<Connection>>,
    pub paused: Arc<AtomicBool>,
    /// Última checagem de foco (cache por janela) — ver `CachedCheck`.
    pub last_check: Mutex<Option<CachedCheck>>,
    /// Último app real em foco (≠ focusbar) — ver `LastRealWindow`.
    pub last_real_window: Mutex<Option<LastRealWindow>>,
}
