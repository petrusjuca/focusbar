//! Cliente do modelo local (Ollama / Llama 3.2 3B). Tudo roda na máquina.
//! O modelo é carregado sob demanda pelo Ollama e descarregado depois (keep_alive).

use serde::{Deserialize, Serialize};

/// URL do Ollama. Local por padrão; aponte pra VPS via env FOCUSBAR_LLM_URL
/// (ex.: http://127.0.0.1:11434 num túnel SSH pra VPS). Sem recompilar.
fn base() -> String {
    std::env::var("FOCUSBAR_LLM_URL").unwrap_or_else(|_| "http://localhost:11434".to_string())
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

/// Gera texto a partir de um prompt. keep_alive curto = descarrega rápido da RAM.
pub async fn generate(prompt: String) -> Result<String, String> {
    let body = GenReq {
        model: model(),
        prompt,
        stream: false,
        keep_alive: "2m".to_string(),
        options: GenOpts {
            temperature: 0.3,
            num_predict: 700,
        },
    };
    let resp = reqwest::Client::new()
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
