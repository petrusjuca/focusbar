//! Notas/intenções do dia (ritual diário + captura rápida).
//! kind: "intention" (o que quero fazer hoje) ou "note" (anotação rápida).

use crate::models::Note;
use rusqlite::{params, Connection};

pub fn add(
    conn: &Connection,
    day: &str,
    kind: &str,
    text: &str,
    created_at: i64,
) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO notes(day, kind, text, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![day, kind, text, created_at],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn for_day(conn: &Connection, day: &str) -> rusqlite::Result<Vec<Note>> {
    let mut stmt = conn.prepare(
        "SELECT id, day, kind, text, created_at FROM notes
         WHERE day = ?1 ORDER BY created_at ASC",
    )?;
    let rows = stmt.query_map(params![day], |r| {
        Ok(Note {
            id: r.get(0)?,
            day: r.get(1)?,
            kind: r.get(2)?,
            text: r.get(3)?,
            created_at: r.get(4)?,
        })
    })?;
    rows.collect()
}

pub fn delete(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM notes WHERE id = ?1", params![id])?;
    Ok(())
}

/// Só os textos das intenções do dia (pro assistente comparar com a execução).
pub fn intentions_for_day(conn: &Connection, day: &str) -> rusqlite::Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT text FROM notes WHERE day = ?1 AND kind = 'intention' ORDER BY created_at ASC",
    )?;
    let rows = stmt.query_map(params![day], |r| r.get::<_, String>(0))?;
    rows.collect()
}
