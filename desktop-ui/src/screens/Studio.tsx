import { useState } from "react";
import { Persona, newId } from "../lib/store";

const EMOJI = ["🤖", "🧠", "🛰️", "🧑‍💻", "📊", "✍️", "🔬", "🎯", "🗂️", "🧭"];

/**
 * Mini-GPT Studio — create small custom assistants (a name + a system prompt +
 * an optional model). The active one shapes every reply in Chat.
 */
export default function Studio({
  personas,
  activeId,
  onSave,
  onDelete,
  onActivate,
}: {
  personas: Persona[];
  activeId: string | null;
  onSave: (p: Persona) => void;
  onDelete: (id: string) => void;
  onActivate: (id: string | null) => void;
}) {
  const [editing, setEditing] = useState<Persona | null>(null);

  const blank = (): Persona => ({ id: newId(), name: "", emoji: "🤖", system: "", model: "" });

  return (
    <div className="screen studio">
      <header className="screen-head">
        <div>
          <h2 className="screen-title">Mini-GPT Studio</h2>
          <p className="screen-sub">Build focused assistants. The active one steers every chat reply.</p>
        </div>
        <button className="btn-primary" onClick={() => setEditing(blank())}>
          + New Mini-GPT
        </button>
      </header>

      {editing ? (
        <div className="panel editor">
          <div className="editor-row">
            <div className="emoji-picker">
              {EMOJI.map((e) => (
                <button
                  key={e}
                  className={`emoji ${editing.emoji === e ? "on" : ""}`}
                  onClick={() => setEditing({ ...editing, emoji: e })}
                >
                  {e}
                </button>
              ))}
            </div>
          </div>
          <label className="field">
            <span>Name</span>
            <input
              value={editing.name}
              placeholder="e.g. Code Reviewer"
              onChange={(e) => setEditing({ ...editing, name: e.target.value })}
            />
          </label>
          <label className="field">
            <span>Model (optional — blank uses your default)</span>
            <input
              value={editing.model || ""}
              placeholder="qwen3.5:latest"
              onChange={(e) => setEditing({ ...editing, model: e.target.value })}
            />
          </label>
          <label className="field">
            <span>System prompt — who it is and how it should behave</span>
            <textarea
              rows={6}
              value={editing.system}
              placeholder="You are a meticulous senior engineer. Review code for correctness, security, and clarity. Be concise and specific; cite line numbers."
              onChange={(e) => setEditing({ ...editing, system: e.target.value })}
            />
          </label>
          <div className="editor-actions">
            <button className="btn-ghost" onClick={() => setEditing(null)}>
              Cancel
            </button>
            <button
              className="btn-primary"
              disabled={!editing.name.trim() || !editing.system.trim()}
              onClick={() => {
                onSave(editing);
                setEditing(null);
              }}
            >
              Save Mini-GPT
            </button>
          </div>
        </div>
      ) : personas.length === 0 ? (
        <div className="panel empty-panel">
          <p>No Mini-GPTs yet. Create one to give Overseer a focused personality and job.</p>
        </div>
      ) : (
        <div className="persona-grid">
          {personas.map((p) => (
            <div key={p.id} className={`persona-card ${activeId === p.id ? "active" : ""}`}>
              <div className="persona-top">
                <span className="persona-emoji">{p.emoji}</span>
                <div>
                  <div className="persona-name">{p.name}</div>
                  <div className="persona-model">{p.model || "default model"}</div>
                </div>
              </div>
              <p className="persona-system">{p.system}</p>
              <div className="persona-actions">
                <button
                  className={activeId === p.id ? "btn-ghost" : "btn-primary"}
                  onClick={() => onActivate(activeId === p.id ? null : p.id)}
                >
                  {activeId === p.id ? "Deactivate" : "Activate"}
                </button>
                <button className="btn-ghost" onClick={() => setEditing(p)}>
                  Edit
                </button>
                <button className="btn-danger" onClick={() => onDelete(p.id)}>
                  Delete
                </button>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
