//! Camada SQLite local. Schema criado no startup (idempotente).
//! Volume é pequeno (uma linha por sessão de foco), então agregação é on-the-fly.

use crate::models::{AppTotal, FocusSession};
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashMap;
use std::path::Path;

pub mod notes;
pub mod reminders;
pub mod tasks;
pub mod todos;

/// Abre (ou cria) o banco em `path`, liga WAL e roda as migrations.
pub fn open(path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    migrate(&conn)?;
    // Limpeza: remove ruído (tela de bloqueio etc.) do histórico — não é foco real.
    let _ = conn.execute(
        "DELETE FROM focus_events WHERE app_id IN
         (SELECT id FROM apps WHERE lower(name) IN
            ('loginwindow', 'windowserver', 'screensaverengine'))",
        [],
    );
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

        CREATE TABLE IF NOT EXISTS settings (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        -- OCR (Estágio 2 dos olhos) LIGADO por padrão: dá visão de conteúdo no
        -- Windows (sem AX) e reforça no Mac. Idempotente — nunca sobrescreve a
        -- escolha do usuário (só semeia o default na 1ª criação).
        INSERT OR IGNORE INTO settings(key, value) VALUES ('ocr_enabled', '1');

        CREATE TABLE IF NOT EXISTS focus_log (
            id   INTEGER PRIMARY KEY,
            goal TEXT NOT NULL,
            secs INTEGER NOT NULL,
            ts   INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS focus_now (
            id     INTEGER PRIMARY KEY CHECK (id = 1),
            text   TEXT NOT NULL,
            set_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS focus_rules (
            id      INTEGER PRIMARY KEY,
            focus   TEXT NOT NULL,
            app     TEXT NOT NULL,
            on_task INTEGER NOT NULL,
            UNIQUE(focus, app)
        );

        CREATE TABLE IF NOT EXISTS todos (
            id         INTEGER PRIMARY KEY,
            text       TEXT NOT NULL,
            done       INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL,
            done_at    INTEGER
        );

        CREATE TABLE IF NOT EXISTS notes (
            id         INTEGER PRIMARY KEY,
            day        TEXT NOT NULL,
            kind       TEXT NOT NULL,
            text       TEXT NOT NULL,
            created_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_notes_day ON notes(day);

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

/// Define o "foco atual" (no que a pessoa está tentando trabalhar agora).
pub fn set_focus(conn: &Connection, text: &str, now: i64) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO focus_now(id, text, set_at) VALUES (1, ?1, ?2)
         ON CONFLICT(id) DO UPDATE SET text = excluded.text, set_at = excluded.set_at",
        params![text, now],
    )?;
    Ok(())
}

/// Foco atual, se houver.
pub fn get_focus(conn: &Connection) -> rusqlite::Result<Option<String>> {
    conn.query_row("SELECT text FROM focus_now WHERE id = 1", [], |r| r.get(0))
        .optional()
}

/// Limpa o foco atual.
pub fn clear_focus(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM focus_now WHERE id = 1", [])?;
    Ok(())
}

/// Salva a correção do usuário: para este foco, este app ajuda (true) ou distrai (false).
pub fn set_focus_rule(
    conn: &Connection,
    focus: &str,
    app: &str,
    on_task: bool,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO focus_rules(focus, app, on_task) VALUES (?1, ?2, ?3)
         ON CONFLICT(focus, app) DO UPDATE SET on_task = excluded.on_task",
        params![focus, app, on_task as i64],
    )?;
    Ok(())
}

/// Correção salva para (foco, app), se houver.
pub fn get_focus_rule(
    conn: &Connection,
    focus: &str,
    app: &str,
) -> rusqlite::Result<Option<bool>> {
    conn.query_row(
        "SELECT on_task FROM focus_rules WHERE focus = ?1 AND app = ?2",
        params![focus, app],
        |r| r.get::<_, i64>(0),
    )
    .optional()
    .map(|o| o.map(|v| v != 0))
}

/// Define (ou limpa, se vazio) a categoria manual de um app/site, por nome.
/// Lê uma configuração local (key-value). Tudo fica só na máquina.
pub fn get_setting(conn: &Connection, key: &str) -> rusqlite::Result<Option<String>> {
    conn.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        params![key],
        |r| r.get(0),
    )
    .optional()
}

/// Grava uma configuração local.
pub fn set_setting(conn: &Connection, key: &str, value: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO settings(key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

/// Registra tempo de foco dedicado a um objetivo/tarefa (ex.: fim de um bloco).
pub fn log_focus_time(conn: &Connection, goal: &str, secs: i64, ts: i64) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO focus_log(goal, secs, ts) VALUES (?1, ?2, ?3)",
        params![goal, secs, ts],
    )?;
    Ok(())
}

/// Tempo dedicado por objetivo no intervalo [start, end), do maior pro menor.
pub fn focus_time_by_goal(
    conn: &Connection,
    start: i64,
    end: i64,
) -> rusqlite::Result<Vec<crate::models::GoalTime>> {
    let mut stmt = conn.prepare(
        "SELECT goal, SUM(secs) AS tot FROM focus_log
         WHERE ts >= ?1 AND ts < ?2 AND goal <> ''
         GROUP BY goal ORDER BY tot DESC",
    )?;
    let rows = stmt.query_map(params![start, end], |r| {
        Ok(crate::models::GoalTime {
            goal: r.get(0)?,
            secs: r.get(1)?,
        })
    })?;
    rows.collect()
}

pub fn set_app_category(conn: &Connection, name: &str, category: &str) -> rusqlite::Result<()> {
    if category.trim().is_empty() {
        conn.execute("UPDATE apps SET category = NULL WHERE name = ?1", params![name])?;
    } else {
        conn.execute(
            "UPDATE apps SET category = ?1 WHERE name = ?2",
            params![category, name],
        )?;
    }
    Ok(())
}

/// Mapa nome→categoria das categorias manuais (overrides do usuário).
pub fn category_overrides(conn: &Connection) -> rusqlite::Result<HashMap<String, String>> {
    let mut stmt = conn
        .prepare("SELECT name, category FROM apps WHERE category IS NOT NULL AND category != ''")?;
    let rows = stmt.query_map([], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
    })?;
    rows.collect()
}

/// Categoria manual de um app específico (se houver).
pub fn app_category(conn: &Connection, name: &str) -> rusqlite::Result<Option<String>> {
    conn.query_row(
        "SELECT category FROM apps WHERE name = ?1 AND category IS NOT NULL AND category != '' LIMIT 1",
        params![name],
        |r| r.get(0),
    )
    .optional()
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
        "SELECT a.name, f.title, f.start_ts, COALESCE(f.duration_secs, 0), f.was_idle_trimmed, f.rowid
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
            id: r.get(5)?,
        })
    })?;
    rows.collect()
}

/// Apaga UMA sessão específica (pelo rowid). O usuário escolhe o que remover —
/// ex.: navegação sigilosa/anônima que não quer no histórico.
pub fn delete_session(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM focus_events WHERE rowid = ?1", params![id])?;
    Ok(())
}

/// Apaga TODAS as sessões de um app/site (pelo nome exibido). Retorna quantas.
pub fn delete_app_sessions(conn: &Connection, app_name: &str) -> rusqlite::Result<usize> {
    let n = conn.execute(
        "DELETE FROM focus_events WHERE app_id IN (SELECT id FROM apps WHERE name = ?1)",
        params![app_name],
    )?;
    Ok(n)
}

/// Tempo total por app no intervalo [start_ts, end_ts), do maior pro menor.
pub fn app_totals(conn: &Connection, start: i64, end: i64) -> rusqlite::Result<Vec<AppTotal>> {
    let mut stmt = conn.prepare(
        "SELECT a.name, SUM(COALESCE(f.duration_secs, 0)) AS tot, COUNT(*) AS c, a.category
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
            category: r.get::<_, Option<String>>(3)?.unwrap_or_default(),
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
        "SELECT a.name, f.title, f.start_ts, COALESCE(f.duration_secs, 0), f.was_idle_trimmed, f.rowid
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
            id: r.get(5)?,
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
        "SELECT a.name, f.title, f.start_ts, COALESCE(f.duration_secs, 0), f.was_idle_trimmed, f.rowid
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
                id: r.get(5)?,
            })
        },
    )
    .optional()
}
