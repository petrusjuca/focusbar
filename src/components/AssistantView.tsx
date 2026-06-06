import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

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
  const [available, setAvailable] = useState<boolean | null>(null);
  const [loading, setLoading] = useState(false);
  const [review, setReview] = useState<string>("");
  const [err, setErr] = useState<string | null>(null);

  useEffect(() => {
    invoke<boolean>("ai_available")
      .then(setAvailable)
      .catch(() => setAvailable(false));
  }, []);

  async function gen() {
    setLoading(true);
    setErr(null);
    try {
      const r = await invoke<string>("ai_day_review", { day: null });
      setReview(r);
      setAvailable(true);
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

      {available === false && (
        <div className="permission-banner">
          <div className="perm-title">Modelo não está acessível</div>
          <p className="perm-text">
            Inicie o Ollama local (ou aponte pra VPS via FOCUSBAR_LLM_URL). O resumo
            usa o Llama; o porteiro limpa dados sensíveis antes de enviar.
          </p>
        </div>
      )}

      <button className="grant-btn" onClick={gen} disabled={loading}>
        {loading ? "pensando… (o modelo pode levar alguns segundos)" : "Gerar resumo do dia com IA"}
      </button>

      {err && <p className="error">{err}</p>}

      {review && <div className="ai-review">{renderMd(review)}</div>}

      <p className="bg-note" style={{ marginTop: "1rem" }}>
        Roda no teu modelo local (ou na tua VPS). Nada vai pra terceiros; o porteiro
        redige senha/CPF/cartão/token e pula apps de banco/senha.
      </p>
    </section>
  );
}
