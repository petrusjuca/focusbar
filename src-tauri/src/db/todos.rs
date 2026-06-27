//! Lista de tarefas (to-do): algo a fazer, marca como feito.

use crate::models::Todo;
use rusqlite::{params, Connection};

pub fn add(conn: &Connection, text: &str, created_at: i64) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO todos(text, done, created_at) VALUES (?1, 0, ?2)",
        params![text, created_at],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Abertas primeiro (mais novas no topo); feitas depois (mais recentes no topo).
pub fn list(conn: &Connection) -> rusqlite::Result<Vec<Todo>> {
    let mut stmt = conn.prepare(
        "SELECT id, text, done, created_at, done_at FROM todos
         ORDER BY done ASC, COALESCE(done_at, created_at) DESC",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(Todo {
            id: r.get(0)?,
            text: r.get(1)?,
            done: r.get::<_, i64>(2)? != 0,
            created_at: r.get(3)?,
            done_at: r.get(4)?,
        })
    })?;
    rows.collect()
}

/// Alterna feito/não-feito, registrando done_at.
pub fn toggle(conn: &Connection, id: i64, now: i64) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE todos
         SET done = CASE WHEN done = 1 THEN 0 ELSE 1 END,
             done_at = CASE WHEN done = 1 THEN NULL ELSE ?2 END
         WHERE id = ?1",
        params![id, now],
    )?;
    Ok(())
}

pub fn delete(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM todos WHERE id = ?1", params![id])?;
    Ok(())
}

/// Marca como feita a tarefa ABERTA mais recente cujo texto bate (sem caixa/espaços).
/// Usado quando você termina um bloco de foco ligado a uma tarefa — ela sai da
/// lista sozinha, sem precisar ir remover na mão.
pub fn complete_by_text(conn: &Connection, text: &str, now: i64) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE todos SET done = 1, done_at = ?2
         WHERE id = (
            SELECT id FROM todos
            WHERE done = 0 AND lower(trim(text)) = lower(trim(?1))
            ORDER BY created_at DESC LIMIT 1
         )",
        params![text, now],
    )?;
    Ok(())
}
