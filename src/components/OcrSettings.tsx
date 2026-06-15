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
