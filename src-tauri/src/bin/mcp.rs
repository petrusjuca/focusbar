//! Servidor MCP local do focusbar.
//!
//! Fala JSON-RPC 2.0 por stdio (uma mensagem por linha) e expõe ferramentas
//! sobre o mesmo SQLite que o app grava. É assim que o Claude (Desktop ou
//! Code) lê seus dados de foco SEM o app gastar API paga: você traz o seu
//! Claude, o servidor entrega os números.
//!
//! Escrita: SÓ a ferramenta `corrigir_categoria`, e só na coluna de categoria
//! dos blocos (o veredito do fim do dia volta pro app — decisão D2). Todo o
//! resto é read-only por construção (conexão aberta READ_ONLY).
//!
//! Configurar no Claude Desktop (claude_desktop_config.json):
//!   "focusbar": { "command": "/caminho/para/mcp" }
//! ou no Claude Code:
//!   claude mcp add focusbar -- /caminho/para/mcp

use std::io::{BufRead, Write};
use std::path::PathBuf;

use chrono::{Duration, Local, NaiveDate, TimeZone};
use focusbar_lib::category::effective;
use focusbar_lib::db;
use rusqlite::{Connection, OpenFlags};
use serde_json::{json, Value};

const PROTOCOL_VERSION: &str = "2024-11-05";
const APP_ID: &str = "com.petrusjuca.focusbar";

fn main() {
    let conn = match open_db() {
        Ok(c) => c,
        Err(e) => {
            // Sem banco não dá pra servir nada — loga no stderr e sai.
            eprintln!("focusbar-mcp: nao consegui abrir o banco: {e}");
            std::process::exit(1);
        }
    };

    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    // Lê por bytes (read_until) + from_utf8_lossy: UTF-8 inválido vira caractere
    // de substituição e a linha é só descartada — NÃO derruba o servidor (o
    // `lines()` antigo dava Err e a gente quebrava o loop à toa).
    let mut reader = stdin.lock();
    let mut raw: Vec<u8> = Vec::new();
    loop {
        raw.clear();
        match reader.read_until(b'\n', &mut raw) {
            Ok(0) => break,  // EOF: stdin fechado
            Ok(_) => {}
            Err(_) => break, // erro de I/O real (pipe quebrado) → encerra
        }
        let line = String::from_utf8_lossy(&raw);
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let req: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue, // ignora lixo
        };
        let id = req.get("id").cloned();
        let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");

        // Notificações (sem id) não recebem resposta.
        if id.is_none() {
            continue;
        }
        let id = id.unwrap();

        let response = match method {
            "initialize" => ok(id, initialize_result()),
            "tools/list" => ok(id, json!({ "tools": tools_list() })),
            "ping" => ok(id, json!({})),
            "tools/call" => match call_tool(&conn, &req) {
                Ok(text) => ok(id, tool_text(&text)),
                Err(msg) => ok(id, tool_error(&msg)),
            },
            _ => err(id, -32601, "metodo nao suportado"),
        };

        if writeln!(out, "{response}").is_err() {
            break;
        }
        let _ = out.flush();
    }
}

// ---------- banco ----------

fn open_db() -> rusqlite::Result<Connection> {
    let path = db_path();
    // Read-only: o servidor jamais escreve. NO_MUTEX porque é single-thread.
    Connection::open_with_flags(
        &path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
}

/// Onde o app guarda o banco (igual ao app_data_dir do Tauri). Dá pra
/// sobrescrever com a env FOCUSBAR_DB (útil em teste/dev).
fn db_path() -> PathBuf {
    if let Ok(p) = std::env::var("FOCUSBAR_DB") {
        return PathBuf::from(p);
    }
    #[cfg(target_os = "macos")]
    let base = home().join("Library").join("Application Support");
    #[cfg(target_os = "windows")]
    let base = std::env::var("APPDATA").map(PathBuf::from).unwrap_or_else(|_| home());
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let base = home().join(".local").join("share");
    base.join(APP_ID).join("focusbar.db")
}

fn home() -> PathBuf {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

// ---------- protocolo MCP ----------

fn initialize_result() -> Value {
    json!({
        "protocolVersion": PROTOCOL_VERSION,
        "capabilities": { "tools": {} },
        "serverInfo": { "name": "focusbar", "version": env!("CARGO_PKG_VERSION") }
    })
}

fn tools_list() -> Value {
    let dia_prop = json!({
        "dia": {
            "type": "string",
            "description": "Dia no formato AAAA-MM-DD. Omita para hoje."
        }
    });
    json!([
        {
            "name": "resumo_do_dia",
            "description": "Tempo total de foco, top apps e quebra por categoria de um dia (padrao: hoje).",
            "inputSchema": { "type": "object", "properties": dia_prop }
        },
        {
            "name": "blocos_do_dia",
            "description": "Lista cronologica dos blocos de atividade do dia: horario, app, atividade, categoria e duracao.",
            "inputSchema": { "type": "object", "properties": dia_prop }
        },
        {
            "name": "pomodoros_do_dia",
            "description": "Quantos pomodoros, quanto tempo focado e quantos concluidos num dia (padrao: hoje).",
            "inputSchema": { "type": "object", "properties": dia_prop }
        },
        {
            "name": "resumo_da_semana",
            "description": "Tempo de foco por dia nos ultimos 7 dias (terminando hoje ou no dia informado).",
            "inputSchema": { "type": "object", "properties": dia_prop }
        },
        {
            "name": "recap_do_dia",
            "description": "O DOSSIE completo do dia num tiro so: intencao declarada, resumo, blocos, pomodoros e media da semana — pronto pra voce analisar (gentil, nunca punitivo) e comparar intencao vs realidade. Use no fim do dia.",
            "inputSchema": { "type": "object", "properties": dia_prop }
        },
        {
            "name": "standup",
            "description": "Dados de ontem + hoje condensados pra voce redigir um update de standup em primeira pessoa (o que fiz, no que estou, travas).",
            "inputSchema": { "type": "object", "properties": dia_prop }
        },
        {
            "name": "corrigir_categoria",
            "description": "ESCREVE no focusbar: corrige a categoria de um app/site num dia (o veredito da sua analise volta pro dashboard). Categorias validas: Trabalho, Estudo, Comunicação, Pessoal, Ferramenta, Procrastinação, Outro. Use categoria vazia pra voltar ao automatico.",
            "inputSchema": { "type": "object", "properties": {
                "app": { "type": "string", "description": "Nome EXATO do app/site como aparece nos blocos (ex.: YouTube, Opera GX)." },
                "categoria": { "type": "string", "description": "Uma das 7 categorias, ou vazia pra automatico." },
                "dia": { "type": "string", "description": "Dia AAAA-MM-DD. Omita para hoje." }
            }, "required": ["app", "categoria"] }
        }
    ])
}

fn ok(id: Value, result: Value) -> String {
    json!({ "jsonrpc": "2.0", "id": id, "result": result }).to_string()
}

fn err(id: Value, code: i64, message: &str) -> String {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
        .to_string()
}

fn tool_text(text: &str) -> Value {
    json!({ "content": [{ "type": "text", "text": text }] })
}

fn tool_error(msg: &str) -> Value {
    json!({ "content": [{ "type": "text", "text": format!("Erro: {msg}") }], "isError": true })
}

// ---------- ferramentas ----------

fn call_tool(conn: &Connection, req: &Value) -> Result<String, String> {
    let params = req.get("params").cloned().unwrap_or(json!({}));
    let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
    let args = params.get("arguments").cloned().unwrap_or(json!({}));
    let dia = args.get("dia").and_then(|d| d.as_str());

    match name {
        "resumo_do_dia" => resumo_do_dia(conn, dia),
        "blocos_do_dia" => blocos_do_dia(conn, dia),
        "pomodoros_do_dia" => pomodoros_do_dia(conn, dia),
        "resumo_da_semana" => resumo_da_semana(conn, dia),
        "recap_do_dia" => recap_do_dia(conn, dia),
        "standup" => standup(conn, dia),
        "corrigir_categoria" => corrigir_categoria(&args, dia),
        other => Err(format!("ferramenta desconhecida: {other}")),
    }
}

/// O dossiê do fim de dia: tudo que o Claude precisa num tiro só.
fn recap_do_dia(conn: &Connection, dia: Option<&str>) -> Result<String, String> {
    let label = day_label(dia);
    let ints = db::notes::intentions_for_day(conn, &label).unwrap_or_default();
    let intencao = if ints.is_empty() {
        "  (nenhuma intenção declarada neste dia)".to_string()
    } else {
        ints.iter().map(|i| format!("  - {i}")).collect::<Vec<_>>().join("\n")
    };
    let resumo = resumo_do_dia(conn, dia)?;
    let blocos = blocos_do_dia(conn, dia)?;
    let poms = pomodoros_do_dia(conn, dia)?;
    let semana = resumo_da_semana(conn, dia)?;
    Ok(format!(
        "RECAP DE {label}\n\nINTENÇÃO DECLARADA:\n{intencao}\n\n{resumo}\n\n{blocos}\n\n{poms}\n\nCONTEXTO DA SEMANA (pra comparar com a média):\n{semana}\n\n\
COMO ANALISAR (a pessoa tem TDAH — gentil e honesto, NUNCA punitivo):\n\
1. O dia em 3-5 blocos concretos (use URLs/títulos, não só nome de app).\n\
2. Intenção vs realidade: foi pra onde disse que ia?\n\
3. O maior ladrão de tempo, nomeado, e QUANDO o dia escapou.\n\
4. UMA melhoria pequena e concreta pra amanhã.\n\
Se uma categoria estiver errada (ex.: YouTube que era estudo), corrija com a ferramenta corrigir_categoria."
    ))
}

/// Ontem + hoje condensados pro Claude redigir o update de standup.
fn standup(conn: &Connection, dia: Option<&str>) -> Result<String, String> {
    let base = parse_date(dia);
    // "Ontem" útil: volta até 3 dias procurando o último dia com dados
    // (segunda-feira o standup fala da sexta, não do domingo vazio).
    let mut ontem_label = (base - Duration::days(1)).format("%Y-%m-%d").to_string();
    for back in 1..=3 {
        let d = (base - Duration::days(back)).format("%Y-%m-%d").to_string();
        let (s, e) = day_bounds(Some(&d));
        if !db::sessions_in_range(conn, s, e).map_err(de)?.is_empty() {
            ontem_label = d;
            break;
        }
    }
    let ontem = blocos_do_dia(conn, Some(&ontem_label))?;
    let hoje = blocos_do_dia(conn, Some(&day_label(dia)))?;
    let poms_ontem = pomodoros_do_dia(conn, Some(&ontem_label))?;
    Ok(format!(
        "DADOS PRO STANDUP\n\nÚLTIMO DIA TRABALHADO ({ontem_label}):\n{ontem}\n{poms_ontem}\n\nHOJE (até agora):\n{hoje}\n\n\
Redija um update de standup CURTO, em primeira pessoa, em 3 partes: o que fiz, \
no que estou agora, travas (se os dados não mostrarem trava, diga 'sem travas'). \
Foque em atividades concretas (projetos/sites), não em nomes de app."
    ))
}

/// A ÚNICA escrita do MCP: o veredito do Claude vira categoria no dashboard.
/// Abre uma conexão própria read-write só pra isto (a principal segue READ_ONLY).
fn corrigir_categoria(args: &Value, dia: Option<&str>) -> Result<String, String> {
    const VALIDAS: &[&str] = &[
        "Trabalho", "Estudo", "Comunicação", "Pessoal", "Ferramenta", "Procrastinação", "Outro",
    ];
    let app = args
        .get("app")
        .and_then(|a| a.as_str())
        .map(str::trim)
        .filter(|a| !a.is_empty())
        .ok_or("informe o app (nome exato do bloco)")?;
    let categoria = args.get("categoria").and_then(|c| c.as_str()).unwrap_or("").trim();
    if !categoria.is_empty() && !VALIDAS.contains(&categoria) {
        return Err(format!(
            "categoria invalida: \"{categoria}\". Validas: {} (ou vazia pra automatico)",
            VALIDAS.join(", ")
        ));
    }
    let (start, end) = day_bounds(dia);
    let rw = Connection::open_with_flags(
        db_path(),
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(de)?;
    let n = db::set_block_category(&rw, app, start, end, categoria).map_err(de)?;
    if n == 0 {
        return Err(format!(
            "nenhuma sessão de \"{app}\" em {} — confira o nome exato no blocos_do_dia",
            day_label(dia)
        ));
    }
    Ok(if categoria.is_empty() {
        format!("{app} em {} voltou pra categoria automática ({n} sessões).", day_label(dia))
    } else {
        format!("{app} em {} → {categoria} ({n} sessões corrigidas).", day_label(dia))
    })
}

fn resumo_do_dia(conn: &Connection, dia: Option<&str>) -> Result<String, String> {
    let (start, end) = day_bounds(dia);
    let label = day_label(dia);
    let sessions = db::sessions_in_range(conn, start, end).map_err(de)?;
    if sessions.is_empty() {
        return Ok(format!("{label}: nenhum tempo registrado ainda."));
    }
    let overrides = db::category_overrides(conn).map_err(de)?;

    let mut total = 0i64;
    let mut by_app: std::collections::HashMap<String, i64> = Default::default();
    let mut by_cat: std::collections::HashMap<String, i64> = Default::default();
    for s in &sessions {
        total += s.duration_secs;
        *by_app.entry(s.app_name.clone()).or_insert(0) += s.duration_secs;
        let cat = category_of(conn, &overrides, s);
        *by_cat.entry(cat).or_insert(0) += s.duration_secs;
    }

    let mut out = format!("Resumo de {label}\nFoco total: {}\n\nTop apps:", fmt_dur(total));
    for (app, secs) in top(&by_app, 6) {
        out.push_str(&format!("\n  - {app}: {}", fmt_dur(secs)));
    }
    out.push_str("\n\nPor categoria:");
    for (cat, secs) in top(&by_cat, 10) {
        let pct = if total > 0 { secs * 100 / total } else { 0 };
        out.push_str(&format!("\n  - {cat}: {} ({pct}%)", fmt_dur(secs)));
    }
    Ok(out)
}

fn blocos_do_dia(conn: &Connection, dia: Option<&str>) -> Result<String, String> {
    let (start, end) = day_bounds(dia);
    let label = day_label(dia);
    let sessions = db::sessions_in_range(conn, start, end).map_err(de)?;
    let blocks = db::group_sessions(&sessions, db::GROUP_GAP_SECS);
    if blocks.is_empty() {
        return Ok(format!("{label}: sem blocos."));
    }
    let overrides = db::category_overrides(conn).map_err(de)?;
    let mut out = format!("Blocos de {label}:");
    for b in &blocks {
        if b.duration_secs < 60 {
            continue;
        }
        let hora = Local.timestamp_opt(b.start_ts, 0).single();
        let hhmm = hora
            .map(|t| t.format("%H:%M").to_string())
            .unwrap_or_else(|| "--:--".into());
        let atividade = b.activity_ai.clone().unwrap_or_else(|| b.app_name.clone());
        let cat = category_of(conn, &overrides, b);
        out.push_str(&format!(
            "\n  {hhmm}  {atividade} [{}]  · {cat} · {}",
            b.app_name,
            fmt_dur(b.duration_secs)
        ));
    }
    Ok(out)
}

fn pomodoros_do_dia(conn: &Connection, dia: Option<&str>) -> Result<String, String> {
    let (start, end) = day_bounds(dia);
    let label = day_label(dia);
    let (count, secs, done) = db::pomodoro_stats(conn, start, end).map_err(de)?;
    if count == 0 {
        return Ok(format!("{label}: nenhum pomodoro registrado."));
    }
    Ok(format!(
        "Pomodoros em {label}: {count} iniciados, {done} concluidos, {} focados.",
        fmt_dur(secs)
    ))
}

fn resumo_da_semana(conn: &Connection, dia: Option<&str>) -> Result<String, String> {
    let base = parse_date(dia);
    let mut out = String::from("Ultimos 7 dias:");
    let mut total = 0i64;
    for back in (0..7).rev() {
        let d = base - Duration::days(back);
        let label = d.format("%Y-%m-%d").to_string();
        let (start, end) = day_bounds(Some(&label));
        let sessions = db::sessions_in_range(conn, start, end).map_err(de)?;
        let secs: i64 = sessions.iter().map(|s| s.duration_secs).sum();
        total += secs;
        let wd = weekday_pt(d);
        out.push_str(&format!("\n  {label} ({wd}): {}", fmt_dur(secs)));
    }
    out.push_str(&format!("\n  ----\n  Total: {}", fmt_dur(total)));
    Ok(out)
}

// ---------- helpers ----------

/// Categoria efetiva igual ao app: override por app > categoria por CONTEUDO
/// (IA) > regra por nome de app.
fn category_of(
    conn: &Connection,
    overrides: &std::collections::HashMap<String, String>,
    s: &focusbar_lib::models::FocusSession,
) -> String {
    if let Some(c) = db::app_category(conn, &s.app_name)
        .ok()
        .flatten()
        .filter(|c| !c.is_empty())
    {
        return c;
    }
    if let Some(c) = s.category_ai.clone() {
        return c;
    }
    effective(overrides, &s.app_name, &s.title)
}

fn top(map: &std::collections::HashMap<String, i64>, n: usize) -> Vec<(String, i64)> {
    let mut v: Vec<(String, i64)> = map.iter().map(|(k, &c)| (k.clone(), c)).collect();
    v.sort_by(|a, b| b.1.cmp(&a.1));
    v.truncate(n);
    v
}

fn de(e: rusqlite::Error) -> String {
    e.to_string()
}

fn parse_date(dia: Option<&str>) -> NaiveDate {
    dia.and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        .unwrap_or_else(|| Local::now().date_naive())
}

fn day_label(dia: Option<&str>) -> String {
    parse_date(dia).format("%Y-%m-%d").to_string()
}

fn day_bounds(dia: Option<&str>) -> (i64, i64) {
    let date = parse_date(dia);
    (local_midnight(date), local_midnight(date + Duration::days(1)))
}

fn local_midnight(date: NaiveDate) -> i64 {
    // 00:00 pode NÃO existir num dia de horário-de-verão (spring-forward); aí
    // `earliest()` é None. Cair em 0 colapsaria o dia em "1970→hoje", então
    // usamos 01:00 como rede (sempre existe) — desloca 1h só nesse dia raro.
    let at = |h, m| {
        date.and_hms_opt(h, m, 0)
            .and_then(|n| Local.from_local_datetime(&n).earliest())
            .map(|t| t.timestamp())
    };
    at(0, 0).or_else(|| at(1, 0)).unwrap_or(0)
}

fn fmt_dur(secs: i64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    if h > 0 {
        format!("{h}h{m:02}min")
    } else {
        format!("{m}min")
    }
}

fn weekday_pt(d: NaiveDate) -> &'static str {
    use chrono::Datelike;
    match d.weekday() {
        chrono::Weekday::Mon => "seg",
        chrono::Weekday::Tue => "ter",
        chrono::Weekday::Wed => "qua",
        chrono::Weekday::Thu => "qui",
        chrono::Weekday::Fri => "sex",
        chrono::Weekday::Sat => "sab",
        chrono::Weekday::Sun => "dom",
    }
}
