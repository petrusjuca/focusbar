import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";

interface AiStatus {
  running: boolean;
  model: boolean;
}

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
  const [copied, setCopied] = useState(false);
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
      setReview(await invoke<string>("ai_day_review", { day: null }));
    } catch (e) {
      setErr(String(e));
    }
    setLoading(false);
  }

  async function copyForClaude() {
    setErr(null);
    try {
      const digest = await invoke<string>("ai_day_digest", { day: null });
      await navigator.clipboard.writeText(digest);
      setCopied(true);
      setTimeout(() => setCopied(false), 2500);
    } catch (e) {
      setErr(String(e));
    }
  }

  return (
    <section className="daily">
      <div className="daily-header">
        <span>ASSISTENTE</span>
      </div>

      {/* Opção A: IA forte (Claude/etc.) — sempre disponível, sem instalar nada */}
      <div className="ai-card">
        <div className="ai-card-title">Analisar com uma IA forte (ex.: Claude)</div>
        <p className="ai-card-text">
          Copia o resumo do seu dia (já limpo: sem senha/CPF/banco) pra você colar
          no Claude.ai e ter uma análise inteligente. Grátis no seu plano.
        </p>
        <button className="grant-btn" onClick={copyForClaude}>
          {copied ? "copiado! cole no Claude ✓" : "Copiar resumo do dia"}
        </button>
      </div>

      <div className="daily-header" style={{ marginTop: "1.5rem" }}>
        <span>OU IA LOCAL (NO SEU PC)</span>
      </div>

      {/* Estado 1: Ollama não está rodando */}
      {status && !status.running && (
        <div className="permission-banner">
          <div className="perm-title">A IA local não está ativa</div>
          <p className="perm-text">
            A IA local roda pelo <b>Ollama</b> (grátis). Se já instalou, <b>abra o
            app Ollama</b> (ele precisa estar aberto). Pra conferir, acesse{" "}
            <b>http://localhost:11434</b> no navegador — deve dizer "Ollama is running".
          </p>
          <button
            className="grant-btn"
            onClick={() => openUrl("https://ollama.com/download")}
          >
            Baixar o Ollama
          </button>
          <button className="link-btn" style={{ marginLeft: 12 }} onClick={refreshStatus}>
            já abri, verificar
          </button>
        </div>
      )}

      {/* Estado 2: falta baixar o modelo */}
      {status && status.running && !status.model && (
        <div className="permission-banner">
          <div className="perm-title">Baixar a IA (uma vez, ~2GB)</div>
          <p className="perm-text">Um clique e funciona pra sempre, offline.</p>
          <button className="grant-btn" onClick={downloadModel} disabled={pulling}>
            {pulling ? "baixando a IA… (alguns minutos)" : "Baixar a IA"}
          </button>
        </div>
      )}

      {/* Estado 3: pronto */}
      {status && status.running && status.model && (
        <button className="grant-btn" onClick={gen} disabled={loading}>
          {loading ? "pensando…" : "Gerar resumo do dia (IA local)"}
        </button>
      )}

      {err && <p className="error">{err}</p>}
      {review && <div className="ai-review">{renderMd(review)}</div>}

      <p className="bg-note" style={{ marginTop: "1rem" }}>
        O porteiro redige senha/CPF/cartão e pula apps de banco/senha antes de
        qualquer análise.
      </p>
    </section>
  );
}
