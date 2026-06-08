import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Note } from "../types";

type Kind = "intention" | "note";

export function DiaryView() {
  const [notes, setNotes] = useState<Note[]>([]);
  const [text, setText] = useState("");
  const [kind, setKind] = useState<Kind>("intention");

  async function refresh() {
    try {
      setNotes(await invoke<Note[]>("list_notes", { day: null }));
    } catch {
      /* ignore */
    }
  }

  useEffect(() => {
    refresh();
  }, []);

  async function add() {
    if (!text.trim()) return;
    await invoke("add_note", { text: text.trim(), kind, day: null });
    setText("");
    refresh();
  }

  async function del(id: number) {
    await invoke("delete_note", { id });
    refresh();
  }

  const intentions = notes.filter((n) => n.kind === "intention");
  const quick = notes.filter((n) => n.kind === "note");

  return (
    <section className="daily">
      <div className="daily-header">
        <span>DIÁRIO DE HOJE</span>
      </div>

      <div className="reminder-form">
        <input
          className="rm-text"
          placeholder={
            kind === "intention"
              ? "o que você quer fazer hoje? (intenção)"
              : "anota rápido — uma ideia, algo pra não esquecer…"
          }
          value={text}
          onChange={(e) => setText(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && add()}
        />
        <div className="rm-row">
          <select value={kind} onChange={(e) => setKind(e.target.value as Kind)}>
            <option value="intention">intenção</option>
            <option value="note">nota rápida</option>
          </select>
          <button className="grant-btn" onClick={add}>
            Adicionar
          </button>
        </div>
      </div>

      {notes.length === 0 ? (
        <p className="empty">
          Diga ao focusbar o que quer fazer hoje — ele compara com o que você
          realmente fez no resumo da IA.
        </p>
      ) : (
        <ul className="session-list">
          {intentions.map((n) => (
            <li key={n.id} className="session-row reminder-row">
              <div className="rm-info">
                <div className="session-app">🎯 {n.text}</div>
                <div className="session-title">intenção</div>
              </div>
              <button className="rm-btn danger" onClick={() => del(n.id)}>
                x
              </button>
            </li>
          ))}
          {quick.map((n) => (
            <li key={n.id} className="session-row reminder-row">
              <div className="rm-info">
                <div className="session-app">📝 {n.text}</div>
                <div className="session-title">nota</div>
              </div>
              <button className="rm-btn danger" onClick={() => del(n.id)}>
                x
              </button>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
