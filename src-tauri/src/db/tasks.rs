//! Regras de task (keyword -> nome da task) e matching.

use crate::models::TaskRule;
use rusqlite::{params, Connection};

/// Regras ordenadas por keyword mais longa primeiro (mais específica vence).
pub fn list(conn: &Connection) -> rusqlite::Result<Vec<TaskRule>> {
    let mut stmt = conn.prepare(
        "SELECT id, keyword, task_name FROM task_rules ORDER BY length(keyword) DESC",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(TaskRule {
            id: r.get(0)?,
            keyword: r.get(1)?,
            task_name: r.get(2)?,
        })
    })?;
    rows.collect()
}

pub fn create(conn: &Connection, keyword: &str, task_name: &str) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO task_rules(keyword, task_name) VALUES (?1, ?2)
         ON CONFLICT(keyword) DO UPDATE SET task_name = excluded.task_name",
        params![keyword.to_lowercase(), task_name],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn delete(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM task_rules WHERE id = ?1", params![id])?;
    Ok(())
}

/// Casa uma sessão (app + título) contra as regras. None se nenhuma bate.
pub fn match_task<'a>(rules: &'a [TaskRule], app: &str, title: &str) -> Option<&'a str> {
    let hay = format!("{} {}", app, title).to_lowercase();
    rules
        .iter()
        .find(|r| hay.contains(&r.keyword))
        .map(|r| r.task_name.as_str())
}
