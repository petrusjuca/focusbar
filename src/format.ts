export function fmtDuration(secs: number): string {
  if (secs < 60) return `${secs}s`;
  const m = Math.floor(secs / 60);
  const s = secs % 60;
  if (m < 60) return s ? `${m}m ${s}s` : `${m}m`;
  const h = Math.floor(m / 60);
  return `${h}h ${m % 60}m`;
}

export function fmtTime(ts: number): string {
  return new Date(ts * 1000).toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
  });
}

/** Segundos → "MM:SS" (pro cronômetro do Modo Foco). */
export function fmtClock(secs: number): string {
  const s = Math.max(0, Math.floor(secs));
  const m = Math.floor(s / 60);
  const r = s % 60;
  return `${String(m).padStart(2, "0")}:${String(r).padStart(2, "0")}`;
}

/** Mensagem de erro amigável. Os erros do backend já vêm em PT; só limpa prefixos. */
export function friendlyError(e: unknown): string {
  const msg =
    typeof e === "string" ? e : e instanceof Error ? e.message : String(e);
  return msg.replace(/^Error:\s*/i, "").trim() || "Algo deu errado. Tente de novo.";
}
