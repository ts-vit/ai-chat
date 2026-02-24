import React from "react";
import ReactDOM from "react-dom/client";
import { MantineProvider, createTheme } from "@mantine/core";
import { Notifications } from "@mantine/notifications";
import "@mantine/core/styles.css";
import "@mantine/notifications/styles.css";
import "@fontsource/inter";
import "@fontsource/inter/latin-500.css";
import "@fontsource/inter/latin-600.css";
import "@fontsource/inter/latin-700.css";
import "@fontsource/jetbrains-mono";
import "./styles/markdown.css";
import "./styles/resize.css";
import App from "./App";

// Тема приложения — тёмная по умолчанию
const theme = createTheme({
    primaryColor: "blue",
    fontFamily: "'Inter', 'Segoe UI', system-ui, sans-serif",
    fontFamilyMonospace: "'JetBrains Mono', 'Fira Code', monospace",
});

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
        <MantineProvider theme={theme} defaultColorScheme="dark">
            <Notifications position="top-right" autoClose={4000} />
            <App />
        </MantineProvider>
    </React.StrictMode>
);