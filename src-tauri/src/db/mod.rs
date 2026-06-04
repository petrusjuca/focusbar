//! Camada SQLite local. Schema criado no startup (idempotente).
//! Volume é pequeno (uma linha por sessão de foco), então agregação é on-the-fly.

use crate::models::{AppTotal, FocusSession};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

pub mod reminders;
pub mod tasks;

/// Abre (ou cria) o banco em `path`, liga WAL e roda as migrations.
pub fn open(path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    migrate(&conn)?;
    Ok(conn)
}

fn migrate(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS apps (
            id        INTEGER PRIMARY KEY,
            name      TEXT NOT NULL,
            bundle_id TEXT NOT NULL DEFAULT '',
            category  TEXT,
            UNIQUE(name, bundle_id)
        );

        CREATE TABLE IF NOT EXISTS focus_events (
            id               INTEGER PRIMARY KEY,
            app_id           INTEGER NOT NULL REFERENCES apps(id),
            title            TEXT NOT NULL,
            start_ts         INTEGER NOT NULL,
            end_ts           INTEGER,
            duration_secs    INTEGER,
            was_idle_trimmed INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_focus_start ON focus_events(start_ts);
        CREATE INDEX IF NOT EXISTS idx_focus_app   ON focus_events(app_id, start_ts);

        CREATE TABLE IF NOT EXISTS reminders (
            id            INTEGER PRIMARY KEY,
            text          TEXT NOT NULL,
            kind          TEXT NOT NULL,
            fire_at       INTEGER,
            interval_secs INTEGER,
            cron          TEXT,
            enabled       INTEGER NOT NULL DEFAULT 1,
            last_fired_ts INTEGER,
            created_at    INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS task_rules (
            id        INTEGER PRIMARY KEY,
            keyword   TEXT NOT NULL UNIQUE,
            task_name TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS daily_rollups (
            day           TEXT NOT NULL,
            app_id        INTEGER NOT NULL REFERENCES apps(id),
            total_secs    INTEGER NOT NULL,
            session_count INTEGER NOT NULL,
            PRIMARY KEY(day, app_id)
        );",
    )
}

/// Resolve o id do app (cria se não existir). bundle vazio = "" para casar UNIQUE.
pub fn get_or_create_app(conn: &Connection, name: &str, bundle: &str) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT OR IGNORE INTO apps(name, bundle_id) VALUES (?1, ?2)",
        params![name, bundle],
    )?;
    conn.query_row(
        "SELECT id FROM apps WHERE name = ?1 AND bundle_id = ?2",
        params![name, bundle],
        |r| r.get(0),
    )
}

/// Grava uma sessão de foco fechada (uma linha por troca de janela).
pub fn insert_session(
    conn: &Connection,
    app_id: i64,
    title: &str,
    start_ts: i64,
    end_ts: i64,
    idle_trimmed: bool,
) -> rusqlite::Result<()> {
    let duration = end_ts - start_ts;
    conn.execute(
        "INSERT INTO focus_events(app_id, title, start_ts, end_ts, duration_secs, was_idle_trimmed)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![app_id, title, start_ts, end_ts, duration, idle_trimmed as i64],
    )?;
    Ok(())
}

/// Últimas N sessões gravadas (mais recentes primeiro). Para debug/visualização.
pub fn recent_sessions(conn: &Connection, limit: i64) -> rusqlite::Result<Vec<FocusSession>> {
    let mut stmt = conn.prepare(
        "SELECT a.name, f.title, f.start_ts, COALESCE(f.duration_secs, 0), f.was_idle_trimmed
         FROM focus_events f
         JOIN apps a ON a.id = f.app_id
         ORDER BY f.start_ts DESC
         LIMIT ?1",
    )?;
    let rows = stmt.query_map(params![limit], |r| {
        Ok(FocusSession {
            app_name: r.get(0)?,
            title: r.get(1)?,
            start_ts: r.get(2)?,
            duration_secs: r.get(3)?,
            was_idle_trimmed: r.get::<_, i64>(4)? != 0,
        })
    })?;
    rows.collect()
}

/// Tempo total por app no intervalo [start_ts, end_ts), do maior pro menor.
pub fn app_totals(conn: &Connection, start: i64, end: i64) -> rusqlite::Result<Vec<AppTotal>> {
    let mut stmt = conn.prepare(
        "SELECT a.name, SUM(COALESCE(f.duration_secs, 0)) AS tot, COUNT(*) AS c
         FROM focus_events f
         JOIN apps a ON a.id = f.app_id
         WHERE f.start_ts >= ?1 AND f.start_ts < ?2
         GROUP BY a.id
         ORDER BY tot DESC",
    )?;
    let rows = stmt.query_map(params![start, end], |r| {
        Ok(AppTotal {
            app_name: r.get(0)?,
            total_secs: r.get(1)?,
            session_count: r.get(2)?,
        })
    })?;
    rows.collect()
}

/// Tempo total de foco no intervalo [start, end).
pub fn total_in_range(conn: &Connection, start: i64, end: i64) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT COALESCE(SUM(duration_secs), 0)
         FROM focus_events
         WHERE start_ts >= ?1 AND start_ts < ?2",
        params![start, end],
        |r| r.get(0),
    )
}

/// Sessões no intervalo, em ordem cronológica (para a timeline do dia).
pub fn sessions_in_range(
    conn: &Connection,
    start: i64,
    end: i64,
) -> rusqlite::Result<Vec<FocusSession>> {
    let mut stmt = conn.prepare(
        "SELECT a.name, f.title, f.start_ts, COALESCE(f.duration_secs, 0), f.was_idle_trimmed
         FROM focus_events f
         JOIN apps a ON a.id = f.app_id
         WHERE f.start_ts >= ?1 AND f.start_ts < ?2
         ORDER BY f.start_ts ASC",
    )?;
    let rows = stmt.query_map(params![start, end], |r| {
        Ok(FocusSession {
            app_name: r.get(0)?,
            title: r.get(1)?,
            start_ts: r.get(2)?,
            duration_secs: r.get(3)?,
            was_idle_trimmed: r.get::<_, i64>(4)? != 0,
        })
    })?;
    rows.collect()
}

/// A sessão de foco mais longa no intervalo (o "pico de foco" do dia).
pub fn longest_session(
    conn: &Connection,
    start: i64,
    end: i64,
) -> rusqlite::Result<Option<FocusSession>> {
    conn.query_row(
        "SELECT a.name, f.title, f.start_ts, COALESCE(f.duration_secs, 0), f.was_idle_trimmed
         FROM focus_events f
         JOIN apps a ON a.id = f.app_id
         WHERE f.start_ts >= ?1 AND f.start_ts < ?2
         ORDER BY f.duration_secs DESC
         LIMIT 1",
        params![start, end],
        |r| {
            Ok(FocusSession {
                app_name: r.get(0)?,
                title: r.get(1)?,
                start_ts: r.get(2)?,
                duration_secs: r.get(3)?,
                was_idle_trimmed: r.get::<_, i64>(4)? != 0,
            })
        },
    )
    .optional()
}
