import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { TaskRule, TaskTotal } from "../types";
import { fmtDuration } from "../format";

const COLORS = ["#5856d6", "#34c759", "#ff9500", "#ff2d55", "#5ac8fa", "#ffcc00", "#af52de"];

export function TaskBreakdown({
  data,
  rules,
  onChange,
}: {
  data: TaskTotal[];
  rules: TaskRule[];
  onChange: () => void;
}) {
  const [showRules, setShowRules] = useState(false);
  const [kw, setKw] = useState("");
  const [name, setName] = useState("");

  const total = data.reduce((s, t) => s + t.total_secs, 0);

  async function addRule() {
    if (!kw.trim() || !name.trim()) return;
    await invoke("create_task_rule", { keyword: kw.trim(), taskName: name.trim() });
    setKw("");
    setName("");
    onChange();
  }

  async function delRule(id: number) {
    await invoke("delete_task_rule", { id });
    onChange();
  }

  return (
    <section className="category">
      <div className="daily-header">
        <span>POR TASK</span>
        <button className="link-btn" onClick={() => setShowRules((v) => !v)}>
          {showRules ? "ocultar regras" : "regras"}
        </button>
      </div>

      {total === 0 ? (
        <p className="empty">Sem tempo por task hoje. Crie regras pra nomear suas tasks.</p>
      ) : (
        <>
          <div className="cat-bar">
            {data.map((t, i) => (
              <div
                key={i}
                className="cat-seg"
                title={`${t.task}: ${fmtDuration(t.total_secs)}`}
                style={{
                  width: `${(t.total_secs / total) * 100}%`,
                  background: t.task === "(sem task)" ? "#c7c7cc" : COLORS[i % COLORS.length],
                }}
              />
            ))}
          </div>
          <div className="cat-legend">
            {data.map((t, i) => (
              <div key={i} className="cat-row">
                <span
                  className="dot"
                  style={{
                    background: t.task === "(sem task)" ? "#c7c7cc" : COLORS[i % COLORS.length],
                  }}
                />
                <span className="cat-name">{t.task}</span>
                <span className="cat-pct">{Math.round((t.total_secs / total) * 100)}%</span>
                <span className="cat-dur">{fmtDuration(t.total_secs)}</span>
              </div>
            ))}
          </div>
        </>
      )}

      {showRules && (
        <div className="rules-box">
          <div className="rm-row">
            <input
              className="rule-kw"
              placeholder="palavra no título (ex.: del paseo)"
              value={kw}
              onChange={(e) => setKw(e.target.value)}
            />
            <input
              className="rule-name"
              placeholder="nome da task"
              value={name}
              onChange={(e) => setName(e.target.value)}
            />
            <button className="grant-btn" onClick={addRule}>
              add
            </button>
          </div>
          {rules.length > 0 && (
            <ul className="rule-list">
              {rules.map((r) => (
                <li key={r.id} className="rule-item">
                  <span>
                    <b>{r.task_name}</b> ← contém "{r.keyword}"
                  </span>
                  <button className="rm-btn danger" onClick={() => delRule(r.id)}>
                    x
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>
      )}
    </section>
  );
}
