import { useState, useEffect, useLayoutEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow, currentMonitor } from "@tauri-apps/api/window";
import { LogicalSize, LogicalPosition } from "@tauri-apps/api/dpi";
import type { FocusCheck } from "../types";
import type { FocusSessionApi } from "../hooks/useFocusSession";
import { fmtClock } from "../format";
import { useTodos } from "../hooks/useTodos";

const CARD_W = 268;

// Agente flutuante: um card de vidro, ancorado no canto, que guia.
//  • Parado  → a lista: escolha por onde começar (▶ foca, ○ conclui).
//  • Rodando → o timer + o foco, com pausar/parar.
// Calmo e discreto — uma direção, a ação certa, nada de ruído.
export function MiniView({
  paused,
  session,
  onExpand,
  onTogglePause,
}: {
  paused: boolean;
  session: FocusSessionApi;
  onExpand: () => void;
  onTogglePause: () => void;
}) {
  const [focus, setFocus] = useState("");
  const [check, setCheck] = useState<FocusCheck | null>(null);
  const [intention, setIntention] = useState("");
  const { open, toggle } = useTodos(5000);
  const rootRef = useRef<HTMLDivElement>(null);

  // Norte do dia — visível no card (o mini é onde você vive).
  useEffect(() => {
    let on = true;
    async function load() {
      try {
        const notes = await invoke<{ kind: string; text: string }[]>("list_notes", {
          day: null,
        });
        const ints = notes.filter((n) => n.kind === "intention");
        if (on) setIntention(ints.length ? ints[ints.length - 1].text : "");
      } catch {
        /* ignore */
      }
    }
    load();
    const id = setInterval(load, 20000);
    return () => {
      on = false;
      clearInterval(id);
    };
  }, []);

  useEffect(() => {
    let alive = true;
    async function run() {
      try {
        const f = await invoke<string | null>("get_focus");
        if (!alive) return;
        setFocus(f ?? "");
        if (f && f.trim()) {
          const c = await invoke<FocusCheck>("check_focus");
          if (alive) setCheck(c);
        } else {
          setCheck(null);
        }
      } catch {
        /* ignore */
      }
    }
    run();
    const first = setTimeout(run, 8000);
    const id = setInterval(run, 30000);
    return () => {
      alive = false;
      clearTimeout(first);
      clearInterval(id);
    };
  }, []);

  // Ancora no canto superior direito da tela (uma vez).
  useEffect(() => {
    (async () => {
      try {
        const w = getCurrentWindow();
        const mon = await currentMonitor();
        if (!mon) return;
        const sf = mon.scaleFactor || 1;
        const mw = mon.size.width / sf;
        const mx = mon.position.x / sf;
        const my = mon.position.y / sf;
        await w.setPosition(
          new LogicalPosition(Math.round(mx + mw - CARD_W - 18), Math.round(my + 46))
        );
      } catch {
        /* ignore */
      }
    })();
  }, []);

  // Inicia um bloco de 25min num clique (definindo o foco se veio de uma tarefa).
  async function startBlock(g: string) {
    if (g && g !== focus) {
      try {
        await invoke("set_focus", { text: g });
        setFocus(g);
      } catch {
        /* ignore */
      }
    }
    session.start(25 * 60, g || focus);
  }

  const onTask = check?.on_task;
  const phase = session.phase;
  const running = phase !== "idle";
  const isFocus = phase === "focus";
  const isOver = phase === "overtime";
  const isBreak = phase === "break";
  const isBreakOver = phase === "break_over";

  // Tom (cor do ponto de status) + a direção (uma frase calma, sem culpa).
  let tone = "idle";
  let dir = "Tô aqui com você";
  if (paused) {
    tone = "paused";
    dir = "Monitoramento pausado";
  } else if (isOver) {
    tone = "good";
    dir = "Tempo cumprido — pausar?";
  } else if (isBreakOver) {
    tone = "break";
    dir = "Bora pro próximo?";
  } else if (isBreak) {
    tone = "break";
    dir = "Pausa — respira um pouco";
  } else if (isFocus && session.blockPaused) {
    tone = "paused";
    dir = "Bloco pausado";
  } else if (isFocus) {
    tone = onTask === false ? "warn" : "good";
    dir = onTask === false ? "Vamos voltar ao foco?" : "Em foco";
  } else if (session.pomodoros > 0) {
    tone = "good";
    dir = "Boa! Pronto pra mais um?";
  } else {
    tone = "idle";
    dir = open.length ? "Escolha por onde começar" : "Vamos começar pequeno";
  }

  // Ações do timer conforme a fase (no máx. 2, pra não poluir o card).
  const clock = isOver || isBreakOver ? `+${fmtClock(session.over)}` : fmtClock(session.remaining);
  const timerActions: { label: string; on: () => void }[] = isFocus
    ? [
        { label: session.blockPaused ? "retomar" : "pausar", on: session.toggleBlock },
        { label: "+5min", on: () => session.extend(5) },
        { label: "terminei", on: session.finishTask },
      ]
    : isOver
      ? [
          { label: "iniciar pausa", on: session.startBreak },
          { label: "terminei", on: session.finishTask },
        ]
      : isBreak
        ? [
            { label: "próximo", on: () => session.startNext() },
            { label: "pular", on: session.skipBreak },
          ]
        : [
            { label: "próximo", on: () => session.startNext() },
            { label: "encerrar", on: session.skipBreak },
          ];

  // Ajusta a janela à altura do conteúdo — nada de vão vazio.
  useLayoutEffect(() => {
    const el = rootRef.current;
    if (!el) return;
    const h = Math.ceil(el.offsetHeight);
    getCurrentWindow()
      .setSize(new LogicalSize(CARD_W, h))
      .catch(() => {});
  }, [running, isBreak, paused, dir, open.length, session.blockPaused]);

  return (
    <div ref={rootRef} className="agent-wrap">
      <div className={`agent ${tone}`} data-tauri-drag-region>
        <div className="agent-top" data-tauri-drag-region>
          <span className="agent-brand" data-tauri-drag-region>
            <i className="agent-status" />
            focusbar
          </span>
          <div className="agent-ctrl">
            <button
              className="agent-icon"
              onClick={onTogglePause}
              title={paused ? "retomar monitoramento" : "pausar monitoramento"}
            >
              {paused ? "▶" : "⏸"}
            </button>
            <button className="agent-icon" onClick={onExpand} title="abrir janela">
              ⤢
            </button>
          </div>
        </div>

        <div className="agent-dir" data-tauri-drag-region>
          {dir}
        </div>

        {!running && intention && (
          <div className="agent-intent" title={intention} data-tauri-drag-region>
            🎯 {intention}
          </div>
        )}

        {running ? (
          <div className="agent-timer">
            <div className="agent-clock">{clock}</div>
            {(isFocus || isOver) && focus && (
              <div className="agent-focus" title={focus}>
                {focus}
              </div>
            )}
            <div className="agent-timer-ctrl">
              {timerActions.map((a, i) => (
                <span key={a.label}>
                  {i > 0 && <span className="agent-sep">·</span>}
                  <button className="agent-link" onClick={a.on}>
                    {a.label}
                  </button>
                </span>
              ))}
            </div>
          </div>
        ) : open.length === 0 ? (
          <button className="agent-primary" onClick={() => startBlock(focus)}>
            {focus ? `Focar em "${focus}"` : "Começar um bloco de 25 min"}
          </button>
        ) : (
          <div className="agent-list">
            {open.slice(0, 4).map((t) => (
              <div className="agent-item" key={t.id}>
                <button
                  className="agent-item-check"
                  onClick={() => toggle(t.id)}
                  title="concluir"
                />
                <span className="agent-item-text" title={t.text}>
                  {t.text}
                </span>
                <button
                  className="agent-item-go"
                  onClick={() => startBlock(t.text)}
                  title="focar nesta tarefa (25 min)"
                >
                  ▶
                </button>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
