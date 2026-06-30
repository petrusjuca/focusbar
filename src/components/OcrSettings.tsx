import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { friendlyError } from "../format";

/**
 * Toggle dos "olhos" Estágio 2 (OCR de pixel). Quando o app em foco não expõe o
 * texto pela Acessibilidade, o focusbar lê a tela por OCR nativo (Apple Vision /
 * Windows OCR) — só a janela em foco, em memória, sem salvar imagem. Exige a
 * permissão de Gravação de Tela.
 */
export function OcrSettings() {
  const [enabled, setEnabled] = useState(false);
  const [granted, setGranted] = useState(true);
  const [msg, setMsg] = useState("");
  const [err, setErr] = useState("");
  const [testing, setTesting] = useState(false);
  const [testResult, setTestResult] = useState<{ ok: boolean; text: string } | null>(null);

  async function testEyes() {
    setTesting(true);
    setTestResult(null);
    try {
      const txt = await invoke<string>("run_ocr_selftest");
      setTestResult({ ok: true, text: txt });
    } catch (e) {
      setTestResult({ ok: false, text: friendlyError(e) });
    } finally {
      setTesting(false);
    }
  }

  useEffect(() => {
    invoke<boolean>("get_ocr_enabled").then(setEnabled).catch(() => {});
    invoke<boolean>("check_screen_recording").then(setGranted).catch(() => {});
  }, []);

  async function toggle() {
    setErr("");
    setMsg("");
    const next = !enabled;
    try {
      if (next) {
        const ok = await invoke<boolean>("check_screen_recording");
        if (!ok) {
          await invoke("request_screen_recording");
          setGranted(false);
          setMsg(
            "Conceda 'Gravação de Tela' pro focusbar em Ajustes → Privacidade e Segurança, depois feche e reabra o app."
          );
        } else {
          setGranted(true);
        }
      }
      await invoke("set_ocr_enabled", { on: next });
      setEnabled(next);
    } catch (e) {
      setErr(friendlyError(e));
    }
  }

  return (
    <div className="ai-card" style={{ marginTop: "1rem" }}>
      <div className="ai-card-title">👁️ OCR — ler a tela (avançado)</div>
      <p className="ai-card-text">
        Quando o app em foco não mostra o texto pela Acessibilidade (alguns PDFs,
        apps sem título útil), o focusbar pode <b>ler a tela por OCR nativo</b>{" "}
        (Apple Vision) — só a janela em foco, em memória, <b>sem salvar imagem</b>.
        O texto ainda passa pelo porteiro (redige senha/CPF) antes de qualquer uso.
      </p>
      <label className="autostart">
        <input type="checkbox" checked={enabled} onChange={toggle} />
        Ligar OCR de tela
      </label>

      <div style={{ marginTop: "0.7rem" }}>
        <button className="link-btn" onClick={testEyes} disabled={testing}>
          {testing ? "🔍 capturando + lendo…" : "🔍 Testar os olhos agora"}
        </button>
        {testResult && (
          <div
            className="ai-card-text"
            style={{
              marginTop: "0.5rem",
              padding: "0.5rem 0.6rem",
              borderRadius: 8,
              background: "rgba(127,127,127,0.12)",
              color: testResult.ok ? "inherit" : "var(--warn)",
            }}
          >
            {testResult.ok ? (
              <>
                <b>✓ Funcionando.</b> Li {testResult.text.length} caracteres da tela
                agora:
                <br />
                <span style={{ opacity: 0.8, fontStyle: "italic" }}>
                  "{testResult.text.slice(0, 180)}…"
                </span>
              </>
            ) : (
              <>✗ {testResult.text}</>
            )}
          </div>
        )}
      </div>
      {enabled && !granted && (
        <p className="perm-hint" style={{ color: "var(--warn)" }}>
          ⚠️ Falta a permissão de Gravação de Tela — conceda e reinicie o app.
        </p>
      )}
      {msg && (
        <p className="ai-card-text" style={{ marginTop: "0.5rem" }}>
          {msg}
        </p>
      )}
      {err && <p className="error">{err}</p>}
    </div>
  );
}
