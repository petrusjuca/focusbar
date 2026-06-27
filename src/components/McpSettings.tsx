import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { friendlyError } from "../format";

type McpInfo = { path: string; exists: boolean };

/**
 * "Claude lê seus dados sem API". O focusbar traz um servidor MCP local (binário
 * `mcp` ao lado do app) que lê o SQLite e responde sobre seu foco. Você aponta o
 * SEU Claude (Desktop ou Code) pra ele — nada de API paga no app. Aqui a gente
 * só mostra o caminho e os comandos prontos pra copiar.
 */
export function McpSettings() {
  const [info, setInfo] = useState<McpInfo | null>(null);
  const [copied, setCopied] = useState("");
  const [err, setErr] = useState("");

  useEffect(() => {
    invoke<McpInfo>("get_mcp_info").then(setInfo).catch((e) => setErr(friendlyError(e)));
  }, []);

  const path = info?.path ?? "";
  const cliCmd = `claude mcp add focusbar -- "${path}"`;
  const desktopJson = JSON.stringify(
    { mcpServers: { focusbar: { command: path } } },
    null,
    2
  );

  async function copy(text: string, which: string) {
    try {
      await writeText(text);
      setCopied(which);
      setTimeout(() => setCopied(""), 4000);
    } catch (e) {
      setErr(friendlyError(e));
    }
  }

  return (
    <div className="ai-card" style={{ marginTop: "1rem" }}>
      <div className="ai-card-title">🔌 MCP — Claude lê seus dados (sem API)</div>
      <p className="ai-card-text">
        O focusbar tem um <b>servidor MCP local</b>: aponte seu Claude (Desktop ou
        Code) pra ele e pergunte <i>"como foi meu foco hoje?"</i>. Ele lê o banco
        local — <b>nada sai do seu Mac</b> e <b>não usa API paga</b>.
      </p>

      {info && !info.exists && (
        <p className="perm-hint" style={{ color: "var(--warn)" }}>
          ⚠️ O binário <code>mcp</code> não está ao lado do app. Rode o build mais
          recente (<code>build-mac.sh</code>) — ele empacota o servidor no bundle.
        </p>
      )}

      <div className="mcp-block">
        <div className="mcp-label">No Claude Code (terminal):</div>
        <code className="mcp-code">{cliCmd}</code>
        <button className="link-btn" onClick={() => copy(cliCmd, "cli")}>
          {copied === "cli" ? "✓ copiado" : "copiar"}
        </button>
      </div>

      <div className="mcp-block">
        <div className="mcp-label">No Claude Desktop (claude_desktop_config.json):</div>
        <pre className="mcp-code mcp-pre">{desktopJson}</pre>
        <button className="link-btn" onClick={() => copy(desktopJson, "json")}>
          {copied === "json" ? "✓ copiado" : "copiar JSON"}
        </button>
      </div>

      <p className="ai-card-text" style={{ marginTop: "0.5rem", opacity: 0.8 }}>
        Ferramentas: resumo do dia, blocos do dia, pomodoros e resumo da semana.
      </p>

      {err && <p className="error">{err}</p>}
    </div>
  );
}
