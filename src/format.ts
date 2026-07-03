/** Data de HOJE no fuso LOCAL ("YYYY-MM-DD"). NÃO usar toISOString().slice():
 *  ela é UTC — no Brasil, depois das 21h o "dia" viraria amanhã. Era isso que
 *  fazia o ritual da manhã reaparecer às 21h perguntando a intenção de novo
 *  (bug Rev4 #6): as chaves de "hoje não"/"acabou por hoje" expiravam à noite. */
export function todayLocal(): string {
  const d = new Date();
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`;
}

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

/** Segundos → "MM:SS" (ou "H:MM:SS" se >= 1h) pro cronômetro do Modo Foco. */
export function fmtClock(secs: number): string {
  const s = Math.max(0, Math.floor(secs));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const r = s % 60;
  const mm = String(m).padStart(2, "0");
  const ss = String(r).padStart(2, "0");
  return h > 0 ? `${h}:${mm}:${ss}` : `${mm}:${ss}`;
}

/**
 * Interpreta um tempo digitado livremente (estilo timer do Google) → segundos.
 * Aceita: "90" (=90min), "25" (=25min), "1:30" (mm:ss), "1:30:00" (h:mm:ss),
 * "1h30", "1h30m", "90m", "45s", "1h", "2h15m30s". Retorna null se não entender.
 */
export function parseDuration(input: string): number | null {
  const t = input.trim().toLowerCase();
  if (!t) return null;

  // Forma com dois-pontos: mm:ss ou hh:mm:ss
  if (t.includes(":")) {
    const parts = t.split(":").map((x) => parseInt(x, 10));
    if (parts.some((n) => isNaN(n) || n < 0)) return null;
    let secs: number;
    if (parts.length === 2) secs = parts[0] * 60 + parts[1];
    else if (parts.length === 3) secs = parts[0] * 3600 + parts[1] * 60 + parts[2];
    else return null;
    return secs > 0 ? secs : null;
  }

  // Atalho "1h30" = 1h e 30min
  const hm = t.match(/^(\d+)\s*h\s*(\d+)$/);
  if (hm) {
    const s = (parseInt(hm[1], 10) * 60 + parseInt(hm[2], 10)) * 60;
    return s > 0 ? s : null;
  }

  // Forma com unidades: 2h15m30s, 90m, 45s, 1h…
  const re = /(\d+)\s*(h|hora|horas|m|min|minutos?|s|seg|segundos?)/g;
  let secs = 0;
  let matched = false;
  let mm: RegExpExecArray | null;
  while ((mm = re.exec(t))) {
    matched = true;
    const n = parseInt(mm[1], 10);
    const u = mm[2][0];
    if (u === "h") secs += n * 3600;
    else if (u === "s") secs += n;
    else secs += n * 60;
  }
  if (matched) return secs > 0 ? secs : null;

  // Número puro = minutos.
  const n = parseInt(t, 10);
  return !isNaN(n) && n > 0 ? n * 60 : null;
}

/** Mensagem de erro amigável. Os erros do backend já vêm em PT; só limpa prefixos. */
export function friendlyError(e: unknown): string {
  const msg =
    typeof e === "string" ? e : e instanceof Error ? e.message : String(e);
  return msg.replace(/^Error:\s*/i, "").trim() || "Algo deu errado. Tente de novo.";
}
