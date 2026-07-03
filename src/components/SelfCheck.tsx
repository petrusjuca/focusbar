import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

// Auto-diagnóstico do agente: os "sentidos" estão funcionando?
// Tudo ok → uma linha discreta. Algo quebrado → aviso clicável que conserta.
// (O sensor de "IA local" saiu junto com o Ollama — decisão D2: quem pensa é
// o Claude via MCP; os sentidos aqui são captura, OCR e extensão de abas.)
export function SelfCheck() {
  const [ax, setAx] = useState<boolean | null>(null);
  const [screen, setScreen] = useState(false);
  const [ocrOn, setOcrOn] = useState(false);
  const [ocrWorks, setOcrWorks] = useState<boolean | null>(null);
  const [extLast, setExtLast] = useState<number>(0);

  async function refresh() {
    try {
      const [a, s, o, sel, ext] = await Promise.all([
        invoke<boolean>("check_accessibility"),
        invoke<boolean>("check_screen_recording"),
        invoke<boolean>("get_ocr_enabled"),
        // Health-check REAL: o app OCRa a própria tela no startup e grava o
        // resultado. "ok:N" = leu de verdade; "falhou" = capturou vazio.
        invoke<string | null>("get_setting", { key: "ocr_selftest" }),
        invoke<number>("get_extension_last_event"),
      ]);
      setAx(a);
      setScreen(s);
      setOcrOn(o);
      setOcrWorks(sel == null ? null : sel.startsWith("ok:"));
      setExtLast(ext);
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

  // Extensão é OPCIONAL (no Mac o app lê URL sozinho): nunca conectou → não
  // alarma. Mas se JÁ reportava e ficou muda por 1h+, o sentido morreu — avisa.
  const nowS = Math.floor(Date.now() / 1000);
  const extDead = extLast > 0 && nowS - extLast > 3600;

  const sensors = [
    {
      ok: ax,
      label: "visão (títulos das janelas)",
      hint: "Sem Acessibilidade o agente não enxerga o que você faz. Clique pra conceder.",
      fix: () => invoke("request_accessibility"),
    },
    {
      // Verifica de VERDADE: o health-check leu a tela? OCR desligado = ok.
      // Ligado: ok a menos que o teste tenha FALHADO (ocrWorks===false). Null
      // (ainda não testou) não alarma.
      ok: !ocrOn || ocrWorks !== false,
      label: "visão profunda (OCR lê a tela)",
      hint: screen
        ? "OCR ligado mas não consegui LER a tela no último teste. Vá em Assistente → OLHOS → 'Testar os olhos agora'."
        : "OCR ligado mas sem Gravação de Tela. Clique pra conceder (reabra o app depois).",
      fix: () => invoke("request_screen_recording"),
    },
    {
      ok: !extDead,
      label: "extensão do navegador (abas)",
      hint: "A extensão reportava abas e parou há mais de 1h. O navegador está aberto? Ela segue instalada? (extension/README.md)",
      fix: () => Promise.resolve(),
    },
  ];

  const bad = sensors.filter((s) => !s.ok);
  if (bad.length === 0) {
    const extNote =
      extLast === 0
        ? " · extensão de abas: não conectada (opcional)"
        : " · extensão de abas ✓";
    return (
      <div className="selfcheck-ok" title="Acessibilidade, OCR e extensão conferidos agora">
        ✓ agente saudável — sentidos ativos{extNote}
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
