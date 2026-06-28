//! Cliente do modelo local (Ollama / Llama 3.2 3B). Tudo roda na máquina.
//! O modelo é carregado sob demanda pelo Ollama e descarregado depois (keep_alive).

use serde::{Deserialize, Serialize};

/// URL do Ollama. **Local-first por padrão.** Para usar uma VPS, faça um túnel SSH
/// que escuta em `127.0.0.1` (continua loopback) — NÃO aponte FOCUSBAR_LLM_URL pra
/// um host remoto em http puro. Se mesmo assim apontar pra host não-loopback, só vale
/// com `https://` E `FOCUSBAR_LLM_REMOTE_OK=1`; caso contrário cai de volta no localhost
/// (dados não saem da máquina sem opt-in explícito).
fn base() -> String {
    // IPv4 explícito (127.0.0.1), NÃO "localhost": em Mac/Windows o "localhost"
    // costuma resolver pra ::1 (IPv6) primeiro, mas o Ollama escuta só em IPv4 —
    // aí o reqwest falha com "error sending request". Forçar 127.0.0.1 resolve.
    let url = std::env::var("FOCUSBAR_LLM_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:11434".to_string());
    if is_loopback(&url) {
        return url;
    }
    let remote_ok = std::env::var("FOCUSBAR_LLM_REMOTE_OK")
        .map(|v| v == "1")
        .unwrap_or(false);
    if remote_ok && url.starts_with("https://") {
        url
    } else {
        // Recusa silenciosa: volta pro local em vez de mandar dados pra fora.
        "http://127.0.0.1:11434".to_string()
    }
}

/// O host da URL é loopback (a máquina local)?
fn is_loopback(url: &str) -> bool {
    let host = url.split("://").nth(1).unwrap_or(url);
    let host = host.split('/').next().unwrap_or("");
    host.starts_with("localhost") || host.starts_with("127.0.0.1") || host.starts_with("[::1]")
}

/// Modelo. Local: llama3.2:3b. Na VPS dá pra usar um maior via FOCUSBAR_LLM_MODEL.
fn model() -> String {
    std::env::var("FOCUSBAR_LLM_MODEL").unwrap_or_else(|_| "llama3.2:3b".to_string())
}

#[derive(Serialize)]
struct GenReq {
    model: String,
    prompt: String,
    stream: bool,
    keep_alive: String,
    options: GenOpts,
}

#[derive(Serialize)]
struct GenOpts {
    temperature: f32,
    num_predict: i32,
}

#[derive(Deserialize)]
struct GenResp {
    response: String,
}

/// Ollama está rodando e acessível?
pub async fn is_available() -> bool {
    reqwest::Client::new()
        .get(format!("{}/api/tags", base()))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

#[derive(Deserialize)]
struct TagsResp {
    models: Vec<TagModel>,
}
#[derive(Deserialize)]
struct TagModel {
    name: String,
}

/// O modelo configurado já está baixado?
pub async fn has_model() -> bool {
    let wanted = model();
    let base_name = wanted.split(':').next().unwrap_or(&wanted).to_string();
    match reqwest::Client::new()
        .get(format!("{}/api/tags", base()))
        .send()
        .await
    {
        Ok(r) => match r.json::<TagsResp>().await {
            Ok(t) => t
                .models
                .iter()
                .any(|m| m.name == wanted || m.name.starts_with(&base_name)),
            Err(_) => false,
        },
        Err(_) => false,
    }
}

/// Veredito da IA: ajuda no foco? + a PROVA (trecho que o modelo diz ter lido).
/// O chamador VERIFICA se `evidence` existe mesmo na tela — se não, a IA
/// inventou (alucinou pela "funcionalidade" do app) e o veredito é descartado.
pub struct OnTaskJudgment {
    pub on_task: bool,
    pub evidence: String,
}

/// Pega o que vem depois de "marcador:" numa linha que contenha o marcador.
fn line_value(resp: &str, marker_lower: &str) -> Option<String> {
    resp.lines().find_map(|line| {
        if line.to_lowercase().contains(marker_lower) {
            line.splitn(2, ':').nth(1).map(|s| s.trim().to_string())
        } else {
            None
        }
    })
}

/// Julga se a janela ajuda no foco — ANCORADO no que está escrito na tela.
/// O modelo precisa COPIAR um trecho exato como prova; quem chama confere se o
/// trecho existe (anti-alucinação). Best-effort (3B pode errar).
pub async fn on_task_check(
    focus: &str,
    app: &str,
    title: &str,
    extra: &str,
) -> Result<OnTaskJudgment, String> {
    let screen = if extra.trim().is_empty() {
        "(sem texto legível)".to_string()
    } else {
        format!("\"{}\"", extra.trim())
    };
    let prompt = format!(
        "Você decide se a janela atual AJUDA no foco do usuário.\n\
REGRA DURA: julgue SÓ pelo TÍTULO e pelo TEXTO DA TELA abaixo. NUNCA deduza pelo \
que o app/site/ferramenta normalmente SERVE — vale só o que está ESCRITO. Se o que \
está escrito não deixar o assunto claro, diga que não dá pra saber.\n\
Foco do usuário: \"{focus}\".\n\
Janela: {app} — {title}\n\
Texto na tela: {screen}\n\n\
Responda em DUAS linhas, em português:\n\
VEREDITO: SIM (ajuda) ou NAO (distração)\n\
PROVA: copie de 2 a 8 palavras EXATAS do título ou do texto acima que mostram o \
assunto. Se nada acima deixar claro, escreva exatamente: PROVA: nada"
    );
    let resp = generate(prompt).await?;

    // Veredito: primeira palavra alfanumérica da linha VEREDITO (ou da resposta).
    let vline = line_value(&resp, "veredito").unwrap_or_else(|| resp.clone());
    let first: String = vline
        .chars()
        .skip_while(|c| !c.is_alphanumeric())
        .take_while(|c| c.is_alphanumeric())
        .collect::<String>()
        .to_uppercase();
    let on_task = match first.as_str() {
        "SIM" | "S" | "YES" | "Y" => true,
        "NAO" | "NÃO" | "N" | "NO" | "NOT" => false,
        _ => return Err(format!("modelo não respondeu claramente: {first:?}")),
    };

    let evidence = line_value(&resp, "prova")
        .unwrap_or_default()
        .trim_matches(|c: char| c == '"' || c == '\'' || c.is_whitespace())
        .to_string();
    Ok(OnTaskJudgment { on_task, evidence })
}

/// Baixa o modelo via Ollama (bloqueia até terminar — pode levar minutos).
pub async fn pull_model() -> Result<(), String> {
    let body = serde_json::json!({ "name": model(), "stream": false });
    let resp = reqwest::Client::new()
        .post(format!("{}/api/pull", base()))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Ollama indisponível ({e})."))?;
    if !resp.status().is_success() {
        return Err(format!("Falha ao baixar modelo ({})", resp.status()));
    }
    Ok(())
}

/// Gera texto a partir de um prompt. keep_alive curto = descarrega rápido da RAM.
pub async fn generate(prompt: String) -> Result<String, String> {
    let body = GenReq {
        model: model(),
        prompt,
        stream: false,
        keep_alive: "1m".to_string(),
        options: GenOpts {
            temperature: 0.3,
            // A resposta é UMA linha (SIM/NAO + motivo curto); 200 evita o 3B
            // divagar antes do token decisivo.
            num_predict: 200,
        },
    };
    // Timeout teto: evita travar pra sempre se o Ollama enroscar. 45s dá folga
    // pro 1º uso com o modelo FRIO (carregar um 3B na RAM pode passar de 10s) —
    // 8s estourava no cold start e dava "error sending request". É async: uma
    // checagem lenta não congela a UI, só responde mais tarde.
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(45))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .post(format!("{}/api/generate", base()))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Ollama indisponível ({e}). O servidor está rodando?"))?;

    if !resp.status().is_success() {
        return Err(format!("Ollama respondeu {}", resp.status()));
    }
    let parsed: GenResp = resp.json().await.map_err(|e| e.to_string())?;
    Ok(parsed.response.trim().to_string())
}

/// Normaliza o que o modelo respondeu pra UMA categoria conhecida do dashboard.
fn normalize_category(raw: &str) -> Option<String> {
    let r = raw.to_lowercase();
    let cat = if r.contains("distra") || r.contains("procrastin") {
        "Procrastinação"
    } else if r.contains("estud") {
        "Estudo"
    } else if r.contains("comunica") {
        "Comunicação"
    } else if r.contains("pessoal") {
        "Pessoal"
    } else if r.contains("trabalh") {
        "Trabalho"
    } else if r.contains("ferramenta") {
        "Ferramenta"
    } else {
        return None;
    };
    Some(cat.to_string())
}

/// Classifica uma atividade pelo CONTEÚDO da tela (não pelo nome do app) e dá um
/// nome curto. Ex.: YouTube com "Cálculo 1 - derivadas" → ("Estudo","Estudando Cálculo").
/// Precisa do Ollama. Retorna (categoria, nome_atividade).
pub async fn categorize_activity(
    app: &str,
    title: &str,
    content: &str,
) -> Result<(String, String), String> {
    // Categoria de partida pela REGRA (já sabe que qBittorrent/WhatsApp/Discord
    // não são estudo). A IA só pode CONTRARIAR isso com prova no texto da tela.
    let rule_cat = crate::category::categorize(app, title).to_string();

    let prompt = format!(
        "Você classifica uma atividade de computador pelo CONTEÚDO REAL da tela.\n\
REGRA DE OURO: só dê uma categoria se o TEXTO DA TELA mostrar do que se trata. Se \
o texto for genérico (menu, lista, sem assunto) e não deixar claro o que a pessoa \
fazia, responda CATEGORIA: INCERTO. NUNCA invente, NUNCA deduza pelo nome do app.\n\
O ASSUNTO aparecer na tela NÃO torna a atividade produtiva: baixar torrent, \
assistir série/filme/novela, ver gameplay ou rolar feed é Procrastinação MESMO \
que o nome do arquivo/vídeo apareça. Estudo/Trabalho exigem sinal real de estudo \
ou trabalho (matéria, curso, código, documento, planilha), não só um título.\n\
Ex.: vídeo 'Cálculo 1 - derivadas' → Estudo. \
'Baixando Breaking.Bad.mkv' → Procrastinação (é uma série, não estudo). \
Tela genérica → INCERTO.\n\
Categorias possíveis: Trabalho, Estudo, Pessoal, Comunicação, Procrastinação, INCERTO.\n\
App: {app}\n\
Título: {title}\n\
Texto na tela: \"{content}\"\n\n\
Responda em DUAS linhas, em português:\n\
CATEGORIA: <uma das opções>\n\
ATIVIDADE: <2 a 5 palavras COPIADAS do próprio texto; se INCERTO, escreva: -->"
    );
    let resp = generate(prompt).await?;

    let raw_activity: String = line_value(&resp, "atividade")
        .unwrap_or_default()
        .trim()
        .trim_matches(|c: char| c == '"' || c == '\'')
        .chars()
        .take(60)
        .collect();
    // Âncora: a atividade precisa ter ao menos uma palavra de verdade que de fato
    // aparece no texto da tela. Sem isso, é provável invenção.
    let grounded = activity_grounded(&raw_activity, content);

    let cat_line = line_value(&resp, "categoria").unwrap_or_default();
    let category = match normalize_category(&cat_line) {
        Some(c) if c == rule_cat => c,   // concorda com a regra → confia
        Some(c) if grounded => c,        // diverge MAS tem prova → confia (YouTube "Cálculo"→Estudo)
        _ => rule_cat,                   // incerto, ou diverge sem prova → regra manda
    };

    // Atividade sem lastro vira vazia — melhor nada do que um rótulo inventado.
    let activity = if grounded { raw_activity } else { String::new() };
    Ok((category, activity))
}

/// A atividade tem lastro no texto da tela? Verdadeira se alguma palavra
/// significativa (>=4 letras) da atividade aparece como TOKEN INTEIRO no conteúdo
/// (sem acento/caixa). Casa por token, não por substring — senão "casa" daria
/// match em "casamento" (a category.rs já evita isso da mesma forma).
fn activity_grounded(activity: &str, content: &str) -> bool {
    let tokens: std::collections::HashSet<String> = fold(content)
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.chars().count() >= 4)
        .map(|w| w.to_string())
        .collect();
    activity
        .split(|c: char| !c.is_alphanumeric())
        .map(fold)
        .filter(|w| w.chars().count() >= 4)
        .any(|w| tokens.contains(&w))
}

/// minúsculas + sem acento, pra casar texto de forma robusta.
fn fold(s: &str) -> String {
    s.chars()
        .flat_map(|c| c.to_lowercase())
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            'ñ' => 'n',
            other => other,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grounded_when_word_is_on_screen() {
        // "Cálculo" aparece no texto → tem lastro (mesmo com acento/caixa).
        assert!(activity_grounded(
            "Estudando Cálculo",
            "Curso de calculo 1 - derivadas e limites"
        ));
    }

    #[test]
    fn not_grounded_when_invented() {
        // O modelo inventou "Verificar uso de memória" mas nada disso está na tela.
        assert!(!activity_grounded(
            "Verificar uso de memória",
            "Conversa no app, mensagens trocadas hoje"
        ));
    }

    #[test]
    fn short_words_dont_count_as_evidence() {
        // Só palavras curtas em comum (<4 letras) não bastam como prova.
        assert!(!activity_grounded("ver os pdf", "ABC de la os um"));
    }

    #[test]
    fn substring_is_not_evidence() {
        // "casa" NÃO pode dar lastro em "casamento" — casa por token inteiro.
        assert!(!activity_grounded("casa nova", "festa de casamento ontem"));
        // mas o token inteiro conta como prova:
        assert!(activity_grounded("planejando casamento", "lista do casamento e convites"));
    }

    #[test]
    fn empty_activity_is_not_grounded() {
        assert!(!activity_grounded("", "qualquer texto na tela aqui"));
        assert!(!activity_grounded("--", "qualquer texto na tela aqui"));
    }

    #[test]
    fn normalize_maps_incerto_to_none() {
        // "INCERTO" não é categoria válida → cai pra regra no chamador.
        assert!(normalize_category("INCERTO").is_none());
        assert_eq!(normalize_category("Estudo").as_deref(), Some("Estudo"));
    }
}
