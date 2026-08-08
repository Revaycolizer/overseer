import { useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { parseAssistant } from "../lib/store";

const WS_URL = "ws://127.0.0.1:8080/api/ws/chat";

/**
 * Floating always-on-top widget: a compact glass pill you talk (or type) to.
 * Own lightweight WS connection to the daemon; speaks replies aloud.
 */
export default function Widget() {
  const [connected, setConnected] = useState(false);
  const [listening, setListening] = useState(false);
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState("Talk to Overseer");
  const [input, setInput] = useState("");
  const wsRef = useRef<WebSocket | null>(null);
  const sendRef = useRef<(t: string) => void>(() => {});
  const talkRef = useRef<() => void>(() => {});

  // Transparent window background for the floating look.
  useEffect(() => {
    document.documentElement.style.background = "transparent";
    document.body.style.background = "transparent";
  }, []);

  useEffect(() => {
    let closed = false;
    let retry = 0;
    let timer: ReturnType<typeof setTimeout>;
    const connect = () => {
      const ws = new WebSocket(WS_URL);
      wsRef.current = ws;
      ws.onopen = () => {
        retry = 0;
        setConnected(true);
      };
      ws.onclose = () => {
        setConnected(false);
        if (!closed) {
          retry = Math.min(retry + 1, 6);
          timer = setTimeout(connect, Math.min(400 * 2 ** retry, 5000));
        }
      };
      ws.onerror = () => {
        try {
          ws.close();
        } catch {
          /* ignore */
        }
      };
      ws.onmessage = (ev) => {
        let f: { type: string; text?: string; response?: string; error?: string };
        try {
          f = JSON.parse(ev.data);
        } catch {
          return;
        }
        if (f.type === "token" && f.text) setStatus((s) => (s + f.text).slice(-180));
        else if (f.type === "done") {
          setBusy(false);
          if (f.response) setStatus(parseAssistant(f.response).slice(0, 240));
        } else if (f.type === "voice_result" && f.text) {
          sendRef.current(f.text);
        } else if (f.type === "voice_idle") {
          setListening(false);
        } else if (f.type === "voice_error") {
          setListening(false);
          setStatus(f.error || "voice unavailable");
        }
      };
    };
    connect();
    return () => {
      closed = true;
      clearTimeout(timer);
      wsRef.current?.close();
    };
  }, []);

  function send(text: string) {
    const t = text.trim();
    const ws = wsRef.current;
    if (!t || !ws || ws.readyState !== WebSocket.OPEN || busy) return;
    setStatus("…");
    setBusy(true);
    ws.send(JSON.stringify({ conversation_id: 1, message: t, speak: true }));
    setInput("");
  }
  sendRef.current = send;

  function talk() {
    const ws = wsRef.current;
    if (!ws || ws.readyState !== WebSocket.OPEN || listening || busy) return;
    setStatus("Listening…");
    ws.send(JSON.stringify({ voice: true }));
    setListening(true);
  }
  talkRef.current = talk;

  // Subscribe to hands-free wake signals (double-clap / "Hey Overseer"): summon
  // this window and start listening.
  useEffect(() => {
    let closed = false;
    let timer: ReturnType<typeof setTimeout>;
    const connect = () => {
      const ev = new WebSocket("ws://127.0.0.1:8080/api/ws/events");
      ev.onmessage = (e) => {
        try {
          if (JSON.parse(e.data).type === "wake") {
            const w = getCurrentWindow();
            w.show().catch(() => {});
            w.setFocus().catch(() => {});
            setTimeout(() => talkRef.current(), 250);
          }
        } catch {
          /* ignore */
        }
      };
      ev.onclose = () => {
        if (!closed) timer = setTimeout(connect, 1500);
      };
      ev.onerror = () => {
        try {
          ev.close();
        } catch {
          /* ignore */
        }
      };
    };
    connect();
    return () => {
      closed = true;
      clearTimeout(timer);
    };
  }, []);

  function hide() {
    getCurrentWindow().hide().catch(() => {});
  }

  return (
    <div className="widget-root">
      <div className="widget-pill" data-tauri-drag-region>
        <button
          className={`widget-orb ${listening ? "listening" : busy ? "thinking" : ""}`}
          onClick={talk}
          title="Tap to talk"
          aria-label="Talk"
        >
          <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round">
            <rect x="9" y="3" width="6" height="11" rx="3" />
            <path d="M5 11a7 7 0 0 0 14 0M12 18v3" />
          </svg>
        </button>

        <div className="widget-body">
          <div className="widget-status">{listening ? "Listening…" : busy ? "Thinking…" : status}</div>
          <input
            className="widget-input"
            value={input}
            placeholder="Ask or press the mic…"
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") send(input);
            }}
          />
        </div>

        <div className="widget-side">
          <span className={`widget-dot ${connected ? "on" : ""}`} title={connected ? "connected" : "offline"} />
          <button className="widget-x" onClick={hide} title="Hide (Ctrl+Alt+Space)" aria-label="Hide">
            <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
              <path d="M6 6l12 12M18 6L6 18" />
            </svg>
          </button>
        </div>
      </div>
    </div>
  );
}
