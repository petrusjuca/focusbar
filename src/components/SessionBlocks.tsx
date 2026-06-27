import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { FocusSession } from "../types";
import { fmtDuration, fmtTime } from "../format";
import { CATEGORIES, catColor } from "../categories";

// Só lista blocos com algum peso — fragmentos de poucos segundos só fariam ruído.
const MIN_SECS = 60;

/**
 * Lista os blocos do dia com correção de categoria em 1 clique. Quando a IA
 * (ou a regra por app) erra a categoria de um bloco, você ajusta aqui e fica
 * salvo só pra AQUELE bloco — não vira regra geral pro app inteiro.
 */
export function SessionBlocks({
  sessions,
  onChanged,
}: {
  sessions: FocusSession[];
  onChanged?: () => void;
}) {
  const [busy, setBusy] = useState<number | null>(null);

  // O cálculo da fronteira usa a lista cheia (índice real); só o display filtra.
  const visible = sessions
    .map((s, i) => ({ s, i }))
    .filter(({ s }) => s.duration_secs >= MIN_SECS);
  if (visible.length === 0) return null;

  async function setCat(i: number, category: string) {
    const block = sessions[i];
    const now = Math.floor(Date.now() / 1000);
    // Fronteira = começo do próximo bloco (ou agora+1 se for o último), pra
    // pegar exatamente as sessões deste bloco e nenhuma a mais.
    const until = i + 1 < sessions.length ? sessions[i + 1].start_ts : now + 1;
    setBusy(i);
    try {
      await invoke("set_block_category", {
        app: block.app_name,
        startTs: block.start_ts,
        untilTs: until,
        category,
      });
      onChanged?.();
    } finally {
      setBusy(null);
    }
  }

  return (
    <section className="blocks">
      <div className="daily-header">
        <span>BLOCOS DE HOJE</span>
        <span className="daily-day">categoria errada? ajuste no toque</span>
      </div>
      <div className="block-list">
        {visible.map(({ s, i }) => {
          const cat = s.category_ai ?? "";
          const label = s.activity_ai || s.app_name;
          return (
            <div className="block-row" key={i}>
              <span className="block-time">{fmtTime(s.start_ts)}</span>
              <span className="block-label" title={s.title}>
                {label}
                {s.activity_ai && <span className="block-app"> · {s.app_name}</span>}
              </span>
              <span className="block-dur">{fmtDuration(s.duration_secs)}</span>
              <span
                className="block-dot"
                style={{
                  background: cat ? catColor(cat) : "transparent",
                  borderColor: cat ? catColor(cat) : "var(--border, #d0d0d5)",
                }}
              />
              <select
                className="block-cat"
                value={CATEGORIES.includes(cat) ? cat : ""}
                disabled={busy === i}
                onChange={(e) => setCat(i, e.target.value)}
              >
                <option value="">automático</option>
                {CATEGORIES.map((c) => (
                  <option key={c} value={c}>
                    {c}
                  </option>
                ))}
              </select>
            </div>
          );
        })}
      </div>
    </section>
  );
}
