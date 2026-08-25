import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import App from "./App";
import Widget from "./screens/Widget";

// Overseer ships a single, light "paper" theme — no dark mode.
document.documentElement.dataset.theme = "light";

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
