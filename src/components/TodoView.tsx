import { useState } from "react";
import { useTodos } from "../hooks/useTodos";
import type { Todo } from "../types";
import { fmtDuration, parseDuration } from "../format";

function Row({
  t,
  onToggle,
  onRemove,
  onFocus,
}: {
  t: Todo;
  onToggle: () => void;
  onRemove: () => void;
  onFocus?: () => void;
}) {
  return (
    <li className={t.done ? "todo-row done" : "todo-row"}>
      <button className="todo-check" onClick={onToggle} title="marcar">
        {t.done ? "✓" : ""}
      </button>
      <span className="todo-text">
        {t.text}
        {(t.custom_secs || t.est_pomos) && (
          <span className="todo-meta">
            {t.custom_secs ? ` ⏱ ${fmtDuration(t.custom_secs)}` : ""}
            {t.est_pomos ? ` 🍅×${t.est_pomos}` : ""}
          </span>
        )}
      </span>
      {!t.done && onFocus && (
        <button
          className="todo-focus"
          onClick={onFocus}
          title={
            t.custom_secs
              ? `focar ${fmtDuration(t.custom_secs)} nesta tarefa`
              : "focar nesta tarefa"
          }
        >
          ▶ focar
        </button>
      )}
      <button className="rm-btn danger" onClick={onRemove}>
        x
      </button>
    </li>
  );
}

export function TodoView({
  onFocusTask,
}: {
  onFocusTask?: (text: string, secs?: number) => void;
}) {
  const { open, done, add, toggle, remove } = useTodos(4000);
  const [text, setText] = useState("");
  // Rev4 #5: cada tarefa pode ter o SEU tempo ("10min", "1h") e estimativa 🍅.
  const [dur, setDur] = useState("");
  const [pomos, setPomos] = useState("");

  function submit() {
    const secs = parseDuration(dur) ?? undefined;
    const est = parseInt(pomos, 10) || undefined;
    add(text, secs, est);
    setText("");
    setDur("");
    setPomos("");
  }

  return (
    <section className="daily">
      <div className="daily-header">
        <span>TAREFAS</span>
        <span className="sessions-count">{open.length} abertas</span>
      </div>

      <div className="reminder-form">
        <div className="rm-row">
          <input
            className="rm-text"
            style={{ marginBottom: 0, flex: 1 }}
            placeholder="o que você precisa fazer?"
            value={text}
            onChange={(e) => setText(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && submit()}
          />
          <input
            className="rm-text todo-dur"
            placeholder="⏱ tempo (ex: 30min)"
            title="tempo próprio desta tarefa — o ▶ focar usa ele"
            value={dur}
            onChange={(e) => setDur(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && submit()}
          />
          <input
            className="rm-text todo-pomos"
            placeholder="🍅 qtd"
            title="estimativa: quantos pomodoros isso leva?"
            value={pomos}
            onChange={(e) => setPomos(e.target.value.replace(/\D/g, ""))}
            onKeyDown={(e) => e.key === "Enter" && submit()}
          />
          <button className="grant-btn" onClick={submit}>
            Adicionar
          </button>
        </div>
      </div>

      {open.length === 0 && done.length === 0 ? (
        <p className="empty">
          Sem tarefas. Escreva o que não pode esquecer — fica salvo até você concluir.
        </p>
      ) : (
        <ul className="todo-list">
          {open.map((t) => (
            <Row
              key={t.id}
              t={t}
              onToggle={() => toggle(t.id)}
              onRemove={() => remove(t.id)}
              onFocus={
                onFocusTask
                  ? () => onFocusTask(t.text, t.custom_secs ?? undefined)
                  : undefined
              }
            />
          ))}
          {done.length > 0 && <div className="todo-done-label">concluídas</div>}
          {done.map((t) => (
            <Row key={t.id} t={t} onToggle={() => toggle(t.id)} onRemove={() => remove(t.id)} />
          ))}
        </ul>
      )}
    </section>
  );
}
