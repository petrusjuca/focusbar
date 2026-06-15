import { useState } from "react";
import { useTodos } from "../hooks/useTodos";
import type { Todo } from "../types";

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
      <span className="todo-text">{t.text}</span>
      {!t.done && onFocus && (
        <button className="todo-focus" onClick={onFocus} title="focar nesta tarefa">
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
  onFocusTask?: (text: string) => void;
}) {
  const { open, done, add, toggle, remove } = useTodos();
  const [text, setText] = useState("");

  function submit() {
    add(text);
    setText("");
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
              onFocus={onFocusTask ? () => onFocusTask(t.text) : undefined}
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
