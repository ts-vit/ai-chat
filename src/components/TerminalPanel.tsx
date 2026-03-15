import { useEffect, useRef, useState, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { WebLinksAddon } from "@xterm/addon-web-links";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
    ActionIcon,
    Box,
    Group,
    Tooltip,
    useMantineColorScheme,
} from "@mantine/core";
import {
    IconAlertTriangle,
    IconCode,
    IconLayoutBottombar,
    IconLayoutSidebarRight,
    IconPlus,
    IconX,
} from "@tabler/icons-react";
import { notify } from "../utils/notify";
import "@xterm/xterm/css/xterm.css";

interface TerminalPanelProps {
    height?: number;
    width?: number;
    position: "bottom" | "right";
    onPositionChange: (pos: "bottom" | "right") => void;
    onClose: () => void;
    fontSize?: number;
    shell?: string;
}

interface TerminalTab {
    id: string;
    title: string;
    terminal: Terminal;
    fitAddon: FitAddon;
    containerDiv: HTMLDivElement;
    unlistenData: UnlistenFn | null;
    unlistenExit: UnlistenFn | null;
    proxyUrl: string | null;
}

const DARK_THEME = {
    background: "#1a1b1e",
    foreground: "#cccccc",
    cursor: "#cccccc",
    cursorAccent: "#1a1b1e",
    selectionBackground: "rgba(255, 255, 255, 0.2)",
    black: "#000000",
    red: "#ff6b6b",
    green: "#51cf66",
    yellow: "#fcc419",
    blue: "#339af0",
    magenta: "#cc5de8",
    cyan: "#22b8cf",
    white: "#ced4da",
    brightBlack: "#868e96",
    brightRed: "#ff8787",
    brightGreen: "#69db7c",
    brightYellow: "#ffd43b",
    brightBlue: "#5c7cfa",
    brightMagenta: "#da77f2",
    brightCyan: "#3bc9db",
    brightWhite: "#f8f9fa",
};

const LIGHT_THEME = {
    background: "#ffffff",
    foreground: "#333333",
    cursor: "#333333",
    cursorAccent: "#ffffff",
    selectionBackground: "rgba(0, 120, 215, 0.3)",
    black: "#000000",
    red: "#c91b00",
    green: "#00a600",
    yellow: "#c7c400",
    blue: "#0037da",
    magenta: "#c930c7",
    cyan: "#00a6b2",
    white: "#cccccc",
    brightBlack: "#666666",
    brightRed: "#e74856",
    brightGreen: "#16c60c",
    brightYellow: "#b5ba00",
    brightBlue: "#3b78ff",
    brightMagenta: "#b4009e",
    brightCyan: "#61d6d6",
    brightWhite: "#f2f2f2",
};

export function TerminalPanel({
    height,
    width,
    position,
    onPositionChange,
    onClose,
    fontSize = 13,
    shell,
}: TerminalPanelProps) {
    const { t } = useTranslation();
    const { colorScheme } = useMantineColorScheme();
    const [tabs, setTabs] = useState<TerminalTab[]>([]);
    const [activeTabId, setActiveTabId] = useState<string | null>(null);
    const [currentProxyUrl, setCurrentProxyUrl] = useState<string | null>(null);
    const tabCounterRef = useRef(0);
    const tabsContainerRef = useRef<HTMLDivElement>(null);
    const tabsRef = useRef<TerminalTab[]>([]);
    const mountedRef = useRef(false);

    // Keep tabsRef in sync
    tabsRef.current = tabs;

    const getTheme = useCallback(
        () => (colorScheme === "dark" ? DARK_THEME : LIGHT_THEME),
        [colorScheme]
    );

    const createTab = useCallback(
        async (title?: string, initialCommand?: string) => {
            if (!tabsContainerRef.current) return;

            tabCounterRef.current += 1;
            const tabTitle =
                title ?? `${t("terminal.title")} ${tabCounterRef.current}`;

            const term = new Terminal({
                fontFamily: "JetBrains Mono, monospace",
                fontSize,
                theme: getTheme(),
                cursorBlink: true,
                convertEol: true,
            });
            const fitAddon = new FitAddon();
            term.loadAddon(fitAddon);
            term.loadAddon(new WebLinksAddon());

            const containerDiv = document.createElement("div");
            containerDiv.style.width = "100%";
            containerDiv.style.height = "100%";
            containerDiv.style.display = "none";
            tabsContainerRef.current.appendChild(containerDiv);

            term.open(containerDiv);

            // Show this tab's container
            containerDiv.style.display = "block";
            // Hide all other tabs
            for (const tab of tabsRef.current) {
                tab.containerDiv.style.display = "none";
            }

            try {
                fitAddon.fit();
            } catch {
                // fit can throw if not attached
            }

            const sessionId: string = await invoke("terminal_create", {
                cols: term.cols,
                rows: term.rows,
                shell: shell || undefined,
            });

            const proxyUrl: string | null = await invoke("get_current_proxy_url");
            setCurrentProxyUrl(proxyUrl);

            const unlistenData = await listen<{
                sessionId: string;
                data: string;
            }>("pty-data", (event) => {
                if (event.payload.sessionId === sessionId) {
                    term.write(event.payload.data);
                }
            });

            const unlistenExit = await listen<{
                sessionId: string;
                code: number;
            }>("pty-exit", (event) => {
                if (event.payload.sessionId === sessionId) {
                    term.write("\r\n[Process exited]\r\n");
                }
            });

            term.onData((data) => {
                invoke("terminal_write", { sessionId, data }).catch(() => {});
            });

            if (initialCommand) {
                setTimeout(() => {
                    invoke("terminal_write", {
                        sessionId,
                        data: initialCommand,
                    }).catch(() => {});
                }, 300);
            }

            const newTab: TerminalTab = {
                id: sessionId,
                title: tabTitle,
                terminal: term,
                fitAddon,
                containerDiv,
                unlistenData,
                unlistenExit,
                proxyUrl,
            };

            setTabs((prev) => [...prev, newTab]);
            setActiveTabId(sessionId);

            // Fit after DOM settles
            setTimeout(() => {
                try {
                    fitAddon.fit();
                    invoke("terminal_resize", {
                        sessionId,
                        cols: term.cols,
                        rows: term.rows,
                    }).catch(() => {});
                } catch {
                    // ignore
                }
            }, 50);
        },
        [fontSize, shell, getTheme, t]
    );

    const switchTab = useCallback(
        (tabId: string) => {
            for (const tab of tabsRef.current) {
                tab.containerDiv.style.display =
                    tab.id === tabId ? "block" : "none";
            }
            setActiveTabId(tabId);
            setTimeout(() => {
                const tab = tabsRef.current.find((t) => t.id === tabId);
                if (tab) {
                    try {
                        tab.fitAddon.fit();
                        invoke("terminal_resize", {
                            sessionId: tab.id,
                            cols: tab.terminal.cols,
                            rows: tab.terminal.rows,
                        }).catch(() => {});
                    } catch {
                        // ignore
                    }
                }
            }, 50);
        },
        []
    );

    const closeTab = useCallback(
        (tabId: string) => {
            const currentTabs = tabsRef.current;
            const idx = currentTabs.findIndex((t) => t.id === tabId);
            if (idx === -1) return;

            const tab = currentTabs[idx];
            invoke("terminal_kill", { sessionId: tabId }).catch(() => {});
            tab.unlistenData?.();
            tab.unlistenExit?.();
            tab.terminal.dispose();
            tab.containerDiv.remove();

            const remaining = currentTabs.filter((t) => t.id !== tabId);
            setTabs(remaining);

            if (remaining.length === 0) {
                onClose();
            } else {
                setActiveTabId((current) => {
                    if (current === tabId) {
                        const newIdx = Math.min(idx, remaining.length - 1);
                        const newActive = remaining[newIdx].id;
                        // Show the new active tab
                        for (const t of remaining) {
                            t.containerDiv.style.display =
                                t.id === newActive ? "block" : "none";
                        }
                        setTimeout(() => {
                            const t = remaining.find(
                                (t) => t.id === newActive
                            );
                            if (t) {
                                try {
                                    t.fitAddon.fit();
                                } catch {}
                            }
                        }, 50);
                        return newActive;
                    }
                    return current;
                });
            }
        },
        [onClose]
    );

    const closeAllTabs = useCallback(() => {
        for (const tab of tabsRef.current) {
            invoke("terminal_kill", { sessionId: tab.id }).catch(() => {});
            tab.unlistenData?.();
            tab.unlistenExit?.();
            tab.terminal.dispose();
            tab.containerDiv.remove();
        }
        setTabs([]);
        setActiveTabId(null);
        onClose();
    }, [onClose]);

    // Listen for proxy settings changes
    useEffect(() => {
        const unlisten = listen("proxy-settings-changed", async () => {
            const newProxyUrl: string | null = await invoke("get_current_proxy_url");
            setCurrentProxyUrl(newProxyUrl);
            if (tabsRef.current.length > 0) {
                const hasOutdated = tabsRef.current.some(
                    (tab) => tab.proxyUrl !== newProxyUrl
                );
                if (hasOutdated) {
                    notify.warning(t("terminal.proxyChanged"), "proxy-changed");
                }
            }
        });
        return () => {
            unlisten.then((fn) => fn());
        };
    }, [t]);

    // Auto-create first tab on mount
    useEffect(() => {
        if (!mountedRef.current) {
            mountedRef.current = true;
            void createTab();
        }
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, []);

    // Cleanup on unmount
    useEffect(() => {
        return () => {
            for (const tab of tabsRef.current) {
                invoke("terminal_kill", { sessionId: tab.id }).catch(() => {});
                tab.unlistenData?.();
                tab.unlistenExit?.();
                tab.terminal.dispose();
            }
        };
    }, []);

    // Refit on size / position change
    useEffect(() => {
        const timer = setTimeout(() => {
            const tab = tabsRef.current.find((t) => t.id === activeTabId);
            if (tab) {
                try {
                    tab.fitAddon.fit();
                    invoke("terminal_resize", {
                        sessionId: tab.id,
                        cols: tab.terminal.cols,
                        rows: tab.terminal.rows,
                    }).catch(() => {});
                } catch {}
            }
        }, 50);
        return () => clearTimeout(timer);
    }, [height, width, position, activeTabId]);

    // Theme change
    useEffect(() => {
        const theme = getTheme();
        for (const tab of tabsRef.current) {
            tab.terminal.options.theme = theme;
        }
    }, [colorScheme, getTheme]);

    // Font size change
    useEffect(() => {
        for (const tab of tabsRef.current) {
            tab.terminal.options.fontSize = fontSize;
        }
        // Refit active tab
        const timer = setTimeout(() => {
            const tab = tabsRef.current.find((t) => t.id === activeTabId);
            if (tab) {
                try {
                    tab.fitAddon.fit();
                    invoke("terminal_resize", {
                        sessionId: tab.id,
                        cols: tab.terminal.cols,
                        rows: tab.terminal.rows,
                    }).catch(() => {});
                } catch {}
            }
        }, 50);
        return () => clearTimeout(timer);
    }, [fontSize, activeTabId]);

    return (
        <Box
            style={{
                height: position === "bottom" ? height : "100%",
                width: position === "right" ? width : "100%",
                display: "flex",
                flexDirection: "column",
                overflow: "hidden",
                borderTop:
                    position === "bottom"
                        ? "1px solid var(--mantine-color-default-border)"
                        : "none",
                borderLeft:
                    position === "right"
                        ? "1px solid var(--mantine-color-default-border)"
                        : "none",
                flexShrink: 0,
            }}
        >
            {/* Tab bar */}
            <Group
                gap={2}
                wrap="nowrap"
                style={{
                    borderBottom:
                        "1px solid var(--mantine-color-default-border)",
                    padding: "2px 4px",
                    flexShrink: 0,
                    height: 30,
                    overflowX: "auto",
                    overflowY: "hidden",
                }}
            >
                {tabs.map((tab) => (
                    <Group
                        key={tab.id}
                        gap={2}
                        wrap="nowrap"
                        onClick={() => switchTab(tab.id)}
                        style={{
                            cursor: "pointer",
                            padding: "2px 8px",
                            borderRadius: 4,
                            fontSize: 12,
                            whiteSpace: "nowrap",
                            background:
                                tab.id === activeTabId
                                    ? "var(--mantine-color-brand-light)"
                                    : "transparent",
                            userSelect: "none",
                        }}
                    >
                        <span>{tab.title}</span>
                        {tab.proxyUrl !== currentProxyUrl && (
                            <Tooltip label={t("terminal.proxyChanged")} position="bottom">
                                <IconAlertTriangle size={12} color="var(--mantine-color-yellow-6)" />
                            </Tooltip>
                        )}
                        <ActionIcon
                            size={16}
                            variant="transparent"
                            color="dimmed"
                            onClick={(e) => {
                                e.stopPropagation();
                                closeTab(tab.id);
                            }}
                        >
                            <IconX size={10} />
                        </ActionIcon>
                    </Group>
                ))}

                <Tooltip label={t("terminal.newTab")} position="bottom">
                    <ActionIcon
                        variant="subtle"
                        size="sm"
                        onClick={() => void createTab()}
                    >
                        <IconPlus size={14} />
                    </ActionIcon>
                </Tooltip>

                <Tooltip label={t("terminal.claudeCode")} position="bottom">
                    <ActionIcon
                        variant="subtle"
                        size="sm"
                        onClick={() =>
                            void createTab("Claude Code", "claude\r")
                        }
                    >
                        <IconCode size={14} />
                    </ActionIcon>
                </Tooltip>

                <Box style={{ flex: 1 }} />

                <Tooltip
                    label={t(
                        position === "bottom"
                            ? "terminal.right"
                            : "terminal.bottom"
                    )}
                    position="bottom"
                >
                    <ActionIcon
                        variant="subtle"
                        size="sm"
                        onClick={() =>
                            onPositionChange(
                                position === "bottom" ? "right" : "bottom"
                            )
                        }
                    >
                        {position === "bottom" ? (
                            <IconLayoutSidebarRight size={14} />
                        ) : (
                            <IconLayoutBottombar size={14} />
                        )}
                    </ActionIcon>
                </Tooltip>

                <Tooltip label={t("terminal.closeAll")} position="bottom">
                    <ActionIcon
                        variant="subtle"
                        size="sm"
                        color="red"
                        onClick={closeAllTabs}
                    >
                        <IconX size={14} />
                    </ActionIcon>
                </Tooltip>
            </Group>

            {/* Terminal content area */}
            <Box
                ref={tabsContainerRef}
                style={{ flex: 1, overflow: "hidden" }}
            />
        </Box>
    );
}
