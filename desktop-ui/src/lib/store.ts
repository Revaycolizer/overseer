// Local persistence for personas ("mini-GPTs") and app preferences.

export interface Persona {
  id: string;
  name: string;
  emoji: string;
  system: string;
  model?: string;
}

export interface Msg {
  role: "user" | "assistant";
  text: string;
  id: number;
}

export interface Approval {
  id: string;
  tool: string;
  kind: string;
  summary: string;
  reason: string;
}

/** Slash commands surfaced in the composer palette. */
export const SLASH_COMMANDS = [
  { cmd: "/do", args: "<goal>", desc: "Autonomous agent — plans & runs tools to reach a goal" },
  { cmd: "/write", args: "<path> <content>", desc: "Write a file (governed)" },
  { cmd: "/open", args: "<app or url>", desc: "Launch an app or open a URL" },
  { cmd: "/screenshot", args: "", desc: "Capture the screen" },
];

const PERSONAS = "overseer.personas";
const ACTIVE = "overseer.activePersona";
const IDLE = "overseer.idleSecs";

function read<T>(key: string, fallback: T): T {
  try {
    const v = localStorage.getItem(key);
    return v ? (JSON.parse(v) as T) : fallback;
  } catch {
    return fallback;
  }
}

export const loadPersonas = (): Persona[] => read<Persona[]>(PERSONAS, []);
export const savePersonas = (p: Persona[]) => localStorage.setItem(PERSONAS, JSON.stringify(p));

export const getActivePersona = (): string | null => localStorage.getItem(ACTIVE);
export const setActivePersona = (id: string | null) =>
  id ? localStorage.setItem(ACTIVE, id) : localStorage.removeItem(ACTIVE);

/** Inactivity timeout in seconds before the ambient clock takes over. 0 = off. */
export const getIdleSecs = (): number => Number(localStorage.getItem(IDLE) || "0");
export const setIdleSecs = (n: number) => localStorage.setItem(IDLE, String(n));

export const newId = () =>
  `p_${Date.now().toString(36)}_${Math.floor(performance.now()).toString(36)}`;

/** Parse the engine's response into clean assistant text:
 *  1. unwrap a StructuredResponse `{ messages: [...] }`
 *  2. collapse tool-trace bullets (`• write ~/x — {json…}`) into `✓ summary`
 *     so the user sees "✓ Wrote README.md", never raw tool JSON. */
export function parseAssistant(raw: string): string {
  const t = raw?.trim() ?? "";
  if (!t) return raw;

  // 1) Structured { messages: [...] }
  if (t.startsWith("{")) {
    try {
      const v = JSON.parse(t);
      if (Array.isArray(v.messages)) {
        const text = v.messages
          .map((m: { message?: string }) => m.message ?? "")
          .filter(Boolean)
          .join("\n\n");
        if (text) return text;
      }
    } catch {
      /* not JSON — fall through */
    }
  }

  // 2) Tool trace with embedded JSON → clean checklist
  if (/[•\-]\s.*\{.*"tool"/.test(t) || /\{.*"status"\s*:\s*"(completed|failed)"/.test(t)) {
    return t
      .split("\n")
      .filter((l) => l.trim() !== "---")
      .map((line) => {
        const m = line.match(/^\s*[•\-]\s*(.*?)\s*—\s*(\{.*)$/);
        if (!m) return line;
        const label = m[1].trim();
        const json = m[2];
        const sm = json.match(/"summary"\s*:\s*"((?:[^"\\]|\\.)*)"/);
        const failed = /"status"\s*:\s*"failed"/.test(json) || /"exit_code"\s*:\s*[1-9]/.test(json);
        let summary = sm
          ? sm[1].replace(/\\"/g, '"').replace(/\\n/g, " ").replace(/\\t/g, " ").trim()
          : label;
        summary = summary.replace(/\s*\(\.*$/, "").trim() || label; // drop truncation like "(..."
        return `${failed ? "✗" : "✓"} ${summary}`;
      })
      .join("\n")
      .trim();
  }

  return raw;
}
