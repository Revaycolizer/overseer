import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";

// Resolve theme before first paint (saved choice, else system preference) so
// there's no flash of the wrong theme.
const savedTheme = localStorage.getItem("overseer.theme");
document.documentElement.dataset.theme =
  savedTheme || (window.matchMedia("(prefers-color-scheme: light)").matches ? "light" : "dark");

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
