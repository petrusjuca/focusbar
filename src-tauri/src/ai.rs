//! Cliente do modelo local (Ollama / Llama 3.2 3B). Tudo roda na máquina.
//! O modelo é carregado sob demanda pelo Ollama e descarregado depois (keep_alive).

use serde::{Deserialize, Serialize};

/// URL do Ollama. **Local-first por padrão.** Para usar uma VPS, faça um túnel SSH
/// que escuta em `127.0.0.1` (continua loopback) — NÃO aponte FOCUSBAR_LLM_URL pra
/// um host remoto em http puro. Se mesmo assim apontar pra host não-loopback, só vale
/// com `https://` E `FOCUSBAR_LLM_REMOTE_OK=1`; caso contrário cai de volta no localhost
/// (dados não saem da máquina sem opt-in explícito).
fn base() -> String {
    let url = std::env::var("FOCUSBAR_LLM_URL")
        .unwrap_or_else(|_| "http://localhost:11434".to_string());
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
        "http://localhost:11434".to_string()
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

/// Julga se a janela atual ajuda no foco declarado ou é distração.
/// Retorna (no_foco, motivo_curto). Best-effort (3B pode errar).
pub async fn on_task_check(
    focus: &str,
    app: &str,
    title: &str,
    extra: &str,
) -> Result<(bool, String), String> {
    // "Olhos" Estágio 1: trecho do texto visível na janela (lido pela AX).
    let screen = if extra.trim().is_empty() {
        String::new()
    } else {
        format!("Texto visível na janela (use pra entender o assunto real): \"{}\"\n", extra.trim())
    };
    let prompt = format!(
        "Você ajuda alguém com TDAH a manter o foco. Julgue pelo CONTEÚDO concreto: \
o título E o texto visível dizem o ASSUNTO real — use isso, não o nome do app ou site. \
Exemplos: um vídeo 'Cálculo 1 - máximos e mínimos por derivada' no YouTube AJUDA quem \
quer estudar cálculo; um 'Nintendo Direct' ou uma novela é DISTRAÇÃO; um doc com o nome \
do projeto AJUDA.\n\
Foco/tarefa atual: \"{focus}\".\n\
Atividade agora: {app} — {title}\n\
{screen}\
Isso ajuda no foco ou é distração? Responda SEMPRE em português, em UMA linha curta \
começando EXATAMENTE com a palavra SIM (ajuda) ou NAO (distração), seguida de um motivo \
bem curto citando o ASSUNTO que você viu. Não comece com nenhuma outra palavra."
    );
    let resp = generate(prompt).await?;
    // Parser robusto: primeira "palavra" só com letras/dígitos. Ambíguo (não
    // começa com SIM/NAO/YES/NO) → Err, pra cair no fallback honesto em vez de
    // marcar "no foco" por engano (o 3B às vezes responde em inglês: "No, ...").
    let first: String = resp
        .trim()
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
    Ok((on_task, resp.trim().to_string()))
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
        keep_alive: "2m".to_string(),
        options: GenOpts {
            temperature: 0.3,
            // A resposta é UMA linha (SIM/NAO + motivo curto); 200 evita o 3B
            // divagar antes do token decisivo.
            num_predict: 200,
        },
    };
    // Timeout: se o modelo travar/recarregar (pós-reinício, OOM), a checagem de
    // foco NÃO pode congelar — devolve Err e o chamador cai pro fallback.
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
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
