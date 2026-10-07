import { refreshConnection, startupError } from "../lib/connection";
import { useEffect, useRef, useState, type PointerEvent as ReactPointerEvent } from "react";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";
import { parseAssistant, type Approval } from "../lib/store";

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
  const [approvals, setApprovals] = useState<Approval[]>([]);

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
    const has = answer.trim().length > 0 || approvals.length > 0;
    if (has !== expandedRef.current) {
      expandedRef.current = has;
      getCurrentWindow().setSize(has ? EXPANDED : COMPACT).catch(() => {});
    }
    answerBoxRef.current?.scrollTo({ top: answerBoxRef.current.scrollHeight });
  }, [answer, approvals]);

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
    const connect = async () => {
      if (closed || startupError) return;
      let connection;
      try { connection = await refreshConnection(); }
      catch { if (!closed) timer = setTimeout(connect, 2000); return; }
      if (closed) return;
      const ws = new WebSocket(connection.url || WS_URL);
      wsRef.current = ws;
      ws.onopen = () => {
        retry = 0;
        setConnected(true);
        const token = connection.token;
        if (token) ws.send(JSON.stringify({ token }));
      };
      ws.onclose = () => {
        setConnected(false);
        setBusy(false);
        setListening(false);
        setApprovals([]);
        convoRef.current = false;
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
        let f: { type: string; text?: string; response?: string; error?: string; conversation_id?: string };
        try {
          f = JSON.parse(ev.data);
        } catch {
          return;
        }
        if (f.type === "session" && f.conversation_id) {
          localStorage.setItem("overseer.conv", f.conversation_id);
        } else if (f.type === "approval") {
          const approval = f as unknown as Approval;
          setApprovals((prev) => prev.some((a) => a.id === approval.id) ? prev : [...prev, approval]);
        } else if (f.type === "token" && f.text) setAnswer((a) => a + f.text);
        else if (f.type === "done") {
          setBusy(false);
          setApprovals([]);
          idleRef.current = 0;
          if (f.response) setAnswer(parseAssistant(f.response));
          if (convoRef.current) setTimeout(() => talkRef.current(), 700);
        } else if (f.type === "voice_result" && f.text) {
          setListening(false);
          idleRef.current = 0;
          sendRef.current(f.text);
        } else if (f.type === "voice_idle") {
          setListening(false);
          idleRef.current += 1;
          if (convoRef.current && idleRef.current < 2) setTimeout(() => talkRef.current(), 400);
          else convoRef.current = false;
        } else if (f.type === "voice_error") {
          setListening(false);
          convoRef.current = false;
          setAnswer(f.error || "voice unavailable");
        } else if (f.type === "error") {
          setBusy(false);
          setListening(false);
          setApprovals([]);
          convoRef.current = false;
          setAnswer(f.error || "The engine could not complete this request.");
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
    setApprovals([]);
    setBusy(true);
    ws.send(JSON.stringify({ conversation_id: localStorage.getItem("overseer.conv") || "0", message: t, speak: true }));
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

  function resolveApproval(id: string, approved: boolean) {
    const ws = wsRef.current;
    if (!ws || ws.readyState !== WebSocket.OPEN) return;
    ws.send(JSON.stringify(approved ? { approve: id } : { reject: id }));
    setApprovals((prev) => prev.filter((a) => a.id !== id));
  }

  const state = listening ? "Listening…" : busy ? "Thinking…" : connected ? "Ready" : "Offline";

  return (
    <div className="widget-root">
      <div className={`wc ${answer || approvals.length ? "wc-open" : ""}`} onPointerDown={onPillPointerDown}>
        <div className="wc-head">
          <button
            className={`wc-orb ${listening ? "listening" : busy ? "thinking" : ""}`}
            onClick={toggleConvo}
            title={convoRef.current || listening ? "Stop" : "Tap to talk"}
            aria-label="Talk"
            disabled={!connected || busy}
          >
            <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round">
              <rect x="9" y="3" width="6" height="11" rx="3" />
              <path d="M5 11a7 7 0 0 0 14 0M12 18v3" />
            </svg>
          </button>

          <input
            className="wc-input"
            aria-label="Message Overseer"
            value={input}
            placeholder={state === "Ready" ? "Ask Overseer…" : state}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") send(input);
            }}
          />

          {input.trim() && (
            <button className="wc-send" onClick={() => send(input)} aria-label="Send" disabled={!connected || busy}>
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

        {approvals.length > 0 && (
          <div className="wc-approvals" aria-label="Actions awaiting approval">
            {approvals.map((a) => (
              <div className="wc-approval" key={a.id}>
                <strong>{a.summary || `${a.tool} action`}</strong>
                {a.reason && <p>{a.reason}</p>}
                <div className="approval-actions">
                  <button className="btn-danger" onClick={() => resolveApproval(a.id, false)}>Reject</button>
                  <button className="btn-primary" onClick={() => resolveApproval(a.id, true)}>Approve</button>
                </div>
              </div>
            ))}
          </div>
        )}

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
