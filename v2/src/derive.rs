//! Derivador: transforma a TABELA BRUTA (events) em SESSÕES.
//!
//! Regra de ouro do v2: a bruta é a verdade, a derivada é descartável. Este
//! módulo é uma função PURA — mesmos eventos entram, mesmas sessões saem —
//! então dá pra re-derivar o histórico inteiro quando a lógica mudar.
//!
//! Chave de atividade = app + identidade do conteúdo (tab_id da extensão se
//! houver; senão só o app — título muda demais pra ser identidade). Mesma chave
//! reaparecendo em ≤ GAP_MERGE_MS = continuação da mesma sessão (o alt-tab de
//! 5s pro WhatsApp não fragmenta o trabalho em três pedaços).

use serde::Serialize;

/// Um evento da tabela bruta (só os campos que o derivador usa).
#[derive(Debug, Clone)]
pub struct RawEvent {
    pub ts_ms: i64,
    /// "foreground" (troca de janela), "tab" (extensão: aba ativa mudou),
    /// "heartbeat" (poll confirmando que segue a mesma janela).
    pub kind: String,
    pub app: String,
    pub title: String,
    pub url: Option<String>,
    pub tab_id: Option<String>,
}

/// Uma sessão derivada (um uso contínuo da mesma atividade).
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Session {
    pub start_ms: i64,
    pub end_ms: i64,
    /// Soma dos trechos REAIS (sem contar os gaps tolerados pelo merge).
    pub dur_ms: i64,
    pub app: String,
    /// Último título visto (título é metadado, não identidade).
    pub title: String,
    pub url: Option<String>,
    pub tab_id: Option<String>,
}

/// Gap máximo pra tratar o reaparecimento da mesma chave como continuação.
pub const GAP_MERGE_MS: i64 = 90_000;
/// Sessões mais curtas que isso são ruído de alt-tab (ficam SÓ na bruta).
pub const MIN_SESSION_MS: i64 = 2_000;
/// Sem heartbeat/evento por este tempo, consideramos a sessão encerrada
/// neste último sinal (app fechou / máquina dormiu sem evento).
pub const STALE_MS: i64 = 45_000;

fn key(e: &RawEvent) -> (String, Option<String>) {
    (e.app.clone(), e.tab_id.clone())
}

/// Deriva as sessões de uma lista de eventos brutos (ordenada por ts).
/// `now_ms` fecha a sessão em aberto (a corrente, ainda em foco).
pub fn derive_sessions(events: &[RawEvent], now_ms: i64) -> Vec<Session> {
    // 1) Constrói os trechos crus: cada troca de chave fecha o trecho anterior.
    let mut spans: Vec<Session> = Vec::new();
    let mut cur: Option<Session> = None;
    let mut last_seen: i64 = 0;

    for e in events {
        match e.kind.as_str() {
            "foreground" | "tab" => {
                let k = key(e);
                if let Some(c) = cur.as_mut() {
                    if (c.app.clone(), c.tab_id.clone()) == k {
                        // mesma atividade: atualiza metadados e segue.
                        c.title = e.title.clone();
                        if e.url.is_some() {
                            c.url = e.url.clone();
                        }
                        last_seen = e.ts_ms;
                        continue;
                    }
                    // trocou: fecha o trecho no instante da troca.
                    c.end_ms = e.ts_ms.min(last_seen + STALE_MS);
                    c.dur_ms = (c.end_ms - c.start_ms).max(0);
                    spans.push(c.clone());
                }
                cur = Some(Session {
                    start_ms: e.ts_ms,
                    end_ms: e.ts_ms,
                    dur_ms: 0,
                    app: e.app.clone(),
                    title: e.title.clone(),
                    url: e.url.clone(),
                    tab_id: e.tab_id.clone(),
                });
                last_seen = e.ts_ms;
            }
            "heartbeat" => {
                if let Some(c) = cur.as_mut() {
                    if (c.app.clone(), c.tab_id.clone()) == key(e) {
                        last_seen = e.ts_ms;
                        if !e.title.is_empty() {
                            c.title = e.title.clone();
                        }
                    }
                }
            }
            _ => {}
        }
    }
    if let Some(mut c) = cur {
        // fecha a corrente: em `now` se os sinais são frescos; senão no último sinal.
        c.end_ms = now_ms.min(last_seen + STALE_MS);
        c.dur_ms = (c.end_ms - c.start_ms).max(0);
        if c.dur_ms > 0 {
            spans.push(c);
        }
    }

    // 2) Merge: mesma chave reaparecendo em ≤90s = mesma sessão (dur soma os
    //    trechos reais; o gap tolerado NÃO conta como tempo de uso). Procura a
    //    última sessão DA MESMA CHAVE (não a imediatamente anterior) — é isso
    //    que faz "Code → 10s WhatsApp → Code" virar UM bloco de Code, com o
    //    WhatsApp como sessãozinha própria no meio.
    let mut merged: Vec<Session> = Vec::new();
    for s in spans {
        if let Some(m) = merged
            .iter_mut()
            .rev()
            .find(|m| m.app == s.app && m.tab_id == s.tab_id)
        {
            let gap = s.start_ms - m.end_ms;
            if (0..=GAP_MERGE_MS).contains(&gap) {
                m.end_ms = s.end_ms;
                m.dur_ms += s.dur_ms;
                m.title = s.title.clone();
                if s.url.is_some() {
                    m.url = s.url.clone();
                }
                continue;
            }
        }
        merged.push(s);
    }

    // 3) Debounce: some com o ruído (continua íntegro na bruta).
    merged.retain(|s| s.dur_ms >= MIN_SESSION_MS);
    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(ts_s: i64, kind: &str, app: &str, title: &str) -> RawEvent {
        RawEvent {
            ts_ms: ts_s * 1000,
            kind: kind.into(),
            app: app.into(),
            title: title.into(),
            url: None,
            tab_id: None,
        }
    }
    fn tab(ts_s: i64, app: &str, tab_id: &str, url: &str) -> RawEvent {
        RawEvent {
            ts_ms: ts_s * 1000,
            kind: "tab".into(),
            app: app.into(),
            title: String::new(),
            url: Some(url.into()),
            tab_id: Some(tab_id.into()),
        }
    }

    #[test]
    fn mesma_app_com_titulos_diferentes_e_uma_sessao() {
        // título muda (heartbeats/updates) mas a atividade é a mesma.
        let evs = vec![
            ev(0, "foreground", "Code", "main.rs"),
            ev(30, "foreground", "Code", "db.rs"),
            ev(60, "heartbeat", "Code", "db.rs"),
        ];
        let s = derive_sessions(&evs, 90_000);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].app, "Code");
        assert_eq!(s[0].title, "db.rs"); // último título
        assert_eq!(s[0].dur_ms, 90_000);
    }

    #[test]
    fn alt_tab_curto_nao_fragmenta() {
        // Code → 10s de WhatsApp → volta: o Code vira UMA sessão (gap ≤90s),
        // com o WhatsApp como sessãozinha própria no meio.
        let evs = vec![
            ev(0, "foreground", "Code", "a"),
            ev(15, "heartbeat", "Code", "a"),
            ev(30, "foreground", "WhatsApp", "chat"),
            ev(40, "foreground", "Code", "a"),
            ev(55, "heartbeat", "Code", "a"),
            ev(100, "heartbeat", "Code", "a"),
        ];
        let s = derive_sessions(&evs, 100_000);
        let code: Vec<_> = s.iter().filter(|x| x.app == "Code").collect();
        assert_eq!(code.len(), 1);
        // dur soma os trechos REAIS (30s + 60s), sem os 10s de WhatsApp no meio.
        assert_eq!(code[0].dur_ms, 90_000);
        assert_eq!(code[0].end_ms, 100_000);
        assert!(s.iter().any(|x| x.app == "WhatsApp")); // 10s ≥ 2s: fica
    }

    #[test]
    fn gap_longo_divide() {
        let evs = vec![
            ev(0, "foreground", "Code", "a"),
            ev(60, "foreground", "YouTube", "video"),
            ev(300, "foreground", "Code", "a"), // 240s depois → sessão nova
            ev(400, "heartbeat", "Code", "a"),
        ];
        let s = derive_sessions(&evs, 400_000);
        let code: Vec<_> = s.iter().filter(|x| x.app == "Code").collect();
        assert_eq!(code.len(), 2);
    }

    #[test]
    fn ruido_de_menos_de_2s_some_da_derivada() {
        let evs = vec![
            ev(0, "foreground", "Code", "a"),
            RawEvent { ts_ms: 10_000, ..ev(10, "foreground", "Finder", "") },
            RawEvent { ts_ms: 10_800, ..ev(10, "foreground", "Code", "a") }, // Finder durou 0.8s
            ev(60, "heartbeat", "Code", "a"),
        ];
        let s = derive_sessions(&evs, 60_000);
        assert!(!s.iter().any(|x| x.app == "Finder"));
        assert_eq!(s.iter().filter(|x| x.app == "Code").count(), 1);
    }

    #[test]
    fn abas_diferentes_sao_sessoes_diferentes() {
        // tab_id é identidade: trocar de aba no mesmo browser divide.
        let evs = vec![
            tab(0, "Chrome", "t1", "https://youtube.com/watch"),
            tab(120, "Chrome", "t2", "https://claude.ai/chat"),
            tab(240, "Chrome", "t1", "https://youtube.com/watch"), // volta, gap 120s>90 → nova
        ];
        let s = derive_sessions(&evs, 300_000);
        assert_eq!(s.len(), 3);
        assert_eq!(s[0].tab_id.as_deref(), Some("t1"));
        assert_eq!(s[1].tab_id.as_deref(), Some("t2"));
    }

    #[test]
    fn sessao_stale_fecha_no_ultimo_sinal() {
        // app parou de mandar sinal (máquina dormiu): fecha em last_seen+45s,
        // não em `now` — senão inventaria horas de uso.
        let evs = vec![ev(0, "foreground", "Code", "a"), ev(60, "heartbeat", "Code", "a")];
        let s = derive_sessions(&evs, 3_600_000); // "now" = 1h depois
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].end_ms, 60_000 + STALE_MS);
    }

    #[test]
    fn rederivar_e_deterministico() {
        let evs = vec![
            ev(0, "foreground", "Code", "a"),
            ev(100, "foreground", "WhatsApp", "chat"),
            ev(110, "foreground", "Code", "a"),
            ev(500, "heartbeat", "Code", "a"),
        ];
        let a = derive_sessions(&evs, 500_000);
        let b = derive_sessions(&evs, 500_000);
        assert_eq!(a, b);
    }
}
