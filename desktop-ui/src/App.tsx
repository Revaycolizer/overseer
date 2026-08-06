import { useEffect, useMemo, useRef, useState, type CSSProperties } from "react";
import "./App.css";

type Role = "user" | "assistant";
interface Msg {
  role: Role;
  text: string;
  id: number;
}

// Overseer engine daemon: `overseer --server` (loopback, port 8080).
const DEFAULT_URL = "ws://127.0.0.1:8080/api/ws/chat";

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

// Animated sun↔moon toggle. The morph is driven purely by CSS reacting to the
// root's data-theme (a mask circle slides in to carve the crescent; rays fade).
function ThemeToggle({ onToggle }: { onToggle: () => void }) {
  return (
    <button className="theme-toggle" onClick={onToggle} type="button" aria-label="Toggle dark / light theme">
      <svg className="tt-svg" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
        <mask id="tt-moon-mask">
          <rect x="0" y="0" width="24" height="24" fill="white" />
          <circle className="tt-cut" cx="25" cy="0" r="8" fill="black" />
        </mask>
        <circle className="tt-disc" cx="12" cy="12" r="5" fill="currentColor" mask="url(#tt-moon-mask)" />
        <g className="tt-rays">
          <line x1="12" y1="1" x2="12" y2="3" />
          <line x1="12" y1="21" x2="12" y2="23" />
          <line x1="4.22" y1="4.22" x2="5.64" y2="5.64" />
          <line x1="18.36" y1="18.36" x2="19.78" y2="19.78" />
          <line x1="1" y1="12" x2="3" y2="12" />
          <line x1="21" y1="12" x2="23" y2="12" />
          <line x1="4.22" y1="19.78" x2="5.64" y2="18.36" />
          <line x1="18.36" y1="5.64" x2="19.78" y2="4.22" />
        </g>
      </svg>
    </button>
  );
}

export default function App() {
  const [url] = useState(() => localStorage.getItem("overseer.wsUrl") || DEFAULT_URL);
  const [token] = useState(() => localStorage.getItem("overseer.token") || "");
  const [conversationId] = useState(() => Number(localStorage.getItem("overseer.conv")) || 1);

  const [messages, setMessages] = useState<Msg[]>([]);
  const [input, setInput] = useState("");
  const [connected, setConnected] = useState(false);
  const [busy, setBusy] = useState(false);
  const [, setTheme] = useState<string>(() => document.documentElement.dataset.theme || "dark");

  const toggleTheme = () => {
    const next = document.documentElement.dataset.theme === "light" ? "dark" : "light";
    document.documentElement.dataset.theme = next;
    localStorage.setItem("overseer.theme", next);
    setTheme(next);
  };
  const wsRef = useRef<WebSocket | null>(null);
  const scrollRef = useRef<HTMLDivElement | null>(null);
  const stageRef = useRef<HTMLDivElement | null>(null);
  const taRef = useRef<HTMLTextAreaElement | null>(null);
  const idRef = useRef(1);

  const nextId = () => ++idRef.current;

  useEffect(() => {
    let closed = false;
    let retry = 0;
    let timer: ReturnType<typeof setTimeout>;

    const connect = () => {
      const ws = new WebSocket(url);
      wsRef.current = ws;
      ws.onopen = () => {
        retry = 0;
        setConnected(true);
        if (token) ws.send(JSON.stringify({ token }));
      };
      ws.onmessage = (ev) => {
        let frame: { type: string; text?: string; response?: string; error?: string };
        try {
          frame = JSON.parse(ev.data);
        } catch {
          return;
        }
        if (frame.type === "token" && frame.text) appendToLastAssistant(frame.text);
        else if (frame.type === "done") {
          setBusy(false);
          if (frame.response) replaceLastAssistant(frame.response);
        } else if (frame.type === "error") {
          setBusy(false);
          appendToLastAssistant(`\n⚠️ ${frame.error}`);
        }
      };
      ws.onerror = () => {
        try {
          ws.close();
        } catch {
          /* ignore */
        }
      };
      ws.onclose = () => {
        setConnected(false);
        if (!closed) {
          // Reconnect with backoff so the app recovers when the daemon
          // (re)starts — no manual reload needed.
          retry = Math.min(retry + 1, 6);
          timer = setTimeout(connect, Math.min(400 * 2 ** retry, 5000));
        }
      };
    };

    connect();
    return () => {
      closed = true;
      clearTimeout(timer);
      wsRef.current?.close();
    };
  }, [url, token]);

  useEffect(() => {
    scrollRef.current?.scrollTo({ top: scrollRef.current.scrollHeight, behavior: "smooth" });
  }, [messages]);

  // Pointer-reactive aurora: track the cursor as CSS vars for a subtle parallax.
  useEffect(() => {
    const el = stageRef.current;
    if (!el) return;
    const onMove = (e: PointerEvent) => {
      const r = el.getBoundingClientRect();
      el.style.setProperty("--mx", `${((e.clientX - r.left) / r.width) * 100}%`);
      el.style.setProperty("--my", `${((e.clientY - r.top) / r.height) * 100}%`);
    };
    window.addEventListener("pointermove", onMove);
    return () => window.removeEventListener("pointermove", onMove);
  }, []);

  function appendToLastAssistant(chunk: string) {
    setMessages((prev) => {
      const next = [...prev];
      const last = next[next.length - 1];
      if (last && last.role === "assistant") next[next.length - 1] = { ...last, text: last.text + chunk };
      else next.push({ role: "assistant", text: chunk, id: nextId() });
      return next;
    });
  }
  function replaceLastAssistant(text: string) {
    setMessages((prev) => {
      const next = [...prev];
      const last = next[next.length - 1];
      if (last && last.role === "assistant") next[next.length - 1] = { ...last, text };
      else next.push({ role: "assistant", text, id: nextId() });
      return next;
    });
  }

  function send(textOverride?: string) {
    const text = (textOverride ?? input).trim();
    if (!text || busy) return;
    const ws = wsRef.current;
    if (!ws || ws.readyState !== WebSocket.OPEN) {
      // Don't fail silently — tell the user why nothing happened.
      setMessages((prev) => [
        ...prev,
        { role: "user", text, id: nextId() },
        {
          role: "assistant",
          text: "⚠️ Not connected to the Overseer engine. Start it in a terminal:\n\n    overseer --server\n\n(or:  cargo run -- server)  — it listens on 127.0.0.1:8080. Then send again.",
          id: nextId(),
        },
      ]);
      setInput("");
      requestAnimationFrame(() => autosize());
      return;
    }
    setMessages((prev) => [
      ...prev,
      { role: "user", text, id: nextId() },
      { role: "assistant", text: "", id: nextId() },
    ]);
    ws.send(JSON.stringify({ conversation_id: conversationId, message: text }));
    setInput("");
    setBusy(true);
    requestAnimationFrame(() => autosize());
  }

  function autosize() {
    const ta = taRef.current;
    if (!ta) return;
    ta.style.height = "0px";
    ta.style.height = Math.min(ta.scrollHeight, 160) + "px";
  }

  const empty = messages.length === 0;
  const lastAssistantEmpty = useMemo(() => {
    const last = messages[messages.length - 1];
    return last?.role === "assistant" && last.text.length === 0;
  }, [messages]);

  return (
    <div className="stage" ref={stageRef}>
      <div className="aurora" aria-hidden />
      <div className="grain" aria-hidden />

      <header className="bar">
        <div className="brand">
          <span className="mark" aria-hidden>
            <span className="mark-core" />
          </span>
          <span className="wordmark">Overseer</span>
        </div>
        <div className="bar-right">
          <div className={`conn ${connected ? "on" : "off"}`}>
            <span className="pulse" />
            {connected ? "connected" : "offline"}
          </div>
          <ThemeToggle onToggle={toggleTheme} />
        </div>
      </header>

      <main className="thread" ref={scrollRef}>
        {empty ? (
          <section className="hero">
            <h1 className="hero-title">
              What should I <span className="grad">take care of</span>?
            </h1>
            <p className="hero-sub">
              I can see your screen, run commands, browse the web, and act — every step governed by the Kernel.
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
            {busy && lastAssistantEmpty && (
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
      </main>

      <footer className="composer-wrap">
        <div className={`composer ${busy ? "busy" : ""}`}>
          <textarea
            ref={taRef}
            value={input}
            rows={1}
            placeholder={connected ? "Ask Overseer to do something…" : "Start the engine:  overseer --server"}
            onChange={(e) => {
              setInput(e.target.value);
              autosize();
            }}
            onKeyDown={(e) => {
              if (e.key === "Enter" && !e.shiftKey) {
                e.preventDefault();
                send();
              }
            }}
          />
          <button className="send" onClick={() => send()} disabled={busy || !input.trim()} aria-label="Send">
            {busy ? (
              <span className="spinner" />
            ) : (
              <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round">
                <path d="M12 19V5M5 12l7-7 7 7" />
              </svg>
            )}
          </button>
        </div>
        <div className="hint">
          <kbd>Enter</kbd> to send · <kbd>Shift</kbd>+<kbd>Enter</kbd> for newline · try <code>/do …</code>
        </div>
      </footer>
    </div>
  );
}
