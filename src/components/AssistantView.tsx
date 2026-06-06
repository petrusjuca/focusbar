import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";

interface AiStatus {
  running: boolean;
  model: boolean;
}

// Render mínimo de markdown (##, ###, -, parágrafos) — sem dependência extra.
function renderMd(text: string) {
  return text.split("\n").map((line, i) => {
    const t = line.trim();
    if (t.startsWith("## ")) return <h3 key={i} className="ai-h">{t.slice(3)}</h3>;
    if (t.startsWith("### ")) return <h4 key={i} className="ai-h">{t.slice(4)}</h4>;
    if (t.startsWith("- ") || t.startsWith("* "))
      return <li key={i} className="ai-li">{t.slice(2)}</li>;
    if (!t) return <div key={i} style={{ height: 6 }} />;
    return <p key={i} className="ai-p">{t}</p>;
  });
}

export function AssistantView() {
  const [status, setStatus] = useState<AiStatus | null>(null);
  const [pulling, setPulling] = useState(false);
  const [loading, setLoading] = useState(false);
  const [review, setReview] = useState<string>("");
  const [err, setErr] = useState<string | null>(null);

  async function refreshStatus() {
    try {
      setStatus(await invoke<AiStatus>("ai_status"));
    } catch {
      setStatus({ running: false, model: false });
    }
  }

  useEffect(() => {
    refreshStatus();
  }, []);

  async function downloadModel() {
    setPulling(true);
    setErr(null);
    try {
      await invoke("ai_pull_model");
      await refreshStatus();
    } catch (e) {
      setErr(String(e));
    }
    setPulling(false);
  }

  async function gen() {
    setLoading(true);
    setErr(null);
    try {
      const r = await invoke<string>("ai_day_review", { day: null });
      setReview(r);
    } catch (e) {
      setErr(String(e));
    }
    setLoading(false);
  }

  return (
    <section className="daily">
      <div className="daily-header">
        <span>ASSISTENTE (IA LOCAL)</span>
      </div>

      {/* Estado 1: Ollama não está rodando/instalado */}
      {status && !status.running && (
        <div className="permission-banner">
          <div className="perm-title">Ative a IA local (uma vez)</div>
          <p className="perm-text">
            A IA roda na sua própria máquina pelo <b>Ollama</b> (grátis). Instale e
            abra o Ollama — depois é só voltar aqui.
          </p>
          <button
            className="grant-btn"
            onClick={() => openUrl("https://ollama.com/download")}
          >
            Baixar o Ollama
          </button>
          <button
            className="link-btn"
            style={{ marginLeft: 12 }}
            onClick={refreshStatus}
          >
            já instalei, verificar
          </button>
        </div>
      )}

      {/* Estado 2: Ollama ok, falta baixar o modelo */}
      {status && status.running && !status.model && (
        <div className="permission-banner">
          <div className="perm-title">Baixar a IA (uma vez, ~2GB)</div>
          <p className="perm-text">
            Falta baixar o modelo que roda na sua máquina. É um clique — depois
            funciona pra sempre, offline.
          </p>
          <button className="grant-btn" onClick={downloadModel} disabled={pulling}>
            {pulling ? "baixando a IA… (pode levar alguns minutos)" : "Baixar a IA"}
          </button>
        </div>
      )}

      {/* Estado 3: tudo pronto */}
      {status && status.running && status.model && (
        <button className="grant-btn" onClick={gen} disabled={loading}>
          {loading ? "pensando… (alguns segundos)" : "Gerar resumo do dia com IA"}
        </button>
      )}

      {err && <p className="error">{err}</p>}
      {review && <div className="ai-review">{renderMd(review)}</div>}

      <p className="bg-note" style={{ marginTop: "1rem" }}>
        Roda no seu computador. Nada vai pra terceiros; o porteiro redige
        senha/CPF/cartão e pula apps de banco/senha.
      </p>
    </section>
  );
}
