//! focusbar v2 — core local.
//! Roda como processo simples (sem janela): captura → tabela bruta → API em
//! http://127.0.0.1:7690 (UI no navegador). Empacotar em app é a ÚLTIMA fase.

mod api;
mod capture;
mod db;
mod derive;

use std::sync::{Arc, Mutex};

fn main() {
    let conn = db::open().expect("falha ao abrir o banco (~/.focusbar/v2.db)");
    println!("banco: {}", db::db_path().display());
    let db = Arc::new(Mutex::new(conn));

    capture::spawn(db.clone());
    api::serve(db); // bloqueia — Ctrl+C encerra
}
