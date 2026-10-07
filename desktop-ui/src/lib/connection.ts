import { invoke, isTauri } from "@tauri-apps/api/core";
export interface Connection { url: string; token: string }
const savedUrl = localStorage.getItem("overseer.wsUrl");
export const managed = isTauri() && (!savedUrl || savedUrl === "ws://127.0.0.1:8080/api/ws/chat");
export let initialConnection: Connection = { url: savedUrl || "ws://127.0.0.1:8080/api/ws/chat", token: localStorage.getItem("overseer.token") || "" };
export let startupError = "";
export async function refreshConnection(): Promise<Connection> {
  const selected = localStorage.getItem("overseer.wsUrl");
  if (isTauri() && (!selected || selected === "ws://127.0.0.1:8080/api/ws/chat")) {
    initialConnection = await invoke<Connection>("desktop_connection");
  } else {
    initialConnection = { url: selected || "ws://127.0.0.1:8080/api/ws/chat", token: localStorage.getItem("overseer.token") || "" };
  }
  return initialConnection;
}
export async function initializeConnection() {
  try { await refreshConnection(); } catch (error) { startupError = String(error); }
}
