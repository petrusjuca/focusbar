//! API HTTP local (127.0.0.1:7690) — a MESMA porta serve:
//!  • a UI web (GET /) — dev no navegador; no fim vira a janela Tauri
//!  • a API JSON (/api/...) que a UI consome
//!  • o endpoint da extensão de browser (POST /api/tab-event)
//! Escuta SÓ em loopback: nada é acessível de fora da máquina.

use crate::db::{self, NewEvent};
use crate::derive;
use chrono::{Duration as ChDuration, Local, NaiveDate, TimeZone};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use tiny_http::{Header, Method, Response, Server};

pub const ADDR: &str = "127.0.0.1:7690";

fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

/// [início, fim) do dia local. Meia-noite pode não existir (DST) → cai pra 01:00.
fn day_bounds_ms(date: NaiveDate) -> (i64, i64) {
    let at = |d: NaiveDate, h: u32| {
        d.and_hms_opt(h, 0, 0)
            .and_then(|n| Local.from_local_datetime(&n).earliest())
            .map(|t| t.timestamp_millis())
    };
    let start = at(date, 0).or_else(|| at(date, 1)).unwrap_or(0);
    let next = date + ChDuration::days(1);
    let end = at(next, 0).or_else(|| at(next, 1)).unwrap_or(i64::MAX);
    (start, end)
}

fn json_response(body: String, status: u32) -> Response<std::io::Cursor<Vec<u8>>> {
    Response::from_string(body)
        .with_status_code(status)
        .with_header(Header::from_bytes("Content-Type", "application/json").unwrap())
}

pub fn serve(db: Arc<Mutex<Connection>>) {
    let server = Server::http(ADDR).expect("porta 7690 ocupada — outro focusbar-core rodando?");
    println!("focusbar v2 core → http://{ADDR}");

    for mut req in server.incoming_requests() {
        let url = req.url().to_string();
        let path = url.split('?').next().unwrap_or("").to_string();

        let resp = match (req.method(), path.as_str()) {
            (Method::Get, "/") => Response::from_string(UI_HTML)
                .with_header(Header::from_bytes("Content-Type", "text/html; charset=utf-8").unwrap()),

            (Method::Get, "/api/health") => {
                let n = db.lock().ok().and_then(|c| db::event_count(&c).ok()).unwrap_or(-1);
                json_response(format!("{{\"ok\":true,\"version\":\"{}\",\"events\":{n}}}", env!("CARGO_PKG_VERSION")), 200)
            }

            (Method::Get, "/api/day") => {
                // ?date=YYYY-MM-DD (default hoje). Sessões derivadas NA HORA da bruta.
                let date = url.split("date=").nth(1)
                    .and_then(|s| NaiveDate::parse_from_str(s.split('&').next().unwrap_or(""), "%Y-%m-%d").ok())
                    .unwrap_or_else(|| Local::now().date_naive());
                let (start, end) = day_bounds_ms(date);
                let result = db.lock().ok().and_then(|c| db::events_in_range(&c, start, end).ok());
                match result {
                    Some(evs) => {
                        let sessions = derive::derive_sessions(&evs, now_ms());
                        json_response(serde_json::json!({
                            "date": date.format("%Y-%m-%d").to_string(),
                            "raw_events": evs.len(),
                            "sessions": sessions,
                        }).to_string(), 200)
                    }
                    None => json_response("{\"error\":\"db\"}".into(), 500),
                }
            }

            (Method::Post, "/api/tab-event") => {
                // Extensão: {"url":"...","tab_id":"...","title":"...","action":"activated|updated|removed","browser":"..."}
                let mut body = String::new();
                let _ = req.as_reader().read_to_string(&mut body);
                match serde_json::from_str::<serde_json::Value>(&body) {
                    Ok(v) => {
                        let get = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
                        let (u, t, ti, act, br) = (get("url"), get("tab_id"), get("title"), get("action"), get("browser"));
                        let kind = if act == "removed" { "state" } else { "tab" };
                        let payload = if act == "removed" { Some(format!("{{\"tab_closed\":\"{t}\"}}")) } else { None };
                        if let Ok(c) = db.lock() {
                            let _ = db::append(&c, &NewEvent {
                                ts_ms: now_ms(), kind,
                                app: if br.is_empty() { "Browser" } else { &br },
                                title: &ti, url: if u.is_empty() { None } else { Some(&u) },
                                tab_id: if t.is_empty() { None } else { Some(&t) },
                                pid: None, source: "extensao", payload: payload.as_deref(),
                            });
                        }
                        json_response("{\"ok\":true}".into(), 200)
                    }
                    Err(_) => json_response("{\"error\":\"json\"}".into(), 400),
                }
            }

            _ => json_response("{\"error\":\"rota\"}".into(), 404),
        };
        let _ = req.respond(resp);
    }
}

/// UI mínima da Fase 1: prova visual de que bruta → derivador → blocos funciona.
/// (A UI de verdade cresce aqui mesmo nas próximas fases.)
const UI_HTML: &str = r#"<!doctype html><html lang="pt-BR"><meta charset="utf-8">
<title>focusbar v2</title>
<style>
 body{font-family:-apple-system,system-ui,sans-serif;background:#14161b;color:#eee;max-width:720px;margin:2rem auto;padding:0 1rem}
 h1{font-size:1.2rem} .sub{color:#9b9ba0;font-size:.85rem}
 .row{display:flex;gap:.6rem;align-items:center;padding:.45rem .6rem;border-bottom:1px solid #23262e}
 .t{color:#9b9ba0;font-variant-numeric:tabular-nums;min-width:3.2rem;font-size:.8rem}
 .app{font-weight:600} .title{color:#9b9ba0;font-size:.85rem;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;flex:1}
 .dur{color:#5ac8fa;font-variant-numeric:tabular-nums;font-size:.85rem}
</style>
<h1>focusbar v2 <span class="sub">core rodando · sessões derivadas da bruta em tempo real</span></h1>
<div class="sub" id="meta"></div><div id="list"></div>
<script>
async function load(){
  const d = await (await fetch('/api/day')).json();
  document.getElementById('meta').textContent = d.date+' · '+d.raw_events+' eventos brutos → '+d.sessions.length+' sessões';
  const f = ms => { const m=Math.round(ms/60000); return m>=60? Math.floor(m/60)+'h'+String(m%60).padStart(2,'0') : m+'min'; };
  const hh = ms => new Date(ms).toTimeString().slice(0,5);
  document.getElementById('list').innerHTML = d.sessions.map(s=>
    `<div class="row"><span class="t">${hh(s.start_ms)}</span><span class="app">${s.app}</span>`+
    `<span class="title">${s.title||s.url||''}</span><span class="dur">${f(s.dur_ms)}</span></div>`).join('');
}
load(); setInterval(load, 5000);
</script></html>"#;
