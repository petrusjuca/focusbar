import { useState } from "react";
import { parseDuration } from "../format";

// Relógio clicável (estilo timer do Google): clica nos dígitos, digita um tempo
// novo ("10", "1h30", "25:00"), Enter aplica — o RESTANTE vira o que você digitou.
export function EditableClock({
  text,
  onRetime,
  className = "",
}: {
  text: string;
  onRetime: (seconds: number) => void;
  className?: string;
}) {
  const [editing, setEditing] = useState(false);
  const [val, setVal] = useState("");
  const secs = parseDuration(val);

  if (!editing) {
    return (
      <button
        className={`clock-edit ${className}`}
        title="clique pra digitar um tempo novo (ex: 10, 1h30, 25:00)"
        onClick={() => {
          setVal("");
          setEditing(true);
        }}
      >
        {text}
      </button>
    );
  }
  return (
    <input
      className={`clock-input ${className}`}
      autoFocus
      placeholder="1h30…"
      value={val}
      onChange={(e) => setVal(e.target.value)}
      onKeyDown={(e) => {
        if (e.key === "Enter" && secs) {
          onRetime(secs);
          setEditing(false);
        }
        if (e.key === "Escape") setEditing(false);
      }}
      onBlur={() => setEditing(false)}
    />
  );
}
