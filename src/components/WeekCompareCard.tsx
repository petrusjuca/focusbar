import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { CategoryTotal } from "../types";
import { fmtDuration } from "../format";

interface WeekCompare {
  this_start: string;
  this_secs: number;
  prev_same_span_secs: number;
  prev_full_secs: number;
  this_by_cat: CategoryTotal[];
  prev_by_cat: CategoryTotal[];
}

// Análise da semana CIVIL (seg–dom) vs a anterior — FLOWMODE "possibilidade 2":
// a comparação evolui ao longo da semana. Maçãs com maçãs: quarta compara
// seg–qua desta semana com seg–qua da passada.
export function WeekCompareCard() {
  const [cmp, setCmp] = useState<WeekCompare | null>(null);

  useEffect(() => {
    invoke<WeekCompare>("get_week_compare").then(setCmp).catch(() => {});
  }, []);

  if (!cmp || (cmp.this_secs === 0 && cmp.prev_same_span_secs === 0)) return null;

  const delta = cmp.this_secs - cmp.prev_same_span_secs;
  const pct =
    cmp.prev_same_span_secs > 0
      ? Math.round((delta * 100) / cmp.prev_same_span_secs)
      : null;
  const up = delta >= 0;

  // Maior mudança por categoria (mesmo trecho das duas semanas).
  const prevMap = new Map(cmp.prev_by_cat.map((c) => [c.category, c.total_secs]));
  const catSet = new Set([
    ...cmp.this_by_cat.map((c) => c.category),
    ...cmp.prev_by_cat.map((c) => c.category),
  ]);
  const thisMap = new Map(cmp.this_by_cat.map((c) => [c.category, c.total_secs]));
  let bigCat = "";
  let bigDelta = 0;
  for (const cat of catSet) {
    const d = (thisMap.get(cat) ?? 0) - (prevMap.get(cat) ?? 0);
    if (Math.abs(d) > Math.abs(bigDelta)) {
      bigDelta = d;
      bigCat = cat;
    }
  }

  return (
    <div className="week-compare">
      <div className="daily-header">
        <span>ESTA SEMANA × SEMANA PASSADA</span>
        <span className="daily-day">mesmo trecho (desde segunda)</span>
      </div>
      <p className="week-compare-main">
        {up ? "📈" : "📉"} <b>{fmtDuration(cmp.this_secs)}</b> até agora —{" "}
        {up ? "+" : "−"}
        {fmtDuration(Math.abs(delta))}
        {pct !== null && ` (${up ? "+" : ""}${pct}%)`} vs o mesmo trecho da
        semana passada ({fmtDuration(cmp.prev_same_span_secs)}).
      </p>
      {bigCat && Math.abs(bigDelta) >= 900 && (
        <p className="week-compare-sub">
          Maior mudança: <b>{bigCat}</b> {bigDelta > 0 ? "+" : "−"}
          {fmtDuration(Math.abs(bigDelta))}.
        </p>
      )}
      <p className="week-compare-sub">
        Semana passada inteira: {fmtDuration(cmp.prev_full_secs)}.
      </p>
    </div>
  );
}
