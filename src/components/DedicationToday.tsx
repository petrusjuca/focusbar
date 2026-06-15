import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { GoalTime } from "../types";
import { fmtDuration } from "../format";

// "Dedicado hoje": pra onde foi teu tempo de FOCO (soma dos blocos por tarefa).
// Atualiza no evento focus-changed (quando um bloco fecha, os dados mudam).
export function DedicationToday({ refreshKey }: { refreshKey: number }) {
  const [rows, setRows] = useState<GoalTime[]>([]);

  useEffect(() => {
    let alive = true;
    invoke<GoalTime[]>("get_focus_time", { day: null })
      .then((r) => alive && setRows(r))
      .catch(() => {});
    return () => {
      alive = false;
    };
  }, [refreshKey]);

  if (rows.length === 0) return null;

  const total = rows.reduce((a, r) => a + r.secs, 0);

  return (
    <div className="dedication">
      <div className="daily-header">
        <span>DEDICADO HOJE</span>
        <span className="daily-day">{fmtDuration(total)} em foco</span>
      </div>
      <ul className="dedication-list">
        {rows.map((r, i) => {
          const pct = total > 0 ? Math.round((r.secs / total) * 100) : 0;
          return (
            <li key={i} className="dedication-row">
              <div className="dedication-bar-wrap">
                <div className="dedication-info">
                  <span className="dedication-goal">{r.goal}</span>
                  <span className="dedication-time">{fmtDuration(r.secs)}</span>
                </div>
                <div className="dedication-bar">
                  <div
                    className="dedication-bar-fill"
                    style={{ width: `${Math.max(4, pct)}%` }}
                  />
                </div>
              </div>
            </li>
          );
        })}
      </ul>
    </div>
  );
}
