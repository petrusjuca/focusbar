import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Note } from "../types";
import { useTodos } from "../hooks/useTodos";

// O CASCATA (MANUAL): a intenção do dia no topo, e VOCÊ quebra em passos — você
// decide as tarefas. A IA não inventa nada aqui; o papel dela é, depois, ENTENDER
// pela tela se você está realmente fazendo o que delegou (ver o agente de foco).
export function IntentionCascade() {
  const [intention, setIntention] = useState("");
  const { add } = useTodos();
  const [step, setStep] = useState("");

  async function loadIntention() {
    try {
      const notes = await invoke<Note[]>("list_notes", { day: null });
      const ints = notes.filter((n) => n.kind === "intention");
      setIntention(ints.length ? ints[ints.length - 1].text : "");
    } catch {
      setIntention("");
    }
  }
  useEffect(() => {
    loadIntention();
    const id = setInterval(loadIntention, 8000); // reflete quando você define/troca
    return () => clearInterval(id);
  }, []);

  async function addStep() {
    if (!step.trim()) return;
    await add(step.trim());
    setStep("");
  }

  if (!intention) return null;

  return (
    <section className="intention-cascade">
      <div className="intent-label">🎯 Intenção de hoje</div>
      <div className="intent-text">{intention}</div>
      <div className="intent-sub">
        Quebre em passos — <b>você</b> decide as tarefas. Depois é só focar em cada
        uma (▶) que o agente entende, pela tela, se você está mesmo nela.
      </div>
      <div className="intent-steprow">
        <input
          className="intent-stepinput"
          placeholder="+ um passo pra chegar lá…"
          value={step}
          onChange={(e) => setStep(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && addStep()}
        />
        <button className="grant-btn" onClick={addStep} disabled={!step.trim()}>
          adicionar passo
        </button>
      </div>
    </section>
  );
}
