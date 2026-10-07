import { initializeConnection } from "./lib/connection";
import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import App from "./App";
import Widget from "./screens/Widget";

// Warm ivory glass and graphite accents are shared by both windows.
document.documentElement.dataset.theme = "light";

// The same bundle serves two windows: the full app ("main") and the floating
// widget ("widget"). Pick by the Tauri window label; default to the app.
let isWidget = false;
try {
  isWidget = getCurrentWindow().label === "widget";
} catch {
  isWidget = false;
}

initializeConnection().then(() => ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>{isWidget ? <Widget /> : <App />}</React.StrictMode>,
));
