import React from "react";
import ReactDOM from "react-dom/client";
import { MantineProvider, createTheme, MantineColorsTuple, CSSVariablesResolver } from "@mantine/core";
import { Notifications } from "@mantine/notifications";
import "@mantine/core/styles.css";
import "@mantine/notifications/styles.css";
import "@fontsource/inter";
import "@fontsource/inter/latin-500.css";
import "@fontsource/inter/latin-600.css";
import "@fontsource/inter/latin-700.css";
import "@fontsource/jetbrains-mono";
import "./styles/markdown.css";
import "./styles/app.css";
import "./styles/resize.css";
import "./i18n";
import App from "./App";

const brandOrange: MantineColorsTuple = [
    "#fef4ec", // 0 - lightest
    "#f9e2cf", // 1
    "#f3c9a7", // 2
    "#ecad7b", // 3
    "#e59557", // 4
    "#D4854A", // 5 - base
    "#bf7642", // 6
    "#a5653a", // 7
    "#8b5432", // 8
    "#72442a", // 9 - darkest
];

// CSS-переменные: текст, бордеры, dimmed — из brand-палитры
const resolver: CSSVariablesResolver = () => ({
    variables: {},
    light: {
        "--mantine-color-text": "#72442a",           // brand-9
        "--mantine-color-dimmed": "#a5653a",          // brand-7
        "--mantine-color-default-border": "#f3c9a7",  // brand-2
    },
    dark: {
        "--mantine-color-text": "#f3c9a7",            // brand-2
        "--mantine-color-dimmed": "#e59557",           // brand-4
        "--mantine-color-default-border": "#8b5432",   // brand-8
    },
});

// Тема приложения — тёмная по умолчанию
const theme = createTheme({
    primaryColor: "brand",
    colors: {
        brand: brandOrange,
    },
    fontFamily: "'Inter', 'Segoe UI', system-ui, sans-serif",
    fontFamilyMonospace: "'JetBrains Mono', 'Fira Code', monospace",
    components: {
        Divider: {
            defaultProps: {
                color: "brand.2",
            },
        },
        NavLink: {
            defaultProps: {
                color: "brand",
            },
        },
        Table: {
            styles: {
                table: { borderColor: "var(--mantine-color-default-border)" },
                tr: { borderColor: "var(--mantine-color-default-border)" },
                th: { color: "var(--mantine-color-text)", borderColor: "var(--mantine-color-default-border)" },
                td: { borderColor: "var(--mantine-color-default-border)" },
            },
        },
        Accordion: {
            styles: {
                item: { borderColor: "var(--mantine-color-default-border)" },
                control: {
                    backgroundColor: "var(--mantine-color-brand-0)",
                    color: "var(--mantine-color-brand-9)",
                },
                chevron: {
                    color: "var(--mantine-color-brand-5)",
                },
            },
        },
        SegmentedControl: {
            styles: {
                root: {
                    backgroundColor: "var(--mantine-color-brand-1)",
                    borderColor: "var(--mantine-color-default-border)",
                },
                label: {
                    color: "var(--mantine-color-brand-7)",
                },
                indicator: {
                    backgroundColor: "white",
                },
            },
        },
        TextInput: {
            styles: {
                input: { borderColor: "var(--mantine-color-default-border)" },
            },
        },
        PasswordInput: {
            styles: {
                input: { borderColor: "var(--mantine-color-default-border)" },
            },
        },
        Textarea: {
            styles: {
                input: { borderColor: "var(--mantine-color-default-border)" },
            },
        },
        Select: {
            styles: {
                input: { borderColor: "var(--mantine-color-default-border)" },
            },
        },
        Checkbox: {
            styles: {
                input: { borderColor: "var(--mantine-color-default-border)" },
            },
        },
        Paper: {
            styles: {
                root: { borderColor: "var(--mantine-color-default-border)" },
            },
        },
    },
});

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
        <MantineProvider theme={theme} cssVariablesResolver={resolver} defaultColorScheme="dark">
            <Notifications position="top-right" autoClose={4000} />
            <App />
        </MantineProvider>
    </React.StrictMode>
);