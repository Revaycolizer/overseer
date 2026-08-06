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

/** Parse the engine's StructuredResponse JSON into plain assistant text. */
export function parseAssistant(raw: string): string {
  const t = raw?.trim() ?? "";
  if (!t.startsWith("{")) return raw;
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
    /* not JSON — show as-is */
  }
  return raw;
}
