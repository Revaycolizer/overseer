import { useEffect, useMemo, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
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

export default function App() {
  const [url] = useState(() => localStorage.getItem("overseer.wsUrl") || DEFAULT_URL);
  const [token] = useState(() => localStorage.getItem("overseer.token") || "");
  const [conversationId] = useState(() => Number(localStorage.getItem("overseer.conv")) || 1);

  const [view, setView] = useState<View>("chat");
  const [collapsed, setCollapsed] = useState(() => localStorage.getItem("overseer.sidebar") === "1");
  const [isFs, setIsFs] = useState(false);
  const [messages, setMessages] = useState<Msg[]>([]);
  const [input, setInput] = useState("");
  const [connected, setConnected] = useState(false);
  const [busy, setBusy] = useState(false);
  const [approvals, setApprovals] = useState<Approval[]>([]);
  const [listening, setListening] = useState(false);
  const [voiceMode, setVoiceMode] = useState(false);
  const voiceModeRef = useRef(false);
  const voiceIdleRef = useRef(0);
  const sendRef = useRef<(raw: string, opts?: { speak?: boolean }) => void>(() => {});
  const startVoiceRef = useRef<() => void>(() => {});

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
          // Hands-free: after the reply is spoken, listen again for a reply.
          voiceIdleRef.current = 0;
          if (voiceModeRef.current) setTimeout(() => startVoiceRef.current(), 900);
        } else if (frame.type === "voice_result") {
          setListening(false);
          voiceIdleRef.current = 0;
          if (frame.text) {
            if (voiceModeRef.current) sendRef.current(frame.text, { speak: true });
            else setInput((prev) => (prev ? prev + " " : "") + frame.text);
          }
        } else if (frame.type === "voice_idle") {
          setListening(false);
          voiceIdleRef.current += 1;
          // Keep listening across brief silences, but stop after two so it isn't
          // stuck listening forever (and hogging the mic from clap-wake).
          if (voiceModeRef.current && voiceIdleRef.current < 2) {
            setTimeout(() => startVoiceRef.current(), 400);
          } else if (voiceModeRef.current) {
            voiceModeRef.current = false;
            setVoiceMode(false);
          }
        } else if (frame.type === "voice_error") {
          setListening(false);
          if (voiceModeRef.current) {
            voiceModeRef.current = false;
            setVoiceMode(false); // stop the hands-free loop on a miss
          }
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

  // ---- Fullscreen: always leave an exit (Esc / F11) + in-app controls ----
  useEffect(() => {
    let win: ReturnType<typeof getCurrentWindow> | null = null;
    let unlisten: (() => void) | undefined;
    try {
      win = getCurrentWindow();
    } catch {
      return; // running outside Tauri (plain browser dev)
    }
    const w = win;
    const sync = () => w.isFullscreen().then(setIsFs).catch(() => {});
    sync();
    w.onResized(sync).then((u) => (unlisten = u)).catch(() => {});
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        w.isFullscreen().then((v) => { if (v) w.setFullscreen(false); }).catch(() => {});
      } else if (e.key === "F11") {
        e.preventDefault();
        w.isFullscreen().then((v) => w.setFullscreen(!v)).catch(() => {});
      }
    };
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
      unlisten?.();
    };
  }, []);

  const winMinimize = () => getCurrentWindow().minimize().catch(() => {});
  const winExitFs = () => getCurrentWindow().setFullscreen(false).catch(() => {});
  const winClose = () => getCurrentWindow().close().catch(() => {});

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

  function send(raw: string, opts?: { speak?: boolean }) {
    const text = raw.trim();
    if (!text || busy) return;
    const speak = opts?.speak ?? voiceModeRef.current;
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
    ws.send(JSON.stringify({ conversation_id: conversationId, message: payload, speak }));
    setInput("");
    setBusy(true);
  }

  const toggleVoiceMode = () => {
    const next = !voiceModeRef.current;
    voiceModeRef.current = next;
    setVoiceMode(next);
    voiceIdleRef.current = 0;
    if (next) startVoice();
    else setListening(false);
  };

  // Keep refs pointing at the latest closures for the WS handler to call.
  useEffect(() => {
    sendRef.current = send;
    startVoiceRef.current = startVoice;
  });

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

      {isFs && (
        <div className="winctl" role="group" aria-label="Window controls">
          <button className="winctl-btn" onClick={winMinimize} title="Minimize" aria-label="Minimize">
            <svg viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round"><line x1="5" y1="12" x2="19" y2="12" /></svg>
          </button>
          <button className="winctl-btn" onClick={winExitFs} title="Exit full screen (Esc)" aria-label="Exit full screen">
            <svg viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round"><path d="M9 4v5H4M15 4v5h5M9 20v-5H4M15 20v-5h5" /></svg>
          </button>
          <button className="winctl-btn danger" onClick={winClose} title="Close" aria-label="Close">
            <svg viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round"><line x1="6" y1="6" x2="18" y2="18" /><line x1="18" y1="6" x2="6" y2="18" /></svg>
          </button>
        </div>
      )}

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
                  voiceMode={voiceMode}
                  onToggleVoiceMode={toggleVoiceMode}
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
