import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";
import type { FocusCheck } from "../types";

// Barra "Foco agora": você diz no que está focado; a IA local checa
// (automático, a cada ~3min) se a janela atual ajuda ou te distraiu, e te cutuca.
export function FocusBar() {
  const [focus, setFocus] = useState<string>("");
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState("");
  const [check, setCheck] = useState<FocusCheck | null>(null);
  const [loading, setLoading] = useState(false);
  const lastNudge = useRef(0);

  useEffect(() => {
    invoke<string | null>("get_focus")
      .then((f) => setFocus(f ?? ""))
      .catch(() => {});
  }, []);

  async function save() {
    const text = draft.trim();
    await invoke("set_focus", { text });
    setFocus(text);
    setEditing(false);
    setCheck(null);
    lastNudge.current = 0;
  }

  async function clear() {
    await invoke("set_focus", { text: "" });
    setFocus("");
    setCheck(null);
  }

  async function runCheck(auto = false) {
    if (!focus.trim()) return;
    if (!auto) setLoading(true);
    try {
      const c = await invoke<FocusCheck>("check_focus");
      setCheck(c);
      if (c.on_task === false) {
        // Nudge gentil: no máximo 1 a cada 15min enquanto distraído.
        // No "dia ruim", o app fica quieto — sem cutucar.
        const now = Date.now();
        let mode = "companheiro";
        try {
          mode = (await invoke<string | null>("get_setting", { key: "mode" })) ?? "companheiro";
        } catch {
          /* ignore */
        }
        if (mode !== "dia_ruim" && now - lastNudge.current > 15 * 60 * 1000) {
          lastNudge.current = now;
          try {
            let ok = await isPermissionGranted();
            if (!ok) ok = (await requestPermission()) === "granted";
            if (ok)
              sendNotification({
                title: "focusbar",
                body: `Travou em ${c.app ?? "outra coisa"} — quer voltar pro "${c.focus}" ou começar leve? 💛`,
              });
          } catch {
            /* ignore */
          }
        }
      } else if (c.on_task === true) {
        lastNudge.current = 0;
      }
    } catch {
      /* ignore */
    }
    if (!auto) setLoading(false);
  }

  async function correct(onTask: boolean) {
    if (!check?.app) return;
    try {
      await invoke("set_focus_judgment", { app: check.app, onTask });
      runCheck(false);
    } catch {
      /* ignore */
    }
  }

  // Auto-checagem: primeira em ~15s (pra dar tempo de trocar pra janela de
  // trabalho) e depois a cada 3 min, enquanto houver foco definido.
  useEffect(() => {
    if (!focus.trim()) return;
    const first = setTimeout(() => runCheck(true), 15000);
    const id = setInterval(() => runCheck(true), 180000);
    return () => {
      clearTimeout(first);
      clearInterval(id);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [focus]);

  if (!focus || editing) {
    return (
      <div className="focusbar">
        <input
          className="focusbar-input"
          placeholder="no que você quer focar agora? (ex.: terminar o relatório)"
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && save()}
          autoFocus={editing}
        />
        <button className="grant-btn" onClick={save}>
          definir foco
        </button>
      </div>
    );
  }

  const status =
    check?.on_task === true
      ? "ontask"
      : check?.on_task === false
        ? "offtask"
        : "neutral";

  return (
    <div className={`focusbar set ${status}`}>
      <div className="focusbar-line">
        <span className="focusbar-goal">🎯 {focus}</span>
        <div className="focusbar-actions">
          <button className="link-btn" onClick={() => runCheck(false)} disabled={loading}>
            {loading ? "checando…" : "checar agora"}
          </button>
          <button
            className="link-btn"
            onClick={() => {
              setDraft(focus);
              setEditing(true);
            }}
          >
            trocar
          </button>
          <button className="link-btn" onClick={clear}>
            limpar
          </button>
        </div>
      </div>
      {check && (
        <div className="focusbar-status">
          <span>
            {status === "ontask"
              ? "🟢 no foco"
              : status === "offtask"
                ? `🟠 ${check.reason}`
                : `⚪ ${check.reason}`}
          </span>
          {check.app && check.on_task !== null && (
            <span className="focusbar-correct">
              {check.on_task ? (
                <button className="link-btn" onClick={() => correct(false)}>
                  é distração
                </button>
              ) : (
                <button className="link-btn" onClick={() => correct(true)}>
                  é útil ✓
                </button>
              )}
              {check.source === "user" && <span className="focusbar-src">· você ajustou</span>}
              {check.source === "rule" && <span className="focusbar-src">· regra</span>}
              {check.source === "ia" && <span className="focusbar-src">· IA</span>}
            </span>
          )}
        </div>
      )}
    </div>
  );
}
