import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { friendlyError } from "../format";

/**
 * Botão 1-clique: monta o resumo do dia (já limpo pelo porteiro), copia pro
 * clipboard e abre uma conversa nova no Claude.ai. O usuário só dá Cmd+V.
 * É a forma GRÁTIS de usar o Claude forte — usa a assinatura do claude.ai.
 */
export function CopyToClaudeButton({
  day = null,
  label = "Analisar meu dia no Claude.ai",
}: {
  day?: string | null;
  label?: string;
}) {
  const [state, setState] = useState<"idle" | "done" | "err">("idle");
  const [msg, setMsg] = useState("");

  async function go() {
    setState("idle");
    setMsg("");
    try {
      const digest = await invoke<string>("ai_day_digest", { day });
      await writeText(digest); // plugin do Tauri (o navigator.clipboard é bloqueado no WKWebView)
      await openUrl("https://claude.ai/new");
      setState("done");
      setTimeout(() => setState("idle"), 5000);
    } catch (e) {
      setMsg(friendlyError(e));
      setState("err");
    }
  }

  return (
    <div className="claude-cta">
      <button className="claude-btn" onClick={go}>
        {state === "done"
          ? "✓ copiado — é só colar (Cmd+V) na aba do Claude"
          : `✨ ${label}`}
      </button>
      <span className="claude-hint">grátis — usa seu plano do claude.ai</span>
      {state === "err" && <p className="error">{msg}</p>}
    </div>
  );
}
