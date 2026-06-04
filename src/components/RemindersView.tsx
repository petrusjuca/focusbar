import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  isPermissionGranted,
  requestPermission,
} from "@tauri-apps/plugin-notification";
import type { Reminder } from "../types";

type Mode = "recurring" | "once";

export function RemindersView() {
  const [reminders, setReminders] = useState<Reminder[]>([]);
  const [text, setText] = useState("");
  const [mode, setMode] = useState<Mode>("recurring");
  const [everyMin, setEveryMin] = useState(60);
  const [atTime, setAtTime] = useState(""); // 'HH:MM'
  const [notifOk, setNotifOk] = useState(true);
  const [err, setErr] = useState<string | null>(null);

  async function refresh() {
    try {
      setReminders(await invoke<Reminder[]>("list_reminders"));
    } catch (e) {
      setErr(String(e));
    }
  }

  useEffect(() => {
    refresh();
    isPermissionGranted().then(setNotifOk);
  }, []);

  async function askNotif() {
    const p = await requestPermission();
    setNotifOk(p === "granted");
  }

  async function add() {
    if (!text.trim()) return;
    try {
      let fireAt: number | null = null;
      let intervalSecs: number | null = null;
      if (mode === "recurring") {
        intervalSecs = Math.max(1, Math.round(everyMin)) * 60;
      } else {
        if (!atTime) {
          setErr("escolha um horário");
          return;
        }
        const [h, m] = atTime.split(":").map(Number);
        const d = new Date();
        d.setHours(h, m, 0, 0);
        if (d.getTime() < Date.now()) d.setDate(d.getDate() + 1); // amanhã se já passou
        fireAt = Math.floor(d.getTime() / 1000);
      }
      await invoke("create_reminder", {
        text: text.trim(),
        kind: mode,
        fireAt,
        intervalSecs,
      });
      setText("");
      setErr(null);
      refresh();
    } catch (e) {
      setErr(String(e));
    }
  }

  async function toggle(r: Reminder) {
    await invoke("set_reminder_enabled", { id: r.id, enabled: !r.enabled });
    refresh();
  }

  async function remove(r: Reminder) {
    await invoke("delete_reminder", { id: r.id });
    refresh();
  }

  function describe(r: Reminder): string {
    if (r.kind === "recurring" && r.interval_secs) {
      const min = Math.round(r.interval_secs / 60);
      return min >= 60 && min % 60 === 0
        ? `a cada ${min / 60}h`
        : `a cada ${min}min`;
    }
    if (r.kind === "once" && r.fire_at) {
      return `às ${new Date(r.fire_at * 1000).toLocaleString([], {
        hour: "2-digit",
        minute: "2-digit",
        day: "2-digit",
        month: "2-digit",
      })}`;
    }
    return r.kind;
  }

  return (
    <section className="daily">
      <div className="daily-header">
        <span>LEMBRETES</span>
      </div>

      {!notifOk && (
        <div className="permission-banner">
          <div className="perm-title">Ative as notificações</div>
          <p className="perm-text">
            Pra te avisar, o focusbar precisa de permissão de notificação.
          </p>
          <button className="grant-btn" onClick={askNotif}>
            Permitir notificações
          </button>
        </div>
      )}

      <div className="reminder-form">
        <input
          className="rm-text"
          placeholder="ex.: beber água, alongar, conferir tasks…"
          value={text}
          onChange={(e) => setText(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && add()}
        />
        <div className="rm-row">
          <select value={mode} onChange={(e) => setMode(e.target.value as Mode)}>
            <option value="recurring">repetir</option>
            <option value="once">uma vez</option>
          </select>
          {mode === "recurring" ? (
            <label className="rm-inline">
              a cada
              <input
                type="number"
                min={1}
                value={everyMin}
                onChange={(e) => setEveryMin(Number(e.target.value))}
              />
              min
            </label>
          ) : (
            <input
              type="time"
              value={atTime}
              onChange={(e) => setAtTime(e.target.value)}
            />
          )}
          <button className="grant-btn" onClick={add}>
            Adicionar
          </button>
        </div>
        {err && <p className="error">{err}</p>}
      </div>

      {reminders.length === 0 ? (
        <p className="empty">Nenhum lembrete ainda.</p>
      ) : (
        <ul className="session-list">
          {reminders.map((r) => (
            <li key={r.id} className="session-row reminder-row">
              <div className="rm-info">
                <div
                  className="session-app"
                  style={{ opacity: r.enabled ? 1 : 0.45 }}
                >
                  {r.text}
                </div>
                <div className="session-title">{describe(r)}</div>
              </div>
              <div className="rm-actions">
                <button className="rm-btn" onClick={() => toggle(r)}>
                  {r.enabled ? "pausar" : "ativar"}
                </button>
                <button className="rm-btn danger" onClick={() => remove(r)}>
                  excluir
                </button>
              </div>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
