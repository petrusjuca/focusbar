import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Note } from "../types";

// Rituais do dia — o app PUXA você, não espera você lembrar:
//  • De manhã (dia novo sem intenção) → pergunta "qual a intenção de hoje?".
//  • No fim do dia (horário configurável) → "acabou por hoje?".
function today(): string {
  return new Date().toISOString().slice(0, 10);
}

export function DayRituals() {
  const day = today();
  const [intentionToday, setIntentionToday] = useState<boolean | null>(null);
  const [text, setText] = useState("");
  const [skipped, setSkipped] = useState(false);
  const [eodHour, setEodHour] = useState(18);
  const [eodAsk, setEodAsk] = useState(false);
  const [closed, setClosed] = useState(false);

  useEffect(() => {
    (async () => {
      try {
        const notes = await invoke<Note[]>("list_notes", { day: null });
        setIntentionToday(notes.some((n) => n.kind === "intention"));
      } catch {
        setIntentionToday(false);
      }
      setSkipped(localStorage.getItem("intent-skip-" + day) === "1");
      try {
        const h = await invoke<string | null>("get_setting", { key: "eod_hour" });
        const n = h ? parseInt(h, 10) : 18;
        if (!isNaN(n)) setEodHour(n);
      } catch {
        /* ignore */
      }
    })();
  }, [day]);

  // Checa o horário de fim de dia a cada minuto.
  useEffect(() => {
    function check() {
      const done = localStorage.getItem("eod-done-" + day) === "1";
      const snooze = parseInt(localStorage.getItem("eod-snooze-" + day) || "0", 10);
      const hour = new Date().getHours();
      setEodAsk(!done && hour >= eodHour && Date.now() > snooze);
    }
    check();
    const id = setInterval(check, 60000);
    return () => clearInterval(id);
  }, [eodHour, day]);

  async function saveIntention() {
    if (!text.trim()) return;
    try {
      await invoke("add_note", { text: text.trim(), kind: "intention", day: null });
      // Confirma pela RELEITURA (fonte da verdade) — não marca salvo "no otimismo".
      const notes = await invoke<Note[]>("list_notes", { day: null });
      setIntentionToday(notes.some((n) => n.kind === "intention"));
      setText("");
    } catch {
      /* falhou → mantém o card pra você tentar de novo */
    }
  }
  function skipIntention() {
    localStorage.setItem("intent-skip-" + day, "1");
    setSkipped(true);
  }
  function eodYes() {
    localStorage.setItem("eod-done-" + day, "1");
    setEodAsk(false);
    setClosed(true);
  }
  function eodSnooze() {
    localStorage.setItem("eod-snooze-" + day, String(Date.now() + 60 * 60 * 1000));
    setEodAsk(false);
  }

  const showMorning = intentionToday === false && !skipped;

  return (
    <>
      {showMorning && (
        <div className="ritual-card morning">
          <div className="ritual-title">☀️ Bom dia! Qual a intenção de hoje?</div>
          <div className="ritual-sub">
            o NORTE do dia, em uma frase (≠ tarefas, que são os passos). No fim do
            dia ele compara: você foi pra onde queria?
          </div>
          <div className="ritual-row">
            <input
              className="ritual-input"
              autoFocus
              placeholder="ex: avançar no projeto X, organizar a casa…"
              value={text}
              onChange={(e) => setText(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && saveIntention()}
            />
            <button className="grant-btn" onClick={saveIntention}>
              definir
            </button>
            <button className="link-btn" onClick={skipIntention}>
              hoje não
            </button>
          </div>
        </div>
      )}

      {eodAsk && (
        <div className="ritual-card eod">
          <div className="ritual-title">🌙 Acabou por hoje?</div>
          <div className="ritual-row">
            <button className="grant-btn" onClick={eodYes}>
              Sim, ver meu dia
            </button>
            <button className="link-btn" onClick={eodSnooze}>
              ainda não (+1h)
            </button>
          </div>
        </div>
      )}

      {closed && (
        <div className="ritual-card done">
          👏 Bom trabalho hoje. Seu dia tá aí em cima — descansa que amanhã a gente
          continua.
        </div>
      )}
    </>
  );
}
