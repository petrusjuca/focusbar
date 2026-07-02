//! API HTTP local (127.0.0.1:7690) — a porta de entrada da EXTENSÃO de browser
//! (Fase A do roadmap). Escuta SÓ em loopback: nada é acessível de fora da
//! máquina. Rotas:
//!  • POST /api/tab-event — a extensão reporta aba ativada/atualizada/fechada
//!  • GET  /api/health    — sinal de vida (versão + último evento da extensão)
//!
//! Cada evento vira: (1) a "aba ativa agora" no TabFeed, que o sampler consulta
//! pra dar URL certa onde AppleScript/AX falham (Opera GX, Windows); e (2) uma
//! linha crua em tab_events (url já limpa de query/fragment).

use crate::capture::browser;
use crate::capture::tab_feed::{TabFeed, TabInfo};
use crate::db;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use tiny_http::{Header, Method, Response, Server};

pub const ADDR: &str = "127.0.0.1:7690";

fn now_ts() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Evento de aba já validado e limpo, pronto pra usar.
#[derive(Debug, PartialEq)]
struct TabEvent {
    action: String, // "activated" | "updated" | "removed"
    browser: String,
    tab_id: String,
    url: String, // já sem query/fragment (privacidade)
    title: String,
}

/// Valida e limpa o corpo do POST /api/tab-event. Pura — testável sem servidor.
fn parse_tab_event(body: &str) -> Option<TabEvent> {
    let v: serde_json::Value = serde_json::from_str(body).ok()?;
    let get = |k: &str| {
        v.get(k)
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .trim()
            .to_string()
    };
    let action = get("action");
    if !matches!(action.as_str(), "activated" | "updated" | "removed") {
        return None;
    }
    let raw_url = get("url");
    Some(TabEvent {
        action,
        browser: get("browser").to_lowercase(),
        tab_id: get("tab_id"),
        url: if raw_url.is_empty() {
            String::new()
        } else {
            browser::clean_url(&raw_url)
        },
        title: get("title"),
    })
}

fn json(body: String, status: u32) -> Response<std::io::Cursor<Vec<u8>>> {
    let mut r = Response::from_string(body)
        .with_status_code(status)
        .with_header(Header::from_bytes("Content-Type", "application/json").unwrap());
    // Loopback-only; liberar CORS aqui só evita atrito se a extensão (ou uma
    // página de debug local) chamar via fetch com preflight.
    r.add_header(Header::from_bytes("Access-Control-Allow-Origin", "*").unwrap());
    r
}

/// Sobe o servidor numa thread própria. Se a porta estiver ocupada (outro
/// focusbar aberto?), loga e desiste — o app segue funcionando sem extensão.
pub fn spawn(db: Arc<Mutex<Connection>>, feed: Arc<TabFeed>) {
    std::thread::spawn(move || {
        let server = match Server::http(ADDR) {
            Ok(s) => s,
            Err(e) => {
                eprintln!(
                    "focusbar: API local NÃO subiu em {ADDR} ({e}) — extensão de browser inativa"
                );
                return;
            }
        };
        serve_loop(server, db, feed);
    });
}

/// O loop do servidor, separado do bind — os testes sobem numa porta efêmera
/// (a 7690 pode estar ocupada pelo focusbar REAL rodando na máquina).
fn serve_loop(server: Server, db: Arc<Mutex<Connection>>, feed: Arc<TabFeed>) {
    for mut req in server.incoming_requests() {
        let path = req.url().split('?').next().unwrap_or("").to_string();
        let resp = match (req.method(), path.as_str()) {
            // Preflight de CORS (fetch de página local de debug).
            (Method::Options, _) => {
                let mut r = Response::from_string("").with_status_code(204);
                r.add_header(Header::from_bytes("Access-Control-Allow-Origin", "*").unwrap());
                r.add_header(
                    Header::from_bytes("Access-Control-Allow-Headers", "Content-Type").unwrap(),
                );
                r.add_header(
                    Header::from_bytes("Access-Control-Allow-Methods", "POST, GET").unwrap(),
                );
                r
            }

            (Method::Get, "/api/health") => json(
                format!(
                    "{{\"ok\":true,\"version\":\"{}\",\"ext_last_event_ts\":{}}}",
                    env!("CARGO_PKG_VERSION"),
                    feed.last_event_ts()
                ),
                200,
            ),

            (Method::Post, "/api/tab-event") => {
                let mut body = String::new();
                let _ = req.as_reader().read_to_string(&mut body);
                match parse_tab_event(&body) {
                    Some(ev) => {
                        let now = now_ts();
                        if ev.action == "removed" {
                            feed.forget_tab(&ev.tab_id, now);
                        } else {
                            feed.record(TabInfo {
                                url: ev.url.clone(),
                                title: ev.title.clone(),
                                browser: ev.browser.clone(),
                                tab_id: ev.tab_id.clone(),
                                ts: now,
                            });
                        }
                        if let Ok(conn) = db.lock() {
                            let _ = db::append_tab_event(
                                &conn,
                                now,
                                &ev.browser,
                                &ev.action,
                                &ev.tab_id,
                                &ev.url,
                                &ev.title,
                            );
                        }
                        json("{\"ok\":true}".into(), 200)
                    }
                    None => json("{\"error\":\"json\"}".into(), 400),
                }
            }

            _ => json("{\"error\":\"rota\"}".into(), 404),
        };
        let _ = req.respond(resp);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read as _;

    #[test]
    fn parse_limpa_url_e_normaliza_browser() {
        let ev = parse_tab_event(
            r#"{"action":"activated","browser":"Opera GX","tab_id":"7",
                "url":"https://site.com/page?token=SEGREDO#frag","title":"Página"}"#,
        )
        .unwrap();
        assert_eq!(ev.url, "https://site.com/page");
        assert_eq!(ev.browser, "opera gx");
        assert_eq!(ev.tab_id, "7");
    }

    #[test]
    fn parse_rejeita_action_desconhecida_e_json_invalido() {
        assert!(parse_tab_event(r#"{"action":"hacked","url":"https://a.com"}"#).is_none());
        assert!(parse_tab_event("nem json").is_none());
    }

    #[test]
    fn parse_removed_sem_url_ok() {
        let ev = parse_tab_event(r#"{"action":"removed","tab_id":"7"}"#).unwrap();
        assert_eq!(ev.action, "removed");
        assert_eq!(ev.url, "");
    }

    /// Sobe o servidor de verdade e bate nele: health responde, tab-event
    /// atualiza o feed E grava a linha crua no banco.
    #[test]
    fn smoke_do_servidor_de_ponta_a_ponta() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::migrate(&conn).unwrap();
        let db = Arc::new(Mutex::new(conn));
        let feed = Arc::new(TabFeed::new());
        // Porta EFÊMERA: a 7690 pode estar ocupada pelo focusbar real rodando
        // na máquina de quem executa os testes. O bind acontece antes da
        // thread, então não há corrida — dá pra conectar já.
        let server = Server::http("127.0.0.1:0").unwrap();
        let addr = server.server_addr().to_string();
        {
            let (db, feed) = (db.clone(), feed.clone());
            std::thread::spawn(move || serve_loop(server, db, feed));
        }

        let post = |body: &str| {
            let mut stream = std::net::TcpStream::connect(&addr).unwrap();
            use std::io::Write as _;
            write!(
                stream,
                "POST /api/tab-event HTTP/1.1\r\nHost: {ADDR}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .unwrap();
            let mut resp = String::new();
            let _ = stream.read_to_string(&mut resp);
            resp
        };

        let resp = post(
            r#"{"action":"activated","browser":"opera gx","tab_id":"1","url":"https://web.whatsapp.com/?x=1","title":"WhatsApp"}"#,
        );
        assert!(resp.contains("200"), "resposta: {resp}");
        assert!(feed.last_event_ts() > 0);
        assert!(feed
            .url_for("Opera GX", "WhatsApp", now_ts())
            .is_some_and(|u| u == "https://web.whatsapp.com"));

        // Fechar a aba ativa registra E limpa o feed.
        let resp = post(r#"{"action":"removed","tab_id":"1"}"#);
        assert!(resp.contains("200"), "resposta: {resp}");
        assert!(feed.url_for("Opera GX", "WhatsApp", now_ts()).is_none());

        let n: i64 = db
            .lock()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM tab_events", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 2);
    }
}
