//! Camada SQLite local. Schema criado no startup (idempotente).
//! Volume é pequeno (uma linha por sessão de foco), então agregação é on-the-fly.

use crate::models::{AppTotal, FocusSession};
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashMap;
use std::path::Path;

pub mod notes;
pub mod reminders;
pub mod todos;

/// Abre (ou cria) o banco em `path`, liga WAL e roda as migrations.
pub fn open(path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    migrate(&conn)?;
    // Colunas novas em bancos já existentes (ADD COLUMN não tem IF NOT EXISTS;
    // se já existir, o erro é ignorado — efeito idempotente).
    for col in [
        "ALTER TABLE focus_events ADD COLUMN content TEXT",
        "ALTER TABLE focus_events ADD COLUMN category_ai TEXT",
        "ALTER TABLE focus_events ADD COLUMN activity_ai TEXT",
    ] {
        let _ = conn.execute(col, []);
    }
    // Limpeza: remove ruído (tela de bloqueio etc.) do histórico — não é foco real.
    let _ = conn.execute(
        "DELETE FROM focus_events WHERE app_id IN
         (SELECT id FROM apps WHERE lower(name) IN
            ('loginwindow', 'windowserver', 'screensaverengine'))",
        [],
    );
    Ok(conn)
}

/// Tolerância de continuidade (s) pra agrupar sessões cruas em "atividades".
/// Um alt-tab rápido pro WhatsApp e de volta não deve fragmentar o trabalho.
pub const GROUP_GAP_SECS: i64 = 90;

/// Agrupa sessões CRUAS (já ordenadas por start_ts ASC) em blocos de atividade:
/// sessões consecutivas do MESMO app cujo intervalo entre elas é <= `gap` viram
/// um bloco só — mata o "confete" de troca de aba/alt-tab. É camada de EXIBIÇÃO:
/// não toca no dado bruto nem no schema. A duração do bloco é a SOMA das filhas
/// (respeita o idle-trim; nunca reconta tempo de parede). start_ts/title/id ficam
/// os da primeira filha.
pub fn group_sessions(sessions: &[FocusSession], gap: i64) -> Vec<FocusSession> {
    let mut out: Vec<FocusSession> = Vec::new();
    let mut last_end: i64 = 0; // fim (start+dur) da última filha do bloco atual
    for s in sessions {
        let s_end = s.start_ts + s.duration_secs;
        if let Some(cur) = out.last_mut() {
            let gap_to = (s.start_ts - last_end).max(0);
            if cur.app_name == s.app_name && gap_to <= gap {
                cur.duration_secs += s.duration_secs;
                cur.was_idle_trimmed |= s.was_idle_trimmed;
                last_end = s_end;
                continue;
            }
        }
        out.push(s.clone());
        last_end = s_end;
    }
    out
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
            was_idle_trimmed INTEGER NOT NULL DEFAULT 0,
            content          TEXT,  -- trecho REDIGIDO do que estava na tela (pra IA entender)
            category_ai      TEXT,  -- categoria pela IA via CONTEUDO (nao pelo nome do app)
            activity_ai      TEXT   -- nome curto da atividade deduzida do conteudo
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

        -- Histórico rico de cada bloco Pomodoro (pra métricas: quantos, duração
        -- planejada vs real, se a tarefa foi concluída, horário de sucesso).
        CREATE TABLE IF NOT EXISTS pomodoro_log (
            id           INTEGER PRIMARY KEY,
            goal         TEXT NOT NULL,
            start_ts     INTEGER NOT NULL,
            planned_secs INTEGER NOT NULL,
            actual_secs  INTEGER NOT NULL,
            completed    INTEGER NOT NULL,
            ts           INTEGER NOT NULL
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

        CREATE TABLE IF NOT EXISTS daily_rollups (
            day           TEXT NOT NULL,
            app_id        INTEGER NOT NULL REFERENCES apps(id),
            total_secs    INTEGER NOT NULL,
            session_count INTEGER NOT NULL,
            PRIMARY KEY(day, app_id)
        );

        -- Marcadores de intervalo: ausência de dado REAL na timeline (todo minuto
        -- com dono). kind: 'paused' | 'away' | 'away_uncertain'. end_ts NULL = aberto.
        CREATE TABLE IF NOT EXISTS interval_markers (
            id         INTEGER PRIMARY KEY,
            kind       TEXT NOT NULL,
            start_ts   INTEGER NOT NULL,
            end_ts     INTEGER,
            created_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_marker_kind_start ON interval_markers(kind, start_ts);
        CREATE INDEX IF NOT EXISTS idx_marker_start ON interval_markers(start_ts);",
    )
}

/// Abre um marcador de intervalo (ex.: começou a pausa/ausência). Idempotente:
/// fecha qualquer marcador ABERTO do mesmo kind antes (cura close perdido por crash).
pub fn open_marker(conn: &Connection, kind: &str, now: i64) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE interval_markers SET end_ts = ?2 WHERE kind = ?1 AND end_ts IS NULL",
        params![kind, now],
    )?;
    conn.execute(
        "INSERT INTO interval_markers(kind, start_ts, end_ts, created_at)
         VALUES (?1, ?2, NULL, ?2)",
        params![kind, now],
    )?;
    Ok(())
}

/// Fecha o marcador aberto do kind (no-op inofensivo se não houver aberto).
pub fn close_marker(conn: &Connection, kind: &str, now: i64) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE interval_markers SET end_ts = ?2 WHERE kind = ?1 AND end_ts IS NULL",
        params![kind, now],
    )?;
    Ok(())
}

/// Marcadores que cruzam a janela [start, end). Abertos (end_ts NULL) entram —
/// o frontend fecha em "agora"/fim do dia.
pub fn markers_in_range(
    conn: &Connection,
    start: i64,
    end: i64,
) -> rusqlite::Result<Vec<crate::models::IntervalMarker>> {
    let mut stmt = conn.prepare(
        "SELECT id, kind, start_ts, end_ts FROM interval_markers
         WHERE start_ts < ?2 AND (end_ts IS NULL OR end_ts > ?1)
         ORDER BY start_ts ASC",
    )?;
    let rows = stmt.query_map(params![start, end], |r| {
        Ok(crate::models::IntervalMarker {
            id: r.get(0)?,
            kind: r.get(1)?,
            start_ts: r.get(2)?,
            end_ts: r.get(3)?,
        })
    })?;
    rows.collect()
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

/// Registra um bloco Pomodoro concluído (com metadados ricos pras métricas).
#[allow(clippy::too_many_arguments)]
pub fn log_pomodoro(
    conn: &Connection,
    goal: &str,
    start_ts: i64,
    planned_secs: i64,
    actual_secs: i64,
    completed: bool,
    ts: i64,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO pomodoro_log(goal, start_ts, planned_secs, actual_secs, completed, ts)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![goal, start_ts, planned_secs, actual_secs, completed as i64, ts],
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

/// Grava uma sessão de foco fechada (uma linha por troca de janela). `content` =
/// trecho redigido do que estava na tela (pra IA categorizar depois pelo conteúdo).
pub fn insert_session(
    conn: &Connection,
    app_id: i64,
    title: &str,
    start_ts: i64,
    end_ts: i64,
    idle_trimmed: bool,
    content: Option<&str>,
) -> rusqlite::Result<()> {
    let duration = end_ts - start_ts;
    conn.execute(
        "INSERT INTO focus_events(app_id, title, start_ts, end_ts, duration_secs, was_idle_trimmed, content)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![app_id, title, start_ts, end_ts, duration, idle_trimmed as i64, content],
    )?;
    Ok(())
}

/// Sessões que TÊM conteúdo capturado mas ainda NÃO foram categorizadas pela IA.
/// (id, app_name, title, content). Pra o categorizador processar em lote.
pub fn sessions_needing_category(
    conn: &Connection,
    limit: i64,
) -> rusqlite::Result<Vec<(i64, String, String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT f.rowid, a.name, f.title, f.content
         FROM focus_events f JOIN apps a ON a.id = f.app_id
         WHERE f.category_ai IS NULL AND f.content IS NOT NULL AND length(f.content) >= 12
         ORDER BY f.start_ts DESC LIMIT ?1",
    )?;
    let rows = stmt.query_map(params![limit], |r| {
        Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
    })?;
    rows.collect()
}

/// Grava a categoria + nome da atividade que a IA deduziu do conteúdo.
pub fn set_session_category(
    conn: &Connection,
    id: i64,
    category: &str,
    activity: &str,
) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE focus_events SET category_ai = ?2, activity_ai = ?3 WHERE rowid = ?1",
        params![id, category, activity],
    )?;
    Ok(())
}

/// Últimas N sessões gravadas (mais recentes primeiro). Para debug/visualização.
pub fn recent_sessions(conn: &Connection, limit: i64) -> rusqlite::Result<Vec<FocusSession>> {
    let mut stmt = conn.prepare(
        "SELECT a.name, f.title, f.start_ts, COALESCE(f.duration_secs, 0), f.was_idle_trimmed, f.rowid, f.category_ai, f.activity_ai
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
            category_ai: r.get(6)?,
            activity_ai: r.get(7)?,
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
/// Estatísticas de Pomodoro num intervalo: (qtd, soma_real, qtd_concluídos).
pub fn pomodoro_stats(conn: &Connection, start: i64, end: i64) -> rusqlite::Result<(i64, i64, i64)> {
    conn.query_row(
        "SELECT COUNT(*), COALESCE(SUM(actual_secs), 0), COALESCE(SUM(completed), 0)
         FROM pomodoro_log WHERE start_ts >= ?1 AND start_ts < ?2",
        params![start, end],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )
}

/// Soma dos segundos de marcadores de um kind que cruzam [start, end). Marcador
/// aberto (end_ts NULL) é fechado em `now`. Aproximado (pro painel/debug).
pub fn marker_secs(
    conn: &Connection,
    kinds: &[&str],
    start: i64,
    end: i64,
    now: i64,
) -> rusqlite::Result<i64> {
    let mut total = 0i64;
    for m in markers_in_range(conn, start, end)? {
        if kinds.contains(&m.kind.as_str()) {
            let s = m.start_ts.max(start);
            let e = m.end_ts.unwrap_or(now).min(end);
            total += (e - s).max(0);
        }
    }
    Ok(total)
}

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
        "SELECT a.name, f.title, f.start_ts, COALESCE(f.duration_secs, 0), f.was_idle_trimmed, f.rowid, f.category_ai, f.activity_ai
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
            category_ai: r.get(6)?,
            activity_ai: r.get(7)?,
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
        "SELECT a.name, f.title, f.start_ts, COALESCE(f.duration_secs, 0), f.was_idle_trimmed, f.rowid, f.category_ai, f.activity_ai
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
                category_ai: r.get(6)?,
                activity_ai: r.get(7)?,
            })
        },
    )
    .optional()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sess(id: i64, app: &str, start: i64, dur: i64) -> FocusSession {
        FocusSession {
            id,
            app_name: app.into(),
            title: format!("t{id}"),
            start_ts: start,
            duration_secs: dur,
            was_idle_trimmed: false,
            category_ai: None,
            activity_ai: None,
        }
    }

    #[test]
    fn group_empty_is_empty() {
        assert!(group_sessions(&[], GROUP_GAP_SECS).is_empty());
    }

    #[test]
    fn group_collapses_tab_hopping_same_app() {
        // 3 sessões "Opera" coladas (gap 0) → 1 bloco de 180s.
        let raw = vec![
            sess(1, "Opera", 0, 60),
            sess(2, "Opera", 60, 60),
            sess(3, "Opera", 120, 60),
        ];
        let g = group_sessions(&raw, GROUP_GAP_SECS);
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].duration_secs, 180);
        assert_eq!(g[0].id, 1); // mantém a primeira filha
        assert_eq!(g[0].start_ts, 0);
    }

    #[test]
    fn group_tolerates_short_gap() {
        // alt-tab de 30s pro WhatsApp e volta — gap < 90s → mesmo bloco.
        let raw = vec![sess(1, "Opera", 0, 60), sess(2, "Opera", 90, 60)];
        let g = group_sessions(&raw, GROUP_GAP_SECS);
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].duration_secs, 120);
    }

    #[test]
    fn group_splits_on_long_gap() {
        // gap de 120s (> 90) → dois blocos.
        let raw = vec![sess(1, "Opera", 0, 60), sess(2, "Opera", 180, 60)];
        let g = group_sessions(&raw, GROUP_GAP_SECS);
        assert_eq!(g.len(), 2);
    }

    #[test]
    fn group_splits_on_app_switch() {
        let raw = vec![sess(1, "Opera", 0, 60), sess(2, "Code", 60, 60)];
        let g = group_sessions(&raw, GROUP_GAP_SECS);
        assert_eq!(g.len(), 2);
    }

    fn mem() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        migrate(&c).unwrap();
        c
    }

    #[test]
    fn marker_open_close() {
        let c = mem();
        open_marker(&c, "paused", 1000).unwrap();
        let open = markers_in_range(&c, 0, 9999).unwrap();
        assert_eq!(open.len(), 1);
        assert_eq!(open[0].end_ts, None); // aberto
        close_marker(&c, "paused", 1500).unwrap();
        let closed = markers_in_range(&c, 0, 9999).unwrap();
        assert_eq!(closed[0].end_ts, Some(1500));
    }

    #[test]
    fn marker_double_open_closes_first() {
        let c = mem();
        open_marker(&c, "away", 100).unwrap();
        open_marker(&c, "away", 200).unwrap(); // deve fechar o 1º
        let all = markers_in_range(&c, 0, 9999).unwrap();
        let abertos = all.iter().filter(|m| m.end_ts.is_none()).count();
        assert_eq!(abertos, 1); // só um aberto por kind
    }

    #[test]
    fn marker_range_overlap() {
        let c = mem();
        open_marker(&c, "paused", 100).unwrap();
        close_marker(&c, "paused", 200).unwrap();
        // janela que não cruza → vazio
        assert_eq!(markers_in_range(&c, 300, 400).unwrap().len(), 0);
        // janela que cruza → 1
        assert_eq!(markers_in_range(&c, 150, 400).unwrap().len(), 1);
    }
}
