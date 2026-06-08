use rusqlite::Connection;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

/// Estado global gerenciado pelo Tauri. A conexão é compartilhada entre os
/// commands (leitura) e o sampler em background (escrita) via Arc<Mutex<>>.
/// `paused` desliga o rastreamento sem fechar o app (sem contar como nada).
pub struct AppState {
    pub db: Arc<Mutex<Connection>>,
    pub paused: Arc<AtomicBool>,
}
