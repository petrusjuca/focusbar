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
  const offStreak = useRef(0);

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
    offStreak.current = 0;
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
        offStreak.current += 1;
        if (offStreak.current >= 2) {
          try {
            let ok = await isPermissionGranted();
            if (!ok) ok = (await requestPermission()) === "granted";
            if (ok)
              sendNotification({
                title: "focusbar — foco",
                body: `Distraído de "${c.focus}"? Você está em ${c.app ?? "outra coisa"}.`,
              });
          } catch {
            /* ignore */
          }
          offStreak.current = 0;
        }
      } else if (c.on_task === true) {
        offStreak.current = 0;
      }
    } catch {
      /* ignore */
    }
    if (!auto) setLoading(false);
  }

  // Auto-checagem a cada 3 min quando há foco definido.
  useEffect(() => {
    if (!focus.trim()) return;
    const id = setInterval(() => runCheck(true), 180000);
    return () => clearInterval(id);
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
          focar
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
          {status === "ontask" && "🟢 no foco"}
          {status === "offtask" && "🟠 "}
          {status === "neutral" && "⚪ "}
          {status !== "ontask" && <span>{check.reason}</span>}
        </div>
      )}
    </div>
  );
}
