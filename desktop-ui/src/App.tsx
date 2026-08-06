import { useEffect, useRef, useState } from "react";
import "./App.css";

type Role = "user" | "assistant";
interface Msg {
  role: Role;
  text: string;
}

// Overseer engine daemon: `overseer --server` (loopback, port 8080).
const DEFAULT_URL = "ws://127.0.0.1:8080/api/ws/chat";

export default function App() {
  const [url] = useState(
    () => localStorage.getItem("overseer.wsUrl") || DEFAULT_URL,
  );
  const [token] = useState(() => localStorage.getItem("overseer.token") || "");
  const [conversationId] = useState(
    () => Number(localStorage.getItem("overseer.conv")) || 1,
  );

  const [messages, setMessages] = useState<Msg[]>([]);
  const [input, setInput] = useState("");
  const [connected, setConnected] = useState(false);
  const [busy, setBusy] = useState(false);
  const wsRef = useRef<WebSocket | null>(null);
  const scrollRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    const ws = new WebSocket(url);
    wsRef.current = ws;
    ws.onopen = () => {
      setConnected(true);
      if (token) ws.send(JSON.stringify({ token }));
    };
    ws.onclose = () => setConnected(false);
    ws.onerror = () => setConnected(false);
    ws.onmessage = (ev) => {
      let frame: { type: string; text?: string; response?: string; error?: string };
      try {
        frame = JSON.parse(ev.data);
      } catch {
        return;
      }
      if (frame.type === "token" && frame.text) {
        appendToLastAssistant(frame.text);
      } else if (frame.type === "done") {
        setBusy(false);
        // Reconcile the streamed text with the authoritative final response.
        if (frame.response) replaceLastAssistant(frame.response);
      } else if (frame.type === "error") {
        setBusy(false);
        appendToLastAssistant(`\n[error: ${frame.error}]`);
      }
    };
    return () => ws.close();
  }, [url, token]);

  useEffect(() => {
    scrollRef.current?.scrollTo({ top: scrollRef.current.scrollHeight });
  }, [messages]);

  function appendToLastAssistant(chunk: string) {
    setMessages((prev) => {
      const next = [...prev];
      const last = next[next.length - 1];
      if (last && last.role === "assistant") {
        next[next.length - 1] = { ...last, text: last.text + chunk };
      } else {
        next.push({ role: "assistant", text: chunk });
      }
      return next;
    });
  }

  function replaceLastAssistant(text: string) {
    setMessages((prev) => {
      const next = [...prev];
      const last = next[next.length - 1];
      if (last && last.role === "assistant") {
        next[next.length - 1] = { ...last, text };
      } else {
        next.push({ role: "assistant", text });
      }
      return next;
    });
  }

  function send() {
    const text = input.trim();
    const ws = wsRef.current;
    if (!text || !ws || ws.readyState !== WebSocket.OPEN || busy) return;
    setMessages((prev) => [
      ...prev,
      { role: "user", text },
      { role: "assistant", text: "" },
    ]);
    ws.send(JSON.stringify({ conversation_id: conversationId, message: text }));
    setInput("");
    setBusy(true);
  }

  return (
    <div className="app">
      <header className="bar">
        <span className="title">Overseer</span>
        <span className={connected ? "dot on" : "dot off"} />
        <span className="status">{connected ? "connected" : "offline"}</span>
      </header>

      <div className="thread" ref={scrollRef}>
        {messages.length === 0 && (
          <div className="empty">
            Ask anything, or run a governed command:{" "}
            <code>/write ~/notes.txt hello</code>,{" "}
            <code>/open https://example.com</code>, <code>/screenshot</code>.
          </div>
        )}
        {messages.map((m, i) => (
          <div key={i} className={`msg ${m.role}`}>
            {m.text || (m.role === "assistant" && busy ? "…" : "")}
          </div>
        ))}
      </div>

      <div className="composer">
        <textarea
          value={input}
          placeholder={
            connected ? "Message Overseer…" : "Start the engine: overseer --server"
          }
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && !e.shiftKey) {
              e.preventDefault();
              send();
            }
          }}
        />
        <button onClick={send} disabled={!connected || busy}>
          Send
        </button>
      </div>
    </div>
  );
}
