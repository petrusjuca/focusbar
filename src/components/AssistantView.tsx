import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { CopyToClaudeButton } from "./CopyToClaudeButton";
import { OcrSettings } from "./OcrSettings";
import { McpSettings } from "./McpSettings";
import { friendlyError } from "../format";

interface AiStatus {
  running: boolean;
  model: boolean;
}

/** Data de ontem no formato YYYY-MM-DD (fuso local). */
function yesterday(): string {
  const d = new Date(Date.now() - 86_400_000);
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`;
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
  const [starting, setStarting] = useState(false);
  const [pulling, setPulling] = useState(false);
  const [loading, setLoading] = useState(false);
  const [review, setReview] = useState<string>("");
  const [day, setDay] = useState<string | null>(null); // null = hoje
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

  async function startOllama() {
    setStarting(true);
    try {
      await invoke("start_ollama");
    } catch {
      /* ignore */
    }
    // dá um tempo do servidor subir e re-checa algumas vezes
    for (let i = 0; i < 8; i++) {
      await new Promise((r) => setTimeout(r, 1000));
      try {
        const s = await invoke<AiStatus>("ai_status");
        if (s.running) {
          setStatus(s);
          break;
        }
      } catch {
        /* ignore */
      }
    }
    setStarting(false);
    refreshStatus();
  }

  async function downloadModel() {
    setPulling(true);
    setErr(null);
    try {
      await invoke("ai_pull_model");
      await refreshStatus();
    } catch (e) {
      setErr(friendlyError(e));
    }
    setPulling(false);
  }

  async function gen() {
    setLoading(true);
    setErr(null);
    try {
      setReview(await invoke<string>("ai_day_review", { day }));
    } catch (e) {
      setErr(friendlyError(e));
    }
    setLoading(false);
  }

  return (
    <section className="daily">
      <div className="daily-header">
        <span>ASSISTENTE</span>
      </div>

      {/* Opção A: IA forte (Claude.ai) — sempre disponível, grátis, sem instalar nada */}
      <div className="ai-card">
        <div className="ai-card-title">Analisar com o Claude.ai (recomendado)</div>
        <p className="ai-card-text">
          Monta o resumo do seu dia (já limpo: sem senha/CPF/banco), copia e abre o
          Claude.ai num clique — você só cola (Cmd+V). Grátis no seu plano.
        </p>
        <div className="day-toggle">
          <button
            className={day === null ? "day-pill active" : "day-pill"}
            onClick={() => setDay(null)}
          >
            Hoje
          </button>
          <button
            className={day !== null ? "day-pill active" : "day-pill"}
            onClick={() => setDay(yesterday())}
          >
            Ontem
          </button>
        </div>
        <CopyToClaudeButton day={day} />
      </div>

      <div className="daily-header" style={{ marginTop: "1.5rem" }}>
        <span>OU IA LOCAL (NO SEU PC)</span>
      </div>

      {/* Estado 1: Ollama não está rodando */}
      {status && !status.running && (
        <div className="permission-banner">
          <div className="perm-title">Ligar a IA local (Ollama) — 3 passos</div>
          <p className="perm-text">
            A IA local roda pelo <b>Ollama</b> (grátis, fica 100% no seu PC). Faça
            uma vez:
          </p>
          <ol className="perm-text" style={{ margin: "0 0 0.6rem 1.1rem", padding: 0 }}>
            <li>
              <b>Baixe e instale o Ollama</b> (botão abaixo). Depois de instalar ele
              <b> roda sozinho</b> — fica na bandeja, não precisa abrir nada.
            </li>
            <li>
              Volte aqui e clique <b>verificar</b> (deve ficar verde). Se já instalou
              e não conectar, clique <b>tentar ligar</b>.
            </li>
            <li>
              Clique <b>baixar a IA</b> (aparece no próximo passo, ~2GB, uma vez só).
            </li>
          </ol>
          <p className="perm-text" style={{ fontSize: "0.8rem", opacity: 0.8 }}>
            Conferir manualmente: abra <b>http://localhost:11434</b> no navegador —
            deve dizer "Ollama is running".
          </p>
          <button
            className="grant-btn"
            onClick={() => openUrl("https://ollama.com/download")}
          >
            ⬇ Baixar o Ollama (grátis)
          </button>
          <button className="grant-btn" style={{ marginLeft: 12 }} onClick={refreshStatus}>
            ✓ verificar
          </button>
          <button
            className="link-btn"
            style={{ marginLeft: 12 }}
            onClick={startOllama}
            disabled={starting}
            title="Se já instalou e mesmo assim não conecta"
          >
            {starting ? "ligando…" : "tentar ligar"}
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

      <div className="daily-header" style={{ marginTop: "1.5rem" }}>
        <span>OLHOS (OCR)</span>
      </div>
      <OcrSettings />

      <div className="daily-header" style={{ marginTop: "1.5rem" }}>
        <span>CLAUDE LÊ SEUS DADOS (MCP)</span>
      </div>
      <McpSettings />

      <p className="bg-note" style={{ marginTop: "1rem" }}>
        O porteiro redige senha/CPF/cartão e pula apps de banco/senha antes de
        qualquer análise.
      </p>
    </section>
  );
}
