import { useEffect, useRef, useState, type PointerEvent as ReactPointerEvent } from "react";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";
import { parseAssistant } from "../lib/store";

const WS_URL = "ws://127.0.0.1:8080/api/ws/chat";
const COMPACT = new LogicalSize(430, 96);
const EXPANDED = new LogicalSize(460, 340);

/**
 * Floating always-on-top widget: a compact glass bar you talk (or type) to. It
 * expands to show the assistant's answer, then collapses again. Own lightweight
 * WS to the daemon; speaks replies. Summoned by hotkey, tray, or clap/wake.
 */
export default function Widget() {
  const [connected, setConnected] = useState(false);
  const [listening, setListening] = useState(false);
  const [busy, setBusy] = useState(false);
  const [input, setInput] = useState("");
  const [answer, setAnswer] = useState("");

  const wsRef = useRef<WebSocket | null>(null);
  const sendRef = useRef<(t: string) => void>(() => {});
  const talkRef = useRef<() => void>(() => {});
  const convoRef = useRef(false);
  const idleRef = useRef(0);
  const expandedRef = useRef(false);
  const answerBoxRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    document.documentElement.style.background = "transparent";
    document.body.style.background = "transparent";
  }, []);

  // Grow/shrink the window only on the empty↔non-empty transition (not per token).
  useEffect(() => {
    const has = answer.trim().length > 0;
    if (has !== expandedRef.current) {
      expandedRef.current = has;
      getCurrentWindow().setSize(has ? EXPANDED : COMPACT).catch(() => {});
    }
    answerBoxRef.current?.scrollTo({ top: answerBoxRef.current.scrollHeight });
  }, [answer]);

  useEffect(() => {
    const un = listen("overseer-wake", () => {
      convoRef.current = true;
      idleRef.current = 0;
      setTimeout(() => talkRef.current(), 250);
    });
    return () => {
      un.then((f) => f()).catch(() => {});
    };
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
        if (f.type === "token" && f.text) setAnswer((a) => a + f.text);
        else if (f.type === "done") {
          setBusy(false);
          idleRef.current = 0;
          if (f.response) setAnswer(parseAssistant(f.response));
          if (convoRef.current) setTimeout(() => talkRef.current(), 700);
        } else if (f.type === "voice_result" && f.text) {
          idleRef.current = 0;
          sendRef.current(f.text);
        } else if (f.type === "voice_idle") {
          setListening(false);
          idleRef.current += 1;
          if (convoRef.current && idleRef.current < 2) setTimeout(() => talkRef.current(), 400);
          else convoRef.current = false;
        } else if (f.type === "voice_error") {
          setListening(false);
          setAnswer(f.error || "voice unavailable");
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
    setAnswer("");
    setBusy(true);
    ws.send(JSON.stringify({ conversation_id: 1, message: t, speak: true }));
    setInput("");
  }
  sendRef.current = send;

  function talk() {
    const ws = wsRef.current;
    if (!ws || ws.readyState !== WebSocket.OPEN || listening || busy) return;
    ws.send(JSON.stringify({ voice: true }));
    setListening(true);
  }
  talkRef.current = talk;

  function toggleConvo() {
    if (convoRef.current || listening) {
      convoRef.current = false;
      setListening(false);
    } else {
      convoRef.current = true;
      idleRef.current = 0;
      talk();
    }
  }

  function onPillPointerDown(e: ReactPointerEvent) {
    if ((e.target as HTMLElement).closest("button, input, .wc-answer")) return;
    getCurrentWindow().startDragging().catch(() => {});
  }

  function hide() {
    convoRef.current = false;
    setAnswer("");
    getCurrentWindow().hide().catch(() => {});
  }

  const state = listening ? "Listening…" : busy ? "Thinking…" : connected ? "Ready" : "Offline";

  return (
    <div className="widget-root">
      <div className={`wc ${answer ? "wc-open" : ""}`} onPointerDown={onPillPointerDown}>
        <div className="wc-head">
          <button
            className={`wc-orb ${listening ? "listening" : busy ? "thinking" : ""}`}
            onClick={toggleConvo}
            title={convoRef.current || listening ? "Stop" : "Tap to talk"}
            aria-label="Talk"
          >
            <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round">
              <rect x="9" y="3" width="6" height="11" rx="3" />
              <path d="M5 11a7 7 0 0 0 14 0M12 18v3" />
            </svg>
          </button>

          <input
            className="wc-input"
            value={input}
            placeholder={state === "Ready" ? "Ask Overseer…" : state}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") send(input);
            }}
          />

          {input.trim() && (
            <button className="wc-send" onClick={() => send(input)} aria-label="Send">
              <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round">
                <path d="M12 19V5M5 12l7-7 7 7" />
              </svg>
            </button>
          )}
          <span className={`wc-dot ${connected ? "on" : ""}`} title={connected ? "connected" : "offline"} />
          <button className="wc-x" onClick={hide} title="Hide (Ctrl+Alt+Space)" aria-label="Hide">
            <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
              <path d="M6 6l12 12M18 6L6 18" />
            </svg>
          </button>
        </div>

        {answer && (
          <div className="wc-answer" ref={answerBoxRef}>
            {answer}
            {busy && <span className="wc-caret" />}
          </div>
        )}
      </div>
    </div>
  );
}
