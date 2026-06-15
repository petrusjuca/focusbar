use rusqlite::Connection;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

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
}
