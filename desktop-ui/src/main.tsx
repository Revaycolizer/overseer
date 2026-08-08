import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import App from "./App";
import Widget from "./screens/Widget";

// Resolve theme before first paint (saved choice, else system preference) so
// there's no flash of the wrong theme.
const savedTheme = localStorage.getItem("overseer.theme");
document.documentElement.dataset.theme =
  savedTheme || (window.matchMedia("(prefers-color-scheme: light)").matches ? "light" : "dark");

// The same bundle serves two windows: the full app ("main") and the floating
// widget ("widget"). Pick by the Tauri window label; default to the app.
let isWidget = false;
try {
  isWidget = getCurrentWindow().label === "widget";
} catch {
  isWidget = false;
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>{isWidget ? <Widget /> : <App />}</React.StrictMode>,
);
