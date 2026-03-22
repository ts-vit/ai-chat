import React from "react";
import ReactDOM from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import { UniProvider, TauriSettingsAdapter } from "@uni/ui";
import "@fontsource/inter";
import "@fontsource/inter/latin-500.css";
import "@fontsource/inter/latin-600.css";
import "@fontsource/inter/latin-700.css";
import "@fontsource/jetbrains-mono";
import "@uni/ui/src/styles/markdown.css";
import "@xterm/xterm/css/xterm.css";
import "./styles/app.css";
import "./styles/resize.css";
import "./i18n";
import App from "./App";

const settingsAdapter = new TauriSettingsAdapter(invoke);

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <UniProvider settingsAdapter={settingsAdapter}>
      <App />
    </UniProvider>
  </React.StrictMode>
);
