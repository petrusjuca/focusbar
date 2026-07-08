//! Queries de lembretes.

use crate::models::Reminder;
use rusqlite::{params, Connection};

fn map_row(r: &rusqlite::Row) -> rusqlite::Result<Reminder> {
    Ok(Reminder {
        id: r.get(0)?,
        text: r.get(1)?,
        kind: r.get(2)?,
        fire_at: r.get(3)?,
        interval_secs: r.get(4)?,
        enabled: r.get::<_, i64>(5)? != 0,
        last_fired_ts: r.get(6)?,
        created_at: r.get(7)?,
        snoozed_until: r.get(8)?,
    })
}

const COLS: &str =
    "id, text, kind, fire_at, interval_secs, enabled, last_fired_ts, created_at, snoozed_until";

pub fn create(
    conn: &Connection,
    text: &str,
    kind: &str,
    fire_at: Option<i64>,
    interval_secs: Option<i64>,
    created_at: i64,
) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO reminders(text, kind, fire_at, interval_secs, enabled, created_at)
         VALUES (?1, ?2, ?3, ?4, 1, ?5)",
        params![text, kind, fire_at, interval_secs, created_at],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn list(conn: &Connection) -> rusqlite::Result<Vec<Reminder>> {
    let sql = format!("SELECT {COLS} FROM reminders ORDER BY enabled DESC, created_at DESC");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], map_row)?;
    rows.collect()
}

/// Lembretes habilitados (usado pelo scheduler).
pub fn enabled(conn: &Connection) -> rusqlite::Result<Vec<Reminder>> {
    let sql = format!("SELECT {COLS} FROM reminders WHERE enabled = 1");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], map_row)?;
    rows.collect()
}

pub fn set_enabled(conn: &Connection, id: i64, enabled: bool) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE reminders SET enabled = ?2 WHERE id = ?1",
        params![id, enabled as i64],
    )?;
    Ok(())
}

pub fn delete(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM reminders WHERE id = ?1", params![id])?;
    Ok(())
}

/// Marca como disparado agora. Lembretes "once" também são desabilitados.
pub fn mark_fired(conn: &Connection, id: i64, now: i64, disable: bool) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE reminders SET last_fired_ts = ?2, enabled = CASE WHEN ?3 THEN 0 ELSE enabled END
         WHERE id = ?1",
        params![id, now, disable as i64],
    )?;
    Ok(())
}

/// "Chega por hoje": silencia o lembrete até `until_ts` (meia-noite local).
pub fn snooze_until(conn: &Connection, id: i64, until_ts: i64) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE reminders SET snoozed_until = ?2 WHERE id = ?1",
        params![id, until_ts],
    )?;
    Ok(())
}
