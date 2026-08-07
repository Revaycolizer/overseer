import { useEffect, useRef, useState, type CSSProperties } from "react";
import { Approval, Msg, Persona, SLASH_COMMANDS } from "../lib/store";

const ICONS: Record<string, string> = {
  files: "M4 5.5A1.5 1.5 0 0 1 5.5 4h3l2 2h6A1.5 1.5 0 0 1 18 7.5v9A1.5 1.5 0 0 1 16.5 18h-11A1.5 1.5 0 0 1 4 16.5z",
  globe: "M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18M3 12h18M12 3c2.5 2.5 3.5 6 3.5 9s-1 6.5-3.5 9c-2.5-2.5-3.5-6-3.5-9s1-6.5 3.5-9",
  scaffold: "M4 7l8-4 8 4-8 4zM4 7v10l8 4M20 7v10l-8 4M12 11v10",
  memory: "M9 3a3 3 0 0 0-3 3 3 3 0 0 0-2 5 3 3 0 0 0 2 5 3 3 0 0 0 3 3V3zM15 3a3 3 0 0 1 3 3 3 3 0 0 1 2 5 3 3 0 0 1-2 5 3 3 0 0 1-3 3V3z",
};

const EXAMPLES = [
  { icon: "files", label: "List my Downloads and find duplicates", text: "/do list the files in my Downloads folder and find any duplicates" },
  { icon: "globe", label: "Read a webpage and summarize it", text: "/do read https://news.ycombinator.com and summarize the top 5 stories" },
  { icon: "scaffold", label: "Scaffold a project folder", text: "/do create a folder ~/overseer-demo with a README.md that says hello" },
  { icon: "memory", label: "Remember a preference", text: "/do remember that I prefer concise answers and dark mode" },
];

function Glyph({ name }: { name: string }) {
  return (
    <svg viewBox="0 0 24 24" width="17" height="17" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round">
      <path d={ICONS[name]} />
    </svg>
  );
}

export default function Chat({
  messages,
  busy,
  connected,
  persona,
  approvals,
  onResolve,
  input,
  setInput,
  onSend,
}: {
  messages: Msg[];
  busy: boolean;
  connected: boolean;
  persona: Persona | null;
  approvals: Approval[];
  onResolve: (id: string, ok: boolean) => void;
  input: string;
  setInput: (v: string) => void;
  onSend: (text: string) => void;
}) {
  const scrollRef = useRef<HTMLDivElement | null>(null);
  const taRef = useRef<HTMLTextAreaElement | null>(null);
  const [sel, setSel] = useState(0);

  useEffect(() => {
    scrollRef.current?.scrollTo({ top: scrollRef.current.scrollHeight, behavior: "smooth" });
  }, [messages, approvals]);

  function autosize() {
    const ta = taRef.current;
    if (!ta) return;
    ta.style.height = "0px";
    ta.style.height = Math.min(ta.scrollHeight, 160) + "px";
  }

  // Slash palette: active while typing the command token (a leading "/" with no space yet).
  const slashQuery = input.startsWith("/") && !input.includes(" ") ? input : "";
  const matches = slashQuery
    ? SLASH_COMMANDS.filter((c) => c.cmd.startsWith(slashQuery))
    : [];
  const paletteOpen = matches.length > 0;

  useEffect(() => {
    setSel(0);
  }, [slashQuery]);

  function pick(cmd: string) {
    setInput(cmd + " ");
    taRef.current?.focus();
    requestAnimationFrame(autosize);
  }

  const empty = messages.length === 0;
  const last = messages[messages.length - 1];
  const thinking = busy && last?.role === "assistant" && last.text.length === 0 && approvals.length === 0;

  return (
    <div className="chat">
      <div className="thread" ref={scrollRef}>
        {empty ? (
          <section className="hero">
            <h1 className="hero-title">
              {persona ? (
                <>
                  {persona.emoji} <span className="grad">{persona.name}</span>
                </>
              ) : (
                <>
                  What should I <span className="grad">take care of</span>?
                </>
              )}
            </h1>
            <p className="hero-sub">
              {persona
                ? "Your Mini-GPT is active — it shapes every reply. Ask away."
                : "I can see your screen, run commands, browse the web, and act — every step governed by the Kernel."}
            </p>
            <div className="chips">
              {EXAMPLES.map((e, i) => (
                <button
                  key={i}
                  className="chip"
                  style={{ "--d": `${i * 70}ms` } as CSSProperties}
                  onClick={() => {
                    setInput(e.text);
                    taRef.current?.focus();
                    requestAnimationFrame(autosize);
                  }}
                >
                  <span className="chip-ic"><Glyph name={e.icon} /></span>
                  <span>{e.label}</span>
                </button>
              ))}
            </div>
          </section>
        ) : (
          <div className="messages">
            {messages.map((m) =>
              m.role === "assistant" && m.text.length === 0 ? null : (
                <div key={m.id} className={`msg ${m.role}`}>
                  {m.role === "assistant" && (
                    <span className="ava" aria-hidden>
                      <span className="ava-core" />
                    </span>
                  )}
                  <div className="bubble">{m.text}</div>
                </div>
              )
            )}
            {thinking && (
              <div className="msg assistant">
                <span className="ava thinking" aria-hidden>
                  <span className="ava-core" />
                </span>
                <div className="bubble thinking-bubble">
                  <span className="dot" />
                  <span className="dot" />
                  <span className="dot" />
                </div>
              </div>
            )}
          </div>
        )}
      </div>

      <div className="composer-wrap">
        {approvals.map((a) => (
          <div key={a.id} className="approval">
            <div className="approval-body">
              <span className="approval-badge">approval</span>
              <div>
                <div className="approval-summary">{a.summary || `${a.tool} action`}</div>
                <div className="approval-meta">
                  <code>{a.tool}</code> · {a.kind.replace(/_/g, " ")}
                </div>
              </div>
            </div>
            <div className="approval-actions">
              <button className="btn-danger" onClick={() => onResolve(a.id, false)}>
                Reject
              </button>
              <button className="btn-primary" onClick={() => onResolve(a.id, true)}>
                Approve
              </button>
            </div>
          </div>
        ))}

        {persona && approvals.length === 0 && (
          <div className="persona-active">
            <span className="persona-emoji">{persona.emoji}</span>
            {persona.name} is active
          </div>
        )}

        <div className="composer-shell">
          {paletteOpen && (
            <div className="palette">
              {matches.map((c, i) => (
                <button
                  key={c.cmd}
                  className={`palette-item ${i === sel ? "on" : ""}`}
                  onMouseEnter={() => setSel(i)}
                  onClick={() => pick(c.cmd)}
                >
                  <span className="palette-cmd">{c.cmd}</span>
                  <span className="palette-args">{c.args}</span>
                  <span className="palette-desc">{c.desc}</span>
                </button>
              ))}
            </div>
          )}

          <div className={`composer ${busy ? "busy" : ""}`}>
            <textarea
              ref={taRef}
              value={input}
              rows={1}
              placeholder={connected ? (persona ? `Message ${persona.name}…` : "Ask Overseer to do something…  (type / for commands)") : "Start the engine:  overseer --server"}
              onChange={(e) => {
                setInput(e.target.value);
                autosize();
              }}
              onKeyDown={(e) => {
                if (paletteOpen) {
                  if (e.key === "ArrowDown") {
                    e.preventDefault();
                    setSel((s) => (s + 1) % matches.length);
                    return;
                  }
                  if (e.key === "ArrowUp") {
                    e.preventDefault();
                    setSel((s) => (s - 1 + matches.length) % matches.length);
                    return;
                  }
                  if (e.key === "Tab" || (e.key === "Enter" && !e.shiftKey)) {
                    e.preventDefault();
                    pick(matches[sel].cmd);
                    return;
                  }
                }
                if (e.key === "Enter" && !e.shiftKey) {
                  e.preventDefault();
                  onSend(input);
                  requestAnimationFrame(autosize);
                }
              }}
            />
            <button
              className="send"
              onClick={() => {
                onSend(input);
                requestAnimationFrame(autosize);
              }}
              disabled={busy || !input.trim()}
              aria-label="Send"
            >
              {busy ? (
                <span className="spinner" />
              ) : (
                <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round">
                  <path d="M12 19V5M5 12l7-7 7 7" />
                </svg>
              )}
            </button>
          </div>
        </div>
        <div className="hint">
          <kbd>Enter</kbd> to send · <kbd>Shift</kbd>+<kbd>Enter</kbd> for newline · <kbd>/</kbd> for commands
        </div>
      </div>
    </div>
  );
}
