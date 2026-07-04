import { useState } from "react";
import { CopyToClaudeButton } from "./CopyToClaudeButton";
import { OcrSettings } from "./OcrSettings";
import { McpSettings } from "./McpSettings";

// Aba CONFIGURAÇÕES (ex-"Assistente", renomeada — FLOWMODE): modos do dia com
// as diferenças EXPLÍCITAS, conexão com o Claude (MCP), OCR, tema e autostart.
// O app não roda IA local (D2): captura e organiza; quem pensa é o Claude.

/** Data de ontem no formato YYYY-MM-DD (fuso local). */
function yesterday(): string {
  const d = new Date(Date.now() - 86_400_000);
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`;
}

const MODES: { key: string; name: string; what: string; when: string }[] = [
  {
    key: "companheiro",
    name: "🤝 Companheiro",
    what: "Te acompanha e só cutuca depois de ~20min disperso. Uma cutucada por vez, sempre gentil.",
    when: "O modo padrão pro dia a dia.",
  },
  {
    key: "foco",
    name: "🎯 Foco",
    what: "Cutuca mais cedo (~10min disperso). Pra quando você quer proteção extra contra a dispersão.",
    when: "Dia de prazo apertado ou tarefa difícil.",
  },
  {
    key: "dia_ruim",
    name: "🌧️ Dia ruim",
    what: "ZERO cobrança: o app observa em silêncio e não cutuca nada hoje. O registro continua normal.",
    when: "Dia difícil — sem culpa, amanhã a gente volta.",
  },
];

export function AssistantView({
  mode,
  onMode,
  theme,
  onToggleTheme,
  autostart,
  onToggleAutostart,
}: {
  mode: string;
  onMode: (m: string) => void;
  theme: "dark" | "light";
  onToggleTheme: () => void;
  autostart: boolean;
  onToggleAutostart: () => void;
}) {
  const [day, setDay] = useState<string | null>(null); // null = hoje

  return (
    <section className="daily">
      <div className="daily-header">
        <span>MODO DE HOJE</span>
      </div>
      <div className="mode-cards">
        {MODES.map((m) => (
          <button
            key={m.key}
            className={mode === m.key ? "mode-card active" : "mode-card"}
            onClick={() => onMode(m.key)}
          >
            <div className="mode-card-name">
              {m.name}
              {mode === m.key && <span className="mode-card-on"> · ativo</span>}
            </div>
            <div className="mode-card-what">{m.what}</div>
            <div className="mode-card-when">{m.when}</div>
          </button>
        ))}
      </div>

      <div className="daily-header" style={{ marginTop: "1.5rem" }}>
        <span>CLAUDE — O CÉREBRO DO ASSISTENTE (MCP)</span>
      </div>
      <p className="bg-note">
        O focusbar captura e organiza o seu dia (tudo local). A análise profunda
        — "como foi meu dia?", "onde perdi tempo?" — é do Claude, com o dia
        inteiro de contexto. Conecte uma vez:
      </p>
      <McpSettings />

      <div className="daily-header" style={{ marginTop: "1.5rem" }}>
        <span>ANALISAR COM O CLAUDE.AI (SEM MCP — até a v1.0)</span>
      </div>
      <div className="ai-card">
        <p className="ai-card-text">
          Monta o resumo do dia (já limpo: sem senha/CPF/banco), copia e abre o
          Claude.ai num clique — você só cola (Cmd+V).
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

      <div className="daily-header" style={{ marginTop: "1.5rem" }}>
        <span>GERAL</span>
      </div>
      <div className="settings-general">
        <button className="mini-toggle" onClick={onToggleTheme}>
          {theme === "dark" ? "☀️ Mudar pro tema claro" : "🌙 Mudar pro tema escuro"}
        </button>
        <label className="autostart">
          <input type="checkbox" checked={autostart} onChange={onToggleAutostart} />
          iniciar com o sistema
        </label>
      </div>

      <p className="bg-note" style={{ marginTop: "1rem" }}>
        O porteiro redige senha/CPF/cartão e pula apps de banco/senha antes de
        qualquer análise.
      </p>
    </section>
  );
}
