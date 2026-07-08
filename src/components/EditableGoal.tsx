import { useState } from "react";

// Nome do pomodoro clicável (Rev4 #10): renomeia a tarefa DO BLOCO EM ANDAMENTO
// sem parar o tempo — clica no 🎯, digita, Enter.
export function EditableGoal({
  goal,
  onRename,
  className = "",
}: {
  goal: string;
  onRename: (g: string) => void;
  className?: string;
}) {
  const [editing, setEditing] = useState(false);
  const [val, setVal] = useState("");

  if (!editing) {
    return (
      <button
        className={`goal-edit ${className}`}
        title="clique pra renomear a tarefa deste bloco"
        onClick={() => {
          setVal(goal);
          setEditing(true);
        }}
      >
        🎯 {goal}
      </button>
    );
  }
  return (
    <input
      className={`goal-input ${className}`}
      autoFocus
      value={val}
      onChange={(e) => setVal(e.target.value)}
      onKeyDown={(e) => {
        if (e.key === "Enter" && val.trim()) {
          onRename(val.trim());
          setEditing(false);
        }
        if (e.key === "Escape") setEditing(false);
      }}
      onBlur={() => setEditing(false)}
    />
  );
}
