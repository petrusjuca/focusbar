import { useState } from "react";
import { CopyToClaudeButton } from "./CopyToClaudeButton";
import { OcrSettings } from "./OcrSettings";
import { McpSettings } from "./McpSettings";

// O assistente pós-Ollama (decisão D2): o app NÃO roda IA local — ele captura,
// limpa e organiza; quem PENSA é o Claude, via MCP (melhor caminho) ou pelo
// resumo copiado (fallback). Ao vivo, o juiz de foco usa só camadas baratas
// (regra aprendida, match com a intenção, categoria) — rápido e sem alucinar.

/** Data de ontem no formato YYYY-MM-DD (fuso local). */
function yesterday(): string {
  const d = new Date(Date.now() - 86_400_000);
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`;
}

export function AssistantView() {
  const [day, setDay] = useState<string | null>(null); // null = hoje

  return (
    <section className="daily">
      <div className="daily-header">
        <span>ASSISTENTE — QUEM PENSA É O CLAUDE</span>
      </div>
      <p className="bg-note">
        O focusbar captura e organiza o seu dia (tudo local). A análise profunda
        — "como foi meu dia?", "onde perdi tempo?" — é do Claude, com o dia
        inteiro de contexto. Conecte uma vez pelo MCP abaixo.
      </p>
      <McpSettings />

      <div className="daily-header" style={{ marginTop: "1.5rem" }}>
        <span>OU: COPIAR O RESUMO PRO CLAUDE.AI</span>
      </div>
      <div className="ai-card">
        <p className="ai-card-text">
          Sem MCP configurado? Monta o resumo do dia (já limpo: sem senha/CPF/
          banco), copia e abre o Claude.ai num clique — você só cola (Cmd+V).
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
        <span>OLHOS (OCR)</span>
      </div>
      <OcrSettings />

      <p className="bg-note" style={{ marginTop: "1rem" }}>
        O porteiro redige senha/CPF/cartão e pula apps de banco/senha antes de
        qualquer análise.
      </p>
    </section>
  );
}
