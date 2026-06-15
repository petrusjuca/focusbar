import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";

// Modo Foco (Pomodoro): blocos de foco protegidos + pausa, com cronômetro.
// O timer roda no frontend (a janela fica viva mesmo escondida na bandeja) e
// persiste em localStorage pra sobreviver a reload/reinício.

export type SessionPhase = "idle" | "focus" | "break";

const BREAK_MIN = 5;
const KEY = "focus-session";

async function notify(title: string, body: string) {
  try {
    let ok = await isPermissionGranted();
    if (!ok) ok = (await requestPermission()) === "granted";
    if (ok) sendNotification({ title, body });
  } catch {
    /* ignore */
  }
}

// Toque sonoro (sintetizado, sem arquivo) — um "ding-dong" de 2 notas. `rising`
// (sobe) pro fim do bloco de foco; descendo pro fim da pausa.
function playChime(rising: boolean) {
  try {
    const Ctx =
      window.AudioContext || (window as unknown as any).webkitAudioContext;
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
  remaining: number; // segundos restantes
  pomodoros: number; // blocos concluídos hoje
  goal: string;
  blockPaused: boolean; // o BLOCO está congelado?
  start: (minutes: number, goal: string) => void;
  stop: () => void;
  toggleBlock: () => void; // pausa/retoma o bloco em andamento
}

export function useFocusSession(): FocusSessionApi {
  const [phase, setPhaseS] = useState<SessionPhase>("idle");
  const [remaining, setRemaining] = useState(0);
  const [pomodoros, setPomS] = useState(0);
  const [goal, setGoalS] = useState("");
  const [blockPaused, setBPS] = useState(false);

  // Fonte da verdade em refs (evita stale-closure no interval).
  const phaseRef = useRef<SessionPhase>("idle");
  const endsAtRef = useRef(0);
  const pomRef = useRef(0);
  const goalRef = useRef("");
  const minRef = useRef(0); // duração (min) do bloco em andamento
  const bpRef = useRef(false); // bloco congelado?
  const frozenRef = useRef(0); // segundos restantes congelados ao pausar

  function persist() {
    try {
      localStorage.setItem(
        KEY,
        JSON.stringify({
          phase: phaseRef.current,
          endsAt: endsAtRef.current,
          pomodoros: pomRef.current,
          goal: goalRef.current,
          minutes: minRef.current,
          blockPaused: bpRef.current,
          frozen: frozenRef.current,
        })
      );
    } catch {
      /* ignore */
    }
  }
  function setPhase(p: SessionPhase) {
    phaseRef.current = p;
    setPhaseS(p);
  }
  function setPom(n: number) {
    pomRef.current = n;
    setPomS(n);
  }
  function setGoal(g: string) {
    goalRef.current = g;
    setGoalS(g);
  }
  function setBP(v: boolean) {
    bpRef.current = v;
    setBPS(v);
  }

  useEffect(() => {
    // Restaura sessão anterior (se ainda válida).
    try {
      const raw = localStorage.getItem(KEY);
      if (raw) {
        const s = JSON.parse(raw);
        setPom(typeof s.pomodoros === "number" ? s.pomodoros : 0);
        setGoal(s.goal || "");
        minRef.current = typeof s.minutes === "number" ? s.minutes : 0;
        if (s.phase && s.phase !== "idle") {
          if (s.blockPaused) {
            setBP(true);
            frozenRef.current = typeof s.frozen === "number" ? s.frozen : 0;
            endsAtRef.current = s.endsAt || 0;
            setPhase(s.phase);
            setRemaining(frozenRef.current);
          } else if (s.endsAt > Date.now()) {
            endsAtRef.current = s.endsAt;
            setPhase(s.phase);
          }
        }
      }
    } catch {
      /* ignore */
    }

    const id = setInterval(() => {
      if (phaseRef.current === "idle") {
        setRemaining(0);
        return;
      }
      // Bloco congelado: trava o relógio no valor pausado.
      if (bpRef.current) {
        setRemaining(frozenRef.current);
        return;
      }
      const rem = Math.max(0, Math.round((endsAtRef.current - Date.now()) / 1000));
      setRemaining(rem);
      if (rem > 0) return;

      // Fim da fase atual.
      if (phaseRef.current === "focus") {
        setPom(pomRef.current + 1);
        // Registra o tempo dedicado a esse objetivo (pra "direcionar o dia").
        invoke("log_focus_time", {
          goal: goalRef.current,
          secs: minRef.current * 60,
        }).catch(() => {});
        playChime(true);
        notify(
          "Bloco concluído! 🎉",
          goalRef.current
            ? `"${goalRef.current}" — hora da pausa (${BREAK_MIN}min).`
            : `Mandou bem. Pausa de ${BREAK_MIN}min.`
        );
        endsAtRef.current = Date.now() + BREAK_MIN * 60_000;
        setPhase("break");
      } else {
        playChime(false);
        notify("Pausa acabou ⏳", "Bora outro bloco de foco?");
        endsAtRef.current = 0;
        setPhase("idle");
      }
      persist();
    }, 250);

    return () => clearInterval(id);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  function start(minutes: number, g: string) {
    endsAtRef.current = Date.now() + minutes * 60_000;
    minRef.current = minutes;
    setBP(false);
    setGoal(g);
    setPhase("focus");
    persist();
    notify(
      "Foco iniciado 🟢",
      g ? `${minutes}min em "${g}". Vamos!` : `${minutes}min de foco. Vamos!`
    );
  }

  function stop() {
    endsAtRef.current = 0;
    setBP(false);
    setPhase("idle");
    persist();
  }

  // Pausa/retoma o bloco em andamento (congela o relógio).
  function toggleBlock() {
    if (phaseRef.current === "idle") return;
    if (bpRef.current) {
      // retomar: recomeça a contar do ponto congelado.
      endsAtRef.current = Date.now() + frozenRef.current * 1000;
      setBP(false);
    } else {
      // pausar: guarda o que falta.
      frozenRef.current = Math.max(
        0,
        Math.round((endsAtRef.current - Date.now()) / 1000)
      );
      setBP(true);
    }
    persist();
  }

  return { phase, remaining, pomodoros, goal, blockPaused, start, stop, toggleBlock };
}
