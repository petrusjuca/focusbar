import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type AiStatus = { running: boolean; model: boolean };

// Auto-diagnóstico do agente: os 3 "sentidos" estão funcionando?
// Tudo ok → uma linha discreta. Algo quebrado → aviso clicável que conserta.
export function SelfCheck() {
  const [ax, setAx] = useState<boolean | null>(null);
  const [screen, setScreen] = useState(false);
  const [ocrOn, setOcrOn] = useState(false);
  const [ai, setAi] = useState<AiStatus | null>(null);

  async function refresh() {
    try {
      const [a, s, o, st] = await Promise.all([
        invoke<boolean>("check_accessibility"),
        invoke<boolean>("check_screen_recording"),
        invoke<boolean>("get_ocr_enabled"),
        invoke<AiStatus>("ai_status"),
      ]);
      setAx(a);
      setScreen(s);
      setOcrOn(o);
      setAi(st);
    } catch {
      /* ignore */
    }
  }

  useEffect(() => {
    refresh();
    const id = setInterval(refresh, 60000);
    return () => clearInterval(id);
  }, []);

  if (ax === null) return null;

  const sensors = [
    {
      ok: ax,
      label: "visão (títulos das janelas)",
      hint: "Sem Acessibilidade o agente não enxerga o que você faz. Clique pra conceder.",
      fix: () => invoke("request_accessibility"),
    },
    {
      // OCR desligado de propósito = ok; ligado sem permissão = problema.
      ok: !ocrOn || screen,
      label: "visão profunda (OCR)",
      hint: "OCR ligado mas sem Gravação de Tela. Clique pra conceder (reabra o app depois).",
      fix: () => invoke("request_screen_recording"),
    },
    {
      ok: !!ai?.running && !!ai?.model,
      label: "cérebro local (IA)",
      hint: !ai?.running
        ? "Ollama desligado — o agente julga só por regras simples. Clique pra ligar."
        : "Modelo não baixado — baixe na aba Assistente.",
      fix: () => invoke("start_ollama"),
    },
  ];

  const bad = sensors.filter((s) => !s.ok);
  if (bad.length === 0) {
    return (
      <div className="selfcheck-ok" title="Acessibilidade, OCR e IA local conferidos agora">
        ✓ agente saudável — todos os sentidos ativos
      </div>
    );
  }
  return (
    <div className="selfcheck">
      {bad.map((s) => (
        <button
          key={s.label}
          className="selfcheck-item"
          title={s.hint}
          onClick={() => {
            s.fix();
            setTimeout(refresh, 1500);
          }}
        >
          ⚠ {s.label} — clique pra ativar
        </button>
      ))}
    </div>
  );
}
