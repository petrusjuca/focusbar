import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";

// Modo Foco (Pomodoro) — desenhado pro MUNDO REAL, não pro ideal:
//  idle → focus → (acabou o tempo) → OVERTIME (conta pra tarefa até você decidir)
//       → break → (acabou a pausa) → BREAK_OVER (te espera, não pune)
// Nada de auto-iniciar pausa nem auto-retomar: cada transição PEDE confirmação.
// O timer roda no frontend e persiste em localStorage (sobrevive reload/reinício).

export type SessionPhase = "idle" | "focus" | "overtime" | "break" | "break_over";

const BREAK_SECS = 5 * 60;
const KEY = "focus-session";

function today(): string {
  return new Date().toISOString().slice(0, 10);
}

async function notify(title: string, body: string) {
  try {
    let ok = await isPermissionGranted();
    if (!ok) ok = (await requestPermission()) === "granted";
    if (ok) sendNotification({ title, body });
  } catch {
    /* ignore */
  }
}

// Toque sintetizado (sem arquivo). `rising` = fim do foco; descendo = fim da pausa.
function playChime(rising: boolean) {
  try {
    const Ctx = window.AudioContext || (window as unknown as any).webkitAudioContext;
    if (!Ctx) return;
    const ctx = new Ctx();
    ctx.resume?.();
    const notes = rising ? [660, 990] : [880, 587];
    notes.forEach((freq, i) => {
      const osc = ctx.createOscillator();
      const gain = ctx.createGain();
      osc.type = "sine";
      osc.frequency.value = freq;
      osc.connect(gain);
      gain.connect(ctx.destination);
      const t = ctx.currentTime + i * 0.2;
      gain.gain.setValueAtTime(0.0001, t);
      gain.gain.exponentialRampToValueAtTime(0.35, t + 0.02);
      gain.gain.exponentialRampToValueAtTime(0.0001, t + 0.4);
      osc.start(t);
      osc.stop(t + 0.42);
    });
    setTimeout(() => ctx.close?.(), 1200);
  } catch {
    /* ignore */
  }
}

export interface FocusSessionApi {
  phase: SessionPhase;
  remaining: number; // segundos restantes na contagem regressiva (focus/break)
  over: number; // segundos PASSADOS do fim (overtime/break_over)
  goal: string;
  blockPaused: boolean;
  pomodoros: number; // blocos concluídos hoje
  breaksSkipped: number; // pausas puladas hoje
  start: (seconds: number, goal: string) => void;
  stop: () => void; // encerra tudo → idle (registra o bloco se estava em foco)
  toggleBlock: () => void; // congela/retoma o bloco (só em focus)
  startBreak: () => void; // confirma a pausa (a partir do overtime)
  finishTask: () => void; // "terminei!" → registra + marca a tarefa concluída → pausa
  skipBreak: () => void; // pula a pausa → idle (conta como pulada)
  startNext: (seconds?: number, goal?: string) => void; // novo bloco (reusa a duração)
}

export function useFocusSession(): FocusSessionApi {
  const [phase, setPhaseS] = useState<SessionPhase>("idle");
  const [remaining, setRemaining] = useState(0);
  const [over, setOver] = useState(0);
  const [goal, setGoalS] = useState("");
  const [blockPaused, setBPS] = useState(false);
  const [pomodoros, setPomS] = useState(0);
  const [breaksSkipped, setSkipS] = useState(0);

  // Fonte da verdade em refs (evita stale-closure no setInterval).
  const phaseRef = useRef<SessionPhase>("idle");
  const endsAtRef = useRef(0); // fim da contagem regressiva (ms epoch)
  const overStartRef = useRef(0); // início da contagem crescente (ms epoch)
  const plannedRef = useRef(0); // duração planejada do bloco (s)
  const blockStartRef = useRef(0); // início do bloco de foco (ms epoch)
  const goalRef = useRef("");
  const bpRef = useRef(false);
  const frozenRef = useRef(0); // segundos congelados ao pausar
  const pomRef = useRef(0);
  const skipRef = useRef(0);
  const dayRef = useRef(today());

  function persist() {
    try {
      localStorage.setItem(
        KEY,
        JSON.stringify({
          phase: phaseRef.current,
          endsAt: endsAtRef.current,
          overStart: overStartRef.current,
          planned: plannedRef.current,
          blockStart: blockStartRef.current,
          goal: goalRef.current,
          blockPaused: bpRef.current,
          frozen: frozenRef.current,
          pomodoros: pomRef.current,
          breaksSkipped: skipRef.current,
          day: dayRef.current,
        })
      );
    } catch {
      /* ignore */
    }
  }
  const setPhase = (p: SessionPhase) => {
    phaseRef.current = p;
    setPhaseS(p);
  };
  const setPom = (n: number) => {
    pomRef.current = n;
    setPomS(n);
  };
  const setSkip = (n: number) => {
    skipRef.current = n;
    setSkipS(n);
  };
  const setGoal = (g: string) => {
    goalRef.current = g;
    setGoalS(g);
  };
  const setBP = (v: boolean) => {
    bpRef.current = v;
    setBPS(v);
  };

  // Segundos já consumidos do bloco de foco atual (respeita pausa).
  function focusConsumed(): number {
    if (bpRef.current) return Math.max(0, plannedRef.current - frozenRef.current);
    const rem = Math.max(0, Math.round((endsAtRef.current - Date.now()) / 1000));
    return Math.max(0, plannedRef.current - rem);
  }
  function overSecs(): number {
    return Math.max(0, Math.round((Date.now() - overStartRef.current) / 1000));
  }

  // Fecha o bloco de foco: registra o tempo REAL (planejado consumido + overtime),
  // opcionalmente marca a tarefa como concluída, e conta o pomodoro.
  function finishBlock(completed: boolean) {
    const inFocus = phaseRef.current === "focus";
    const inOver = phaseRef.current === "overtime";
    if (!inFocus && !inOver) return;
    const actual = inOver ? plannedRef.current + overSecs() : focusConsumed();
    const g = goalRef.current;
    if (actual > 5) {
      // Tempo real da tarefa (inclui overtime) — alimenta a dedicação por objetivo.
      invoke("log_focus_time", { goal: g, secs: actual }).catch(() => {});
      invoke("log_pomodoro", {
        goal: g,
        startTs: Math.round(blockStartRef.current / 1000),
        plannedSecs: plannedRef.current,
        actualSecs: actual,
        completed,
      }).catch(() => {});
    }
    if (completed && g.trim()) {
      // Sai da lista sozinha — sem precisar ir remover manualmente.
      invoke("complete_todo_by_text", { text: g }).catch(() => {});
    }
    setPom(pomRef.current + 1);
  }

  function enterBreak() {
    endsAtRef.current = Date.now() + BREAK_SECS * 1000;
    setBP(false);
    setOver(0);
    setPhase("break");
    persist();
  }

  useEffect(() => {
    // Restaura sessão anterior (se ainda fizer sentido).
    try {
      const raw = localStorage.getItem(KEY);
      if (raw) {
        const s = JSON.parse(raw);
        if (s.day === today()) {
          setPom(typeof s.pomodoros === "number" ? s.pomodoros : 0);
          setSkip(typeof s.breaksSkipped === "number" ? s.breaksSkipped : 0);
        } else {
          dayRef.current = today();
        }
        setGoal(s.goal || "");
        plannedRef.current = typeof s.planned === "number" ? s.planned : 0;
        blockStartRef.current = typeof s.blockStart === "number" ? s.blockStart : 0;
        if (s.phase === "focus") {
          if (s.blockPaused) {
            setBP(true);
            frozenRef.current = typeof s.frozen === "number" ? s.frozen : 0;
            setPhase("focus");
            setRemaining(frozenRef.current);
          } else if (s.endsAt > Date.now()) {
            endsAtRef.current = s.endsAt;
            setPhase("focus");
          } else {
            // Acabou enquanto fechado → entra em overtime a partir do fim.
            overStartRef.current = s.endsAt || Date.now();
            setPhase("overtime");
          }
        } else if (s.phase === "overtime" || s.phase === "break_over") {
          overStartRef.current = s.overStart || Date.now();
          setPhase(s.phase);
        } else if (s.phase === "break") {
          if (s.endsAt > Date.now()) {
            endsAtRef.current = s.endsAt;
            setPhase("break");
          } else {
            overStartRef.current = s.endsAt || Date.now();
            setPhase("break_over");
          }
        }
      }
    } catch {
      /* ignore */
    }

    const id = setInterval(() => {
      const p = phaseRef.current;
      if (p === "idle") {
        setRemaining(0);
        setOver(0);
        return;
      }
      if (p === "focus") {
        if (bpRef.current) {
          setRemaining(frozenRef.current);
          return;
        }
        const rem = Math.max(0, Math.round((endsAtRef.current - Date.now()) / 1000));
        setRemaining(rem);
        if (rem <= 0) {
          // Tempo acabou — NÃO inicia pausa. Vira overtime (conta pra tarefa).
          playChime(true);
          notify(
            "Tempo do bloco acabou ⏱️",
            goalRef.current
              ? `"${goalRef.current}" — terminou? Confirme a pausa quando quiser.`
              : "Terminou? Confirme a pausa quando quiser."
          );
          overStartRef.current = endsAtRef.current; // conta a partir do fim exato
          setRemaining(0);
          setPhase("overtime");
          persist();
        }
        return;
      }
      if (p === "overtime") {
        setOver(overSecs());
        return;
      }
      if (p === "break") {
        const rem = Math.max(0, Math.round((endsAtRef.current - Date.now()) / 1000));
        setRemaining(rem);
        if (rem <= 0) {
          // Pausa acabou — NÃO retoma sozinho. Te espera (sem punir o cafezinho).
          playChime(false);
          notify("Pausa acabou ☕", "Bora pro próximo quando voltar.");
          overStartRef.current = endsAtRef.current;
          setRemaining(0);
          setPhase("break_over");
          persist();
        }
        return;
      }
      if (p === "break_over") {
        setOver(overSecs());
        return;
      }
    }, 250);

    return () => clearInterval(id);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  function start(seconds: number, g: string) {
    const secs = Math.max(1, Math.round(seconds));
    endsAtRef.current = Date.now() + secs * 1000;
    blockStartRef.current = Date.now();
    plannedRef.current = secs;
    setBP(false);
    setOver(0);
    setGoal(g);
    setPhase("focus");
    persist();
    notify(
      "Foco iniciado 🟢",
      g ? `${Math.round(secs / 60)}min em "${g}". Vamos!` : `${Math.round(secs / 60)}min de foco.`
    );
  }

  function stop() {
    // Se estava focando, registra o que rolou antes de zerar.
    finishBlockIfFocus();
    endsAtRef.current = 0;
    overStartRef.current = 0;
    setBP(false);
    setOver(0);
    setPhase("idle");
    persist();
  }
  function finishBlockIfFocus() {
    if (phaseRef.current === "focus" || phaseRef.current === "overtime") {
      finishBlock(false);
    }
  }

  function toggleBlock() {
    if (phaseRef.current !== "focus") return; // só congela durante o foco
    if (bpRef.current) {
      endsAtRef.current = Date.now() + frozenRef.current * 1000;
      setBP(false);
    } else {
      frozenRef.current = Math.max(0, Math.round((endsAtRef.current - Date.now()) / 1000));
      setBP(true);
    }
    persist();
  }

  // Confirma a pausa (a partir do overtime): fecha o bloco e entra na pausa.
  function startBreak() {
    if (phaseRef.current !== "overtime" && phaseRef.current !== "focus") return;
    finishBlock(false);
    enterBreak();
  }

  // "Terminei a tarefa": fecha o bloco, MARCA a tarefa concluída, e oferece pausa.
  function finishTask() {
    if (phaseRef.current !== "focus" && phaseRef.current !== "overtime") return;
    finishBlock(true);
    enterBreak();
  }

  // Pula a pausa → idle (e conta como pulada, pra você ver a tendência).
  function skipBreak() {
    if (phaseRef.current === "break") {
      setSkip(skipRef.current + 1);
    }
    endsAtRef.current = 0;
    overStartRef.current = 0;
    setOver(0);
    setPhase("idle");
    persist();
  }

  function startNext(seconds?: number, g?: string) {
    const secs = seconds && seconds > 0 ? seconds : plannedRef.current || 25 * 60;
    start(secs, g ?? goalRef.current);
  }

  return {
    phase,
    remaining,
    over,
    goal,
    blockPaused,
    pomodoros,
    breaksSkipped,
    start,
    stop,
    toggleBlock,
    startBreak,
    finishTask,
    skipBreak,
    startNext,
  };
}
