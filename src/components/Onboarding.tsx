import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

// Primeira tela (30s): nada de despejar o dashboard. Uma frase + uma pergunta.
// O resto aparece quando for relevante. Marca "onboarded" e some pra sempre.
export function Onboarding({ onDone }: { onDone: () => void }) {
  const [focus, setFocus] = useState("");
  const [busy, setBusy] = useState(false);

  async function finish() {
    setBusy(true);
    if (focus.trim()) {
      try {
        await invoke("set_focus", { text: focus.trim() });
      } catch {
        /* ignore */
      }
    }
    try {
      await invoke("set_setting", { key: "onboarded", value: "1" });
    } catch {
      /* ignore */
    }
    onDone();
  }

  return (
    <main className="container onboarding">
      <div className="ob-card">
        <div className="ob-hi">oi, eu sou o focusbar 👋</div>
        <p className="ob-sub">
          Eu fico num cantinho da tela e te ajudo a focar — sem cobrança, sem te
          encher. Quando você se perder, eu te puxo de volta com jeitinho.
        </p>
        <label className="ob-q">No que você quer focar agora?</label>
        <input
          className="ob-input"
          autoFocus
          placeholder="ex: terminar o relatório, estudar cálculo…"
          value={focus}
          onChange={(e) => setFocus(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && !busy && finish()}
        />
        <button className="ob-go" onClick={finish} disabled={busy}>
          {focus.trim() ? "Bora 🚀" : "Pular por agora"}
        </button>
        <p className="ob-foot">
          Prometo ser discreto. O resto do app aparece só quando precisar.
        </p>
      </div>
    </main>
  );
}
