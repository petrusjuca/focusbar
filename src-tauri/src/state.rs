use rusqlite::Connection;
use std::sync::{Arc, Mutex};

/// Estado global gerenciado pelo Tauri. A conexão é compartilhada entre os
/// commands (leitura) e o sampler em background (escrita) via Arc<Mutex<>>.
pub struct AppState {
    pub db: Arc<Mutex<Connection>>,
}
