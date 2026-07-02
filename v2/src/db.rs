//! Banco do v2. A tabela `events` é a BRUTA: append-only, nunca editada — é a
//! verdade. Sessões NÃO são gravadas: são derivadas da bruta sob demanda
//! (derive.rs), então mudar a lógica re-processa o histórico inteiro de graça.
//! (Cache materializado de sessões vem depois, SE a performance pedir.)

use crate::derive::RawEvent;
use rusqlite::{params, Connection};
use std::path::PathBuf;

pub fn db_path() -> PathBuf {
    if let Ok(p) = std::env::var("FOCUSBAR_V2_DB") {
        return PathBuf::from(p);
    }
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".focusbar").join("v2.db")
}

pub fn open() -> rusqlite::Result<Connection> {
    let path = db_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    migrate(&conn)?;
    Ok(conn)
}

pub fn migrate(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS events (
            id      INTEGER PRIMARY KEY,
            ts_ms   INTEGER NOT NULL,
            kind    TEXT NOT NULL,           -- foreground | tab | heartbeat | state | ui | pomodoro
            app     TEXT NOT NULL DEFAULT '',
            title   TEXT NOT NULL DEFAULT '',
            url     TEXT,
            tab_id  TEXT,
            pid     INTEGER,
            source  TEXT NOT NULL DEFAULT '', -- poll | event | extensao
            payload TEXT                       -- JSON livre (estado, clique de ui, etc.)
         );
         CREATE INDEX IF NOT EXISTS idx_events_ts ON events(ts_ms);
         CREATE TABLE IF NOT EXISTS settings (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
         );",
    )
}

pub struct NewEvent<'a> {
    pub ts_ms: i64,
    pub kind: &'a str,
    pub app: &'a str,
    pub title: &'a str,
    pub url: Option<&'a str>,
    pub tab_id: Option<&'a str>,
    pub pid: Option<i64>,
    pub source: &'a str,
    pub payload: Option<&'a str>,
}

pub fn append(conn: &Connection, e: &NewEvent) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO events(ts_ms, kind, app, title, url, tab_id, pid, source, payload)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
        params![e.ts_ms, e.kind, e.app, e.title, e.url, e.tab_id, e.pid, e.source, e.payload],
    )?;
    Ok(())
}

/// Eventos relevantes pro derivador num intervalo [start, end), ordenados.
pub fn events_in_range(conn: &Connection, start_ms: i64, end_ms: i64) -> rusqlite::Result<Vec<RawEvent>> {
    let mut stmt = conn.prepare(
        "SELECT ts_ms, kind, app, title, url, tab_id FROM events
         WHERE ts_ms >= ?1 AND ts_ms < ?2 AND kind IN ('foreground','tab','heartbeat')
         ORDER BY ts_ms ASC",
    )?;
    let rows = stmt.query_map(params![start_ms, end_ms], |r| {
        Ok(RawEvent {
            ts_ms: r.get(0)?,
            kind: r.get(1)?,
            app: r.get(2)?,
            title: r.get(3)?,
            url: r.get(4)?,
            tab_id: r.get(5)?,
        })
    })?;
    rows.collect()
}

pub fn event_count(conn: &Connection) -> rusqlite::Result<i64> {
    conn.query_row("SELECT COUNT(*) FROM events", [], |r| r.get(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_e_le_de_volta() {
        let c = Connection::open_in_memory().unwrap();
        migrate(&c).unwrap();
        append(&c, &NewEvent {
            ts_ms: 1000, kind: "foreground", app: "Code", title: "main.rs",
            url: None, tab_id: None, pid: Some(42), source: "poll", payload: None,
        }).unwrap();
        append(&c, &NewEvent {
            ts_ms: 2000, kind: "tab", app: "Chrome", title: "",
            url: Some("https://claude.ai"), tab_id: Some("t1"), pid: None,
            source: "extensao", payload: None,
        }).unwrap();
        let evs = events_in_range(&c, 0, 10_000).unwrap();
        assert_eq!(evs.len(), 2);
        assert_eq!(evs[0].app, "Code");
        assert_eq!(evs[1].tab_id.as_deref(), Some("t1"));
        assert_eq!(event_count(&c).unwrap(), 2);
    }
}
