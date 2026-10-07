import Onboarding from "./screens/Onboarding";
import { initialConnection, managed, refreshConnection, startupError } from "./lib/connection";
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
  const [url, setUrl] = useState(() => initialConnection.url || DEFAULT_URL);
  const [token, setToken] = useState(() => initialConnection.token);
  const [conversationId, setConversationId] = useState(() => localStorage.getItem("overseer.conv") || "0");

  const [setupDone, setSetupDone] = useState(false);
  useEffect(() => setSetupDone(false), [url, token]);
  const [view, setView] = useState<View>("chat");
  const [collapsed, setCollapsed] = useState(() => localStorage.getItem("overseer.sidebar") === "1");
  const [isFs, setIsFs] = useState(false);
  const [messages, setMessages] = useState<Msg[]>([]);
  const [input, setInput] = useState("");
  const [connected, setConnected] = useState(false);
  const [historyReady, setHistoryReady] = useState(false);
  const [connectionError, setConnectionError] = useState(startupError);
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
  const idRef = useRef(1);
  const nextId = () => ++idRef.current;

  const activePersona = useMemo(() => personas.find((p) => p.id === activeId) ?? null, [personas, activeId]);

  // ---- WebSocket with auto-reconnect ----
  useEffect(() => {
    let closed = false;
    let retry = 0;
    let timer: ReturnType<typeof setTimeout>;
    const connect = () => {
      if (startupError) return;
      const ws = new WebSocket(url);
      wsRef.current = ws;
      ws.onopen = () => {
        if (closed || wsRef.current !== ws) return;
        retry = 0;
        setConnected(true);
        if (token) ws.send(JSON.stringify({ token }));
        setHistoryReady(false);
        const id = localStorage.getItem("overseer.conv") || "0";
        const restore = async () => {
          if (id === "0") { setConnectionError(""); setHistoryReady(true); return; }
          try {
            const endpoint = new URL(url);
            endpoint.protocol = endpoint.protocol === "wss:" ? "https:" : "http:";
            endpoint.pathname = "/api/messages";
            endpoint.search = new URLSearchParams({ conversation_id: id }).toString();
            const response = await fetch(endpoint, { headers: token ? { Authorization: `Bearer ${token}` } : {}, signal: AbortSignal.timeout(10000) });
            if (!response.ok) throw new Error(`History request failed (${response.status})`);
            const rows = JSON.parse((await response.text()).replace(/("id"\s*:\s*)(-?\d+)/g, '$1"$2"')) as { id: string; role: string; content: string }[];
            if (!Array.isArray(rows)) throw new Error("Unexpected history response");
            if (closed || wsRef.current !== ws) return;
            setMessages(rows.filter(m => m.role === "user" || m.role === "assistant").map(m => ({ id: `db:${m.id}`, role: m.role as "user" | "assistant", text: m.role === "assistant" ? parseAssistant(m.content) : m.content })));
            setConnectionError("");
            setHistoryReady(true);
          } catch (error) {
            if (!closed) setConnectionError(`Could not restore saved chat: ${String(error)}`);
          }
        };
        void restore();
      };
      ws.onmessage = (ev) => {
        if (closed || wsRef.current !== ws) return;
        let frame: { type: string; text?: string; response?: string; error?: string; conversation_id?: string };
        try {
          frame = JSON.parse(ev.data);
        } catch {
          return;
        }
        if (frame.type === "session" && frame.conversation_id) {
          localStorage.setItem("overseer.conv", frame.conversation_id);
          setConversationId(frame.conversation_id);
        } else if (frame.type === "token" && frame.text) appendAssistant(frame.text);
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
          appendAssistant(`⚠️ ${frame.error || "Voice unavailable"}`);
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
        if (closed || wsRef.current !== ws) return;
        setHistoryReady(false);
        setConnected(false);
        setBusy(false);
        setListening(false);
        setApprovals([]);
        voiceModeRef.current = false;
        setVoiceMode(false);
        if (!closed) {
          retry = Math.min(retry + 1, 6);
          timer = setTimeout(async () => {
            if (managed && url === initialConnection.url) {
              try {
                const connection = await refreshConnection();
                if (closed) return;
                if (connection.url !== url || connection.token !== token) {
                  setUrl(connection.url); setToken(connection.token); return;
                }
              } catch (error) { setConnectionError(String(error)); }
            }
            if (!closed) connect();
          }, Math.min(400 * 2 ** retry, 5000));
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
    if (!text || busy || !historyReady) return;
    const speak = opts?.speak ?? voiceModeRef.current;
    const ws = wsRef.current;
    if (!ws || ws.readyState !== WebSocket.OPEN) {
      setMessages((prev) => [
        ...prev,
        { role: "user", text, id: nextId() },
        { role: "assistant", text: "⚠️ Overseer is reconnecting. Please wait and try again.", id: nextId() },
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
    ws.send(JSON.stringify({ conversation_id: conversationId, message: payload, speak, model: activePersona?.model || undefined }));
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
      setApprovals((prev) => prev.filter((a) => a.id !== id));
    }
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
    <div className="stage">
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
                <div className={`conn ${connected ? "on" : "off"}`} role="status">
                  <span className="pulse" />
                  {connected ? "connected" : "offline"}
                </div>
              </div>
            </div>

            <div className="main-body">
              {(!connected || !historyReady || connectionError) && (
                <div className="connection-notice" role="status">
                  <strong>{connectionError ? "Overseer needs attention" : connected ? "Restoring saved chat…" : "Starting Overseer…"}</strong>
                  <span>{connectionError || (managed ? "Your local backend starts automatically. This window reconnects if the connection is interrupted." : "Connecting to the backend configured in Settings.")}</span>
                  {connectionError && <button onClick={() => window.location.reload()}>Retry connection</button>}
                </div>
              )}
              {view === "chat" && connected && historyReady && !setupDone && <Onboarding url={url} token={token} onComplete={() => setSetupDone(true)} />}
              {view === "chat" && (setupDone || !connected || !historyReady) && (
                <Chat
                  messages={messages}
                  busy={busy}
                  connected={connected && historyReady}
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
                  token={token}
                  onConnectionSave={(nextUrl, nextToken) => {
                    if (managed && nextUrl === initialConnection.url) {
                      localStorage.removeItem("overseer.wsUrl");
                      localStorage.removeItem("overseer.token");
                    } else {
                      localStorage.setItem("overseer.wsUrl", nextUrl);
                      localStorage.setItem("overseer.token", nextToken);
                    }
                    setUrl(nextUrl);
                    setToken(nextToken);
                  }}
                />
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
