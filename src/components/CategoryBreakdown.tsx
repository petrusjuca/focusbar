import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { AppTotal, CategoryTotal } from "../types";
import { fmtDuration } from "../format";
import { CATEGORIES, catColor as colorFor } from "../categories";

export function CategoryBreakdown({
  data,
  apps,
}: {
  data: CategoryTotal[];
  apps: AppTotal[];
}) {
  const [editing, setEditing] = useState(false);
  const total = data.reduce((s, c) => s + c.total_secs, 0);
  if (total === 0) return null;

  async function setCat(app: string, category: string) {
    await invoke("set_app_category", { app, category });
  }

  return (
    <section className="category">
      <div className="daily-header">
        <span>POR CATEGORIA</span>
        <button className="link-btn" onClick={() => setEditing((v) => !v)}>
          {editing ? "ocultar" : "ajustar"}
        </button>
      </div>

      <div className="cat-bar">
        {data.map((c, i) => (
          <div
            key={i}
            className="cat-seg"
            title={`${c.category}: ${fmtDuration(c.total_secs)}`}
            style={{
              width: `${(c.total_secs / total) * 100}%`,
              background: colorFor(c.category),
            }}
          />
        ))}
      </div>

      <div className="cat-legend">
        {data.map((c, i) => (
          <div key={i} className="cat-row">
            <span className="dot" style={{ background: colorFor(c.category) }} />
            <span className="cat-name">{c.category}</span>
            <span className="cat-pct">
              {Math.round((c.total_secs / total) * 100)}%
            </span>
            <span className="cat-dur">{fmtDuration(c.total_secs)}</span>
          </div>
        ))}
      </div>

      {editing && (
        <div className="rules-box">
          <p className="bg-note" style={{ margin: "0 0 0.6rem" }}>
            Categoria errada? Ajuste aqui — ele lembra pra sempre.
          </p>
          {apps.map((a) => (
            <div key={a.app_name} className="cat-edit-row">
              <span className="cat-edit-name">{a.app_name}</span>
              <span className="cat-edit-dur">{fmtDuration(a.total_secs)}</span>
              <select
                defaultValue={CATEGORIES.includes(a.category) ? a.category : "Outro"}
                onChange={(e) => setCat(a.app_name, e.target.value)}
              >
                <option value="">automático</option>
                {CATEGORIES.map((c) => (
                  <option key={c} value={c}>
                    {c}
                  </option>
                ))}
              </select>
            </div>
          ))}
        </div>
      )}
    </section>
  );
}
