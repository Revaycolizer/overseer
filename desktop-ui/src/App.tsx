import { useEffect, useMemo, useRef, useState } from "react";
import "./App.css";
import Sidebar, { View } from "./components/Sidebar";
import Chat from "./screens/Chat";
import Studio from "./screens/Studio";
import Clock from "./screens/Clock";
import Settings from "./screens/Settings";
import {
  Approval,
  Msg,
  Persona,
  loadPersonas,
  savePersonas,
  getActivePersona,
  setActivePersona,
  getIdleSecs,
  setIdleSecs,
  parseAssistant,
} from "./lib/store";

const DEFAULT_URL = "ws://127.0.0.1:8080/api/ws/chat";

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
          <line x1="12" y1="1" x2="12" y2="3" /><line x1="12" y1="21" x2="12" y2="23" />
          <line x1="4.22" y1="4.22" x2="5.64" y2="5.64" /><line x1="18.36" y1="18.36" x2="19.78" y2="19.78" />
          <line x1="1" y1="12" x2="3" y2="12" /><line x1="21" y1="12" x2="23" y2="12" />
          <line x1="4.22" y1="19.78" x2="5.64" y2="18.36" /><line x1="18.36" y1="5.64" x2="19.78" y2="4.22" />
        </g>
      </svg>
    </button>
  );
}

export default function App() {
  const [url] = useState(() => localStorage.getItem("overseer.wsUrl") || DEFAULT_URL);
  const [token] = useState(() => localStorage.getItem("overseer.token") || "");
  const [conversationId] = useState(() => Number(localStorage.getItem("overseer.conv")) || 1);

  const [view, setView] = useState<View>("chat");
  const [collapsed, setCollapsed] = useState(() => localStorage.getItem("overseer.sidebar") === "1");
  const [messages, setMessages] = useState<Msg[]>([]);
  const [input, setInput] = useState("");
  const [connected, setConnected] = useState(false);
  const [busy, setBusy] = useState(false);
  const [approvals, setApprovals] = useState<Approval[]>([]);
  const [listening, setListening] = useState(false);

  const [personas, setPersonas] = useState<Persona[]>(() => loadPersonas());
  const [activeId, setActiveId] = useState<string | null>(() => getActivePersona());
  const [idleSecs, setIdleSecsState] = useState<number>(() => getIdleSecs());
  const [idle, setIdle] = useState(false);
  const [greeting, setGreeting] = useState<string | null>(null);
  const greetTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);

  const wsRef = useRef<WebSocket | null>(null);
  const stageRef = useRef<HTMLDivElement | null>(null);
  const idRef = useRef(1);
  const nextId = () => ++idRef.current;

  const activePersona = useMemo(() => personas.find((p) => p.id === activeId) ?? null, [personas, activeId]);

  const [, setThemeTick] = useState(0);
  const toggleTheme = () => {
    const next = document.documentElement.dataset.theme === "light" ? "dark" : "light";
    document.documentElement.dataset.theme = next;
    localStorage.setItem("overseer.theme", next);
    setThemeTick((t) => t + 1);
  };

  // ---- WebSocket with auto-reconnect ----
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
        if (frame.type === "token" && frame.text) appendAssistant(frame.text);
        else if (frame.type === "approval") {
          const a = frame as unknown as Approval;
          setApprovals((prev) => (prev.some((x) => x.id === a.id) ? prev : [...prev, a]));
        } else if (frame.type === "done") {
          setBusy(false);
          setApprovals([]);
          if (frame.response) replaceAssistant(parseAssistant(frame.response));
        } else if (frame.type === "voice_result") {
          setListening(false);
          if (frame.text) setInput((prev) => (prev ? prev + " " : "") + frame.text);
        } else if (frame.type === "voice_error") {
          setListening(false);
          setGreeting(frame.error || "Voice unavailable");
        } else if (frame.type === "error") {
          setBusy(false);
          setApprovals([]);
          appendAssistant(`\n⚠️ ${frame.error}`);
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

  // ---- Pointer-reactive aurora ----
  useEffect(() => {
    const el = stageRef.current?.querySelector<HTMLElement>(".aurora");
    if (!el) return;
    const onMove = (e: PointerEvent) => {
      const r = document.documentElement;
      el.style.setProperty("--mx", `${(e.clientX / r.clientWidth) * 100}%`);
      el.style.setProperty("--my", `${(e.clientY / r.clientHeight) * 100}%`);
    };
    window.addEventListener("pointermove", onMove);
    return () => window.removeEventListener("pointermove", onMove);
  }, []);

  // ---- Greet on summon (window becomes visible, e.g. via the tray hotkey) ----
  useEffect(() => {
    const greetNow = () => {
      const h = new Date().getHours();
      const part = h < 5 ? "Still up" : h < 12 ? "Good morning" : h < 18 ? "Good afternoon" : "Good evening";
      setGreeting(`${part} — what should I take care of?`);
      if (greetTimer.current) clearTimeout(greetTimer.current);
      greetTimer.current = setTimeout(() => setGreeting(null), 3800);
    };
    const onVis = () => {
      if (document.visibilityState === "visible") greetNow();
    };
    greetNow(); // greet on launch
    document.addEventListener("visibilitychange", onVis);
    return () => {
      document.removeEventListener("visibilitychange", onVis);
      if (greetTimer.current) clearTimeout(greetTimer.current);
    };
  }, []);

  // ---- Inactivity → ambient clock ----
  useEffect(() => {
    let last = Date.now();
    const wake = () => {
      last = Date.now();
      setIdle((was) => (was ? false : was));
    };
    const events = ["pointermove", "pointerdown", "keydown", "wheel"];
    events.forEach((e) => window.addEventListener(e, wake, { passive: true }));
    const tick = setInterval(() => {
      if (idleSecs > 0 && Date.now() - last > idleSecs * 1000) setIdle(true);
    }, 1000);
    return () => {
      events.forEach((e) => window.removeEventListener(e, wake));
      clearInterval(tick);
    };
  }, [idleSecs]);

  function appendAssistant(chunk: string) {
    setMessages((prev) => {
      const next = [...prev];
      const last = next[next.length - 1];
      if (last && last.role === "assistant") next[next.length - 1] = { ...last, text: last.text + chunk };
      else next.push({ role: "assistant", text: chunk, id: nextId() });
      return next;
    });
  }
  function replaceAssistant(text: string) {
    setMessages((prev) => {
      const next = [...prev];
      const last = next[next.length - 1];
      if (last && last.role === "assistant") next[next.length - 1] = { ...last, text };
      else next.push({ role: "assistant", text, id: nextId() });
      return next;
    });
  }

  function send(raw: string) {
    const text = raw.trim();
    if (!text || busy) return;
    const ws = wsRef.current;
    if (!ws || ws.readyState !== WebSocket.OPEN) {
      setMessages((prev) => [
        ...prev,
        { role: "user", text, id: nextId() },
        { role: "assistant", text: "⚠️ Not connected to the Overseer engine. Start it in a terminal:\n\n    overseer --server\n\n(it listens on 127.0.0.1:8080). Then send again.", id: nextId() },
      ]);
      setInput("");
      return;
    }
    // Apply the active Mini-GPT to normal chat (not to governed /do commands).
    const payload =
      activePersona && !text.startsWith("/")
        ? `${activePersona.system}\n\n---\nUser: ${text}`
        : text;

    setMessages((prev) => [
      ...prev,
      { role: "user", text, id: nextId() },
      { role: "assistant", text: "", id: nextId() },
    ]);
    ws.send(JSON.stringify({ conversation_id: conversationId, message: payload }));
    setInput("");
    setBusy(true);
  }

  const startVoice = () => {
    const ws = wsRef.current;
    if (!ws || ws.readyState !== WebSocket.OPEN || listening || busy) return;
    ws.send(JSON.stringify({ voice: true }));
    setListening(true);
  };

  const resolveApproval = (id: string, ok: boolean) => {
    const ws = wsRef.current;
    if (ws && ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify(ok ? { approve: id } : { reject: id }));
    }
    setApprovals((prev) => prev.filter((a) => a.id !== id));
  };

  // ---- Persona management ----
  const savePersona = (p: Persona) => {
    setPersonas((prev) => {
      const exists = prev.some((x) => x.id === p.id);
      const next = exists ? prev.map((x) => (x.id === p.id ? p : x)) : [...prev, p];
      savePersonas(next);
      return next;
    });
  };
  const deletePersona = (id: string) => {
    setPersonas((prev) => {
      const next = prev.filter((x) => x.id !== id);
      savePersonas(next);
      return next;
    });
    if (activeId === id) activate(null);
  };
  const activate = (id: string | null) => {
    setActiveId(id);
    setActivePersona(id);
    if (id) setView("chat");
  };

  const changeIdle = (n: number) => {
    setIdleSecsState(n);
    setIdleSecs(n);
  };
  const toggleSidebar = () => {
    setCollapsed((c) => {
      localStorage.setItem("overseer.sidebar", c ? "0" : "1");
      return !c;
    });
  };

  return (
    <div className="stage" ref={stageRef}>
      <div className="aurora" aria-hidden />
      <div className="grain" aria-hidden />

      {greeting && !idle && (
        <div className="greet">
          <span className="mark" aria-hidden>
            <span className="mark-core" />
          </span>
          {greeting}
        </div>
      )}

      {idle ? (
        <Clock idle />
      ) : (
        <div className="shell">
          <Sidebar view={view} onNavigate={setView} collapsed={collapsed} onToggle={toggleSidebar} connected={connected} />
          <div className="main">
            <div className="topbar">
              <div className="spacer" />
              <div className="bar-right">
                <div className={`conn ${connected ? "on" : "off"}`}>
                  <span className="pulse" />
                  {connected ? "connected" : "offline"}
                </div>
                <ThemeToggle onToggle={toggleTheme} />
              </div>
            </div>

            <div className="main-body">
              {view === "chat" && (
                <Chat
                  messages={messages}
                  busy={busy}
                  connected={connected}
                  persona={activePersona}
                  approvals={approvals}
                  onResolve={resolveApproval}
                  listening={listening}
                  onVoice={startVoice}
                  input={input}
                  setInput={setInput}
                  onSend={send}
                />
              )}
              {view === "studio" && (
                <Studio
                  personas={personas}
                  activeId={activeId}
                  onSave={savePersona}
                  onDelete={deletePersona}
                  onActivate={activate}
                />
              )}
              {view === "clock" && <Clock />}
              {view === "settings" && (
                <Settings
                  idleSecs={idleSecs}
                  onIdleChange={changeIdle}
                  connected={connected}
                  wsUrl={url}
                  personaCount={personas.length}
                />
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
